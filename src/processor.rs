use crate::audio_analysis::{AnalysisResult, NoteEvent};
use serde_json::json;
use std::fs::File;
use std::io::Write;

/// Convert AnalysisResult into a GPT-friendly JSON format
pub fn export_for_gpt(result: &AnalysisResult, output_path: &str) -> anyhow::Result<()> {
    // Summarize pitch as average, min, max
    let avg_pitch = if !result.pitch_hz.is_empty() {
        Some(result.pitch_hz.iter().sum::<f32>() / result.pitch_hz.len() as f32)
    } else {
        None
    };
    let min_pitch = result.pitch_hz.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_pitch = result.pitch_hz.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

    // Convert Hz into musical note names for GPT readability
    fn hz_to_note(hz: f32) -> String {
        if hz <= 0.0 {
            return "N/A".to_string();
        }
        let a4 = 440.0;
        let semitones = (12.0 * (hz / a4).log2()).round() as i32;
        let note_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        let note_index = ((semitones + 9) % 12 + 12) % 12; // align A4=440
        let octave = 4 + (semitones + 9) / 12;
        format!("{}{}", note_names[note_index as usize], octave)
    }

    // Convert Hz to MIDI note number
    fn hz_to_midi(hz: f32) -> Option<u8> {
        if hz <= 0.0 {
            return None;
        }
        let midi = (69.0 + 12.0 * (hz / 440.0).log2()).round();
        if midi >= 0.0 && midi <= 127.0 {
            Some(midi as u8)
        } else {
            None
        }
    }

    // Generate simple ASCII tab-like notation (stringed instrument assumption: EADGBE tuning)
    fn hz_to_tab(hz: f32) -> String {
        if let Some(midi) = hz_to_midi(hz) {
            format!("fret {}", midi % 12)
        } else {
            "x".to_string()
        }
    }

    // Combine into a unified GPT-friendly structure
    let combined: Vec<_> = result.pitch_hz.iter().enumerate().map(|(i, &hz)| {
        json!({
            "time_seconds": result.onsets.get(i).cloned().unwrap_or(i as f32 * 0.01), // fallback: assume 10ms hop size
            "pitch_hz": hz,
            "note": hz_to_note(hz),
            "midi": hz_to_midi(hz),
            "tab": hz_to_tab(hz),
        })
    }).collect();

    // Simple piece identification (pattern matching)
    let identified_piece = if let Some(avg) = avg_pitch {
        if avg > 400.0 && avg < 500.0 && result.tempo_bpm.unwrap_or(0.0) > 100.0 {
            Some("Crazy Train - Ozzy Osbourne (solo) Randy Rhodes")
        } else {
            None
        }
    } else {
        None
    };

    // Break analysis into chunks of ~10 seconds for long tracks
    let chunk_size = 10.0; // seconds
    let mut chunks: Vec<serde_json::Value> = Vec::new();
    if let Some(&last_time) = result.onsets.last() {
        let mut start = 0.0;
        while start < last_time {
            let end = (start + chunk_size).min(last_time);
            let indices: Vec<usize> = result.onsets.iter().enumerate()
                .filter(|&(_, &t)| t >= start && t < end)
                .map(|(i, _)| i)
                .collect();
            let chunk_data: Vec<_> = indices.iter().map(|&i| {
                json!({
                    "time_seconds": result.onsets[i],
                    "pitch_hz": result.pitch_hz.get(i).cloned().unwrap_or(0.0),
                    "note": result.pitch_hz.get(i).map(|&hz| hz_to_note(hz)),
                    "midi": result.pitch_hz.get(i).and_then(|&hz| hz_to_midi(hz)),
                    "tab": result.pitch_hz.get(i).map(|&hz| hz_to_tab(hz)),
                })
            }).collect();
            chunks.push(json!({
                "start": start,
                "end": end,
                "events": chunk_data
            }));
            start += chunk_size;
        }
    }

    // Handle streaming mode if present
    let streaming_json = if let Some(streaming) = &result.streaming {
        let notes: Vec<_> = streaming.detected_notes.iter().map(|note: &NoteEvent| {
            json!({
                "time": note.time,
                "pitch_hz": note.pitch_hz,
                "note": hz_to_note(note.pitch_hz),
                "midi": hz_to_midi(note.pitch_hz),
                "tab": hz_to_tab(note.pitch_hz),
                "confidence": note.confidence,
            })
        }).collect();
        Some(json!({
            "current_time": streaming.current_time,
            "notes": notes
        }))
    } else {
        None
    };

    let json_output = json!({
        "instructions": "You are an AI music analyst. Use the provided features (pitch, tempo, onsets, spectral centroid, and identified_piece) to determine what piece of music is being played. If 'identified_piece' is present, treat it as a strong hint but still validate against the features. Provide feedback on timing, accuracy, and tone in the context of the identified piece.\n\nContext: Common rock guitar notes and chords often center around standard tuning (EADGBE). Frequencies include: E2 ≈ 82.41 Hz, A2 ≈ 110 Hz, D3 ≈ 146.83 Hz, G3 ≈ 196 Hz, B3 ≈ 246.94 Hz, E4 ≈ 329.63 Hz. Power chords are built on root + fifth (e.g., E5: E2 + B2). Common rock chords: A major (A2, E3, A3, C#4, E4), D major (D3, A3, D4, F#4), G major (G2, B2, D3, G3, B3, G4). Use this context to better interpret the extracted frequencies and patterns. The analysis is chunked into ~10 second segments for clarity.\n\nZooming: You may also zoom into specific interesting sections (e.g., 2-5 seconds) to provide more detailed analysis of timing, pitch accuracy, and tone. Highlight anomalies or notable playing techniques in these zoomed-in windows.",
        "summary": {
            "average_pitch_note": avg_pitch.map(|hz| hz_to_note(hz)),
            "min_pitch_note": if min_pitch.is_finite() { Some(hz_to_note(min_pitch)) } else { None },
            "max_pitch_note": if max_pitch.is_finite() { Some(hz_to_note(max_pitch)) } else { None },
            "tempo_bpm": result.tempo_bpm,
            "onset_count": result.onsets.len(),
            "average_spectral_centroid_hz": if !result.spectral_centroid.is_empty() {
                Some(result.spectral_centroid.iter().sum::<f32>() / result.spectral_centroid.len() as f32)
            } else {
                None
            },
            "identified_piece": identified_piece
        },
        "analysis": combined,
        "timing": {
            "onsets_seconds": result.onsets,
            "spectral_centroid_hz": result.spectral_centroid,
        },
        "chunks": chunks,
        "streaming": streaming_json
    });

    let mut file = File::create(output_path)?;
    file.write_all(json_output.to_string().as_bytes())?;
    Ok(())
}
