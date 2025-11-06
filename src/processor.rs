use crate::audio_analysis::{AnalysisResult, NoteEvent};
use crate::comparison::{
    ComparisonMetrics, compare_recordings, extract_note_sequence, extract_rhythm_pattern,
    hz_to_note_name,
};
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
    let min_pitch = result
        .pitch_hz
        .iter()
        .cloned()
        .fold(f32::INFINITY, f32::min);
    let max_pitch = result
        .pitch_hz
        .iter()
        .cloned()
        .fold(f32::NEG_INFINITY, f32::max);

    // Convert Hz into musical note names for GPT readability
    fn hz_to_note(hz: f32) -> String {
        if hz <= 0.0 {
            return "N/A".to_string();
        }
        let a4 = 440.0;
        let semitones = (12.0 * (hz / a4).log2()).round() as i32;
        let note_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
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
        if (0.0..=127.0).contains(&midi) {
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
    let combined: Vec<_> = result
        .pitch_hz
        .iter()
        .enumerate()
        .map(|(i, &hz)| {
            json!({
                "time_seconds": result.onsets.get(i).cloned().unwrap_or(i as f32 * 0.01), // fallback: assume 10ms hop size
                "pitch_hz": hz,
                "note": hz_to_note(hz),
                "midi": hz_to_midi(hz),
                "tab": hz_to_tab(hz),
            })
        })
        .collect();

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
            let indices: Vec<usize> = result
                .onsets
                .iter()
                .enumerate()
                .filter(|&(_, &t)| t >= start && t < end)
                .map(|(i, _)| i)
                .collect();
            let chunk_data: Vec<_> = indices
                .iter()
                .map(|&i| {
                    json!({
                        "time_seconds": result.onsets[i],
                        "pitch_hz": result.pitch_hz.get(i).cloned().unwrap_or(0.0),
                        "note": result.pitch_hz.get(i).map(|&hz| hz_to_note(hz)),
                        "midi": result.pitch_hz.get(i).and_then(|&hz| hz_to_midi(hz)),
                        "tab": result.pitch_hz.get(i).map(|&hz| hz_to_tab(hz)),
                    })
                })
                .collect();
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
        let notes: Vec<_> = streaming
            .detected_notes
            .iter()
            .map(|note: &NoteEvent| {
                json!({
                    "time": note.time,
                    "pitch_hz": note.pitch_hz,
                    "note": hz_to_note(note.pitch_hz),
                    "midi": hz_to_midi(note.pitch_hz),
                    "tab": hz_to_tab(note.pitch_hz),
                    "confidence": note.confidence,
                })
            })
            .collect();
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
            "average_pitch_note": avg_pitch.map(hz_to_note),
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

/// Export optimized analysis for GPT with reduced context window usage
/// This version focuses on summarized data and musical patterns rather than raw values
pub fn export_optimized_for_gpt(
    result: &AnalysisResult,
    output_path: &str,
    reference: Option<&AnalysisResult>,
) -> anyhow::Result<()> {
    // Extract high-level musical features
    let note_sequence = extract_note_sequence(result);
    let rhythm_pattern = extract_rhythm_pattern(result);

    // Calculate statistics instead of dumping all values
    let pitch_stats = if !result.pitch_hz.is_empty() {
        let avg = result.pitch_hz.iter().sum::<f32>() / result.pitch_hz.len() as f32;
        let min = result
            .pitch_hz
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min);
        let max = result
            .pitch_hz
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let variance = result
            .pitch_hz
            .iter()
            .map(|&x| (x - avg).powi(2))
            .sum::<f32>()
            / result.pitch_hz.len() as f32;
        let std_dev = variance.sqrt();

        json!({
            "average_hz": avg,
            "average_note": hz_to_note_name(avg),
            "min_hz": min,
            "min_note": hz_to_note_name(min),
            "max_hz": max,
            "max_note": hz_to_note_name(max),
            "pitch_range_semitones": ((max / min).log2() * 12.0).round(),
            "pitch_stability": if avg > 0.0 { 1.0 - (std_dev / avg).min(1.0) } else { 0.0 },
        })
    } else {
        json!({})
    };

    // Calculate unique notes
    let unique_notes: Vec<String> = {
        let mut unique: Vec<String> = note_sequence.iter().map(|n| n.note_name.clone()).collect();
        unique.sort();
        unique.dedup();
        unique
    };

    // Simplified note sequence (top-level patterns only)
    let notes_summary = json!({
        "total_notes": note_sequence.len(),
        "unique_notes": unique_notes,
        "note_sequence": note_sequence.iter().map(|n| {
            json!({
                "note": n.note_name,
                "time": format!("{:.2}", n.start_time),
                "duration": format!("{:.3}", n.duration),
            })
        }).collect::<Vec<_>>(),
    });

    // Rhythm analysis
    let rhythm_summary = json!({
        "total_onsets": rhythm_pattern.onset_times.len(),
        "average_note_interval_ms": (rhythm_pattern.avg_interval * 1000.0).round(),
        "tempo_stability": format!("{:.2}", rhythm_pattern.tempo_stability),
        "tempo_bpm": result.tempo_bpm,
    });

    // Comparison metrics if reference provided
    let comparison = if let Some(ref_result) = reference {
        let metrics = compare_recordings(ref_result, result);
        Some(json!({
            "overall_similarity": format!("{:.1}%", metrics.overall_similarity * 100.0),
            "scores": {
                "note_accuracy": format!("{:.1}%", metrics.note_accuracy * 100.0),
                "pitch_accuracy": format!("{:.1}%", metrics.pitch_accuracy * 100.0),
                "timing_accuracy": format!("{:.1}%", metrics.timing_accuracy * 100.0),
                "rhythm_accuracy": format!("{:.1}%", metrics.rhythm_accuracy * 100.0),
            },
            "errors": {
                "missed_notes": metrics.missed_notes,
                "extra_notes": metrics.extra_notes,
                "pitch_errors": metrics.pitch_errors.iter().take(10).map(|e| {
                    json!({
                        "time": format!("{:.2}s", e.time),
                        "expected": e.expected_note,
                        "played": e.played_note,
                        "cents_off": format!("{:.1}", e.cent_difference),
                    })
                }).collect::<Vec<_>>(),
                "timing_errors": metrics.timing_errors.iter().take(10).map(|e| {
                    json!({
                        "note": e.note,
                        "expected_time": format!("{:.2}s", e.expected_time),
                        "played_time": format!("{:.2}s", e.played_time),
                        "ms_late": format!("{:.1}", e.ms_difference),
                    })
                }).collect::<Vec<_>>(),
            },
            "summary": generate_error_summary(&metrics),
        }))
    } else {
        None
    };

    let json_output = json!({
        "format_version": "2.0-optimized",
        "instructions": generate_instructions(comparison.is_some()),
        "pitch_statistics": pitch_stats,
        "notes": notes_summary,
        "rhythm": rhythm_summary,
        "comparison": comparison,
        "context": {
            "sample_rate": "analyzed",
            "window_size": 1024,
            "hop_size": 512,
        }
    });

    let mut file = File::create(output_path)?;
    file.write_all(serde_json::to_string_pretty(&json_output)?.as_bytes())?;
    Ok(())
}

/// Generate context-appropriate instructions for the AI
fn generate_instructions(has_comparison: bool) -> String {
    if has_comparison {
        "You are analyzing a student's guitar performance compared to a reference recording. \
        The 'comparison' section provides detailed metrics about accuracy. Focus on:\n\
        1. Overall similarity score and what it means\n\
        2. Specific errors in pitch, timing, and rhythm\n\
        3. Missed or extra notes\n\
        4. Constructive feedback on how to improve\n\
        5. Positive reinforcement for what was done well\n\n\
        Use the note sequences and rhythm patterns to understand the musical context. \
        Be specific about which notes or sections need work."
            .to_string()
    } else {
        "You are analyzing a guitar recording. Use the provided statistics and patterns to:\n\
        1. Identify the musical content (notes, rhythm, tempo)\n\
        2. Assess the overall quality and technique\n\
        3. Provide constructive feedback\n\
        4. Suggest areas for improvement\n\n\
        Consider pitch stability, rhythm consistency, and note accuracy."
            .to_string()
    }
}

/// Generate a human-readable summary of errors
fn generate_error_summary(metrics: &ComparisonMetrics) -> String {
    let mut summary = Vec::new();

    if metrics.overall_similarity >= 0.9 {
        summary.push("Excellent performance! Very close to the reference.".to_string());
    } else if metrics.overall_similarity >= 0.75 {
        summary.push("Good performance with minor errors.".to_string());
    } else if metrics.overall_similarity >= 0.5 {
        summary.push("Fair performance. Several areas need improvement.".to_string());
    } else {
        summary.push("Needs significant practice. Many errors detected.".to_string());
    }

    if metrics.note_accuracy < 0.7 {
        summary.push(format!(
            "Note accuracy is low ({:.0}%). Focus on playing the correct notes.",
            metrics.note_accuracy * 100.0
        ));
    }

    if metrics.pitch_accuracy < 0.7 {
        summary.push(format!(
            "Pitch accuracy needs work ({:.0}%). Notes are out of tune.",
            metrics.pitch_accuracy * 100.0
        ));
    }

    if metrics.timing_accuracy < 0.7 {
        summary.push(format!(
            "Timing is off ({:.0}%). Practice with a metronome.",
            metrics.timing_accuracy * 100.0
        ));
    }

    if metrics.rhythm_accuracy < 0.7 {
        summary.push(format!(
            "Rhythm accuracy needs improvement ({:.0}%).",
            metrics.rhythm_accuracy * 100.0
        ));
    }

    if !metrics.missed_notes.is_empty() {
        summary.push(format!(
            "Missed {} note(s). Make sure to play all notes in the piece.",
            metrics.missed_notes.len()
        ));
    }

    if !metrics.extra_notes.is_empty() {
        summary.push(format!(
            "Played {} extra note(s) not in the reference.",
            metrics.extra_notes.len()
        ));
    }

    summary.join(" ")
}
