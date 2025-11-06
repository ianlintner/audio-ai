use crate::audio_analysis::AnalysisResult;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct NoteSequence {
    pub note_name: String,
    pub midi_note: u8,
    pub start_time: f32,
    pub duration: f32,
    pub avg_pitch_hz: f32,
}

#[derive(Serialize, Debug, Clone)]
pub struct RhythmPattern {
    pub onset_times: Vec<f32>,
    pub inter_onset_intervals: Vec<f32>,
    pub avg_interval: f32,
    pub tempo_stability: f32, // 0.0 = unstable, 1.0 = very stable
}

#[derive(Serialize, Debug)]
pub struct ComparisonMetrics {
    pub pitch_accuracy: f32,     // 0.0 to 1.0
    pub rhythm_accuracy: f32,    // 0.0 to 1.0
    pub timing_accuracy: f32,    // 0.0 to 1.0
    pub note_accuracy: f32,      // 0.0 to 1.0 (correct notes played)
    pub overall_similarity: f32, // 0.0 to 1.0
    pub missed_notes: Vec<String>,
    pub extra_notes: Vec<String>,
    pub pitch_errors: Vec<PitchError>,
    pub timing_errors: Vec<TimingError>,
}

#[derive(Serialize, Debug, Clone)]
pub struct PitchError {
    pub time: f32,
    pub expected_note: String,
    pub played_note: String,
    pub cent_difference: f32, // cents off (100 cents = 1 semitone)
}

#[derive(Serialize, Debug, Clone)]
pub struct TimingError {
    pub note: String,
    pub expected_time: f32,
    pub played_time: f32,
    pub ms_difference: f32,
}

/// Convert Hz to MIDI note number
pub fn hz_to_midi(hz: f32) -> Option<u8> {
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

/// Convert MIDI note to note name
pub fn midi_to_note_name(midi: u8) -> String {
    let note_names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (midi / 12) as i32 - 1;
    let note_index = (midi % 12) as usize;
    format!("{}{}", note_names[note_index], octave)
}

/// Convert Hz to note name
pub fn hz_to_note_name(hz: f32) -> String {
    if let Some(midi) = hz_to_midi(hz) {
        midi_to_note_name(midi)
    } else {
        "N/A".to_string()
    }
}

/// Calculate cent difference between two frequencies
pub fn pitch_difference_cents(hz1: f32, hz2: f32) -> f32 {
    if hz1 <= 0.0 || hz2 <= 0.0 {
        return 0.0;
    }
    1200.0 * (hz2 / hz1).log2()
}

/// Extract note sequences from pitch data with onset information
pub fn extract_note_sequence(analysis: &AnalysisResult) -> Vec<NoteSequence> {
    let mut sequences = Vec::new();

    if analysis.pitch_hz.is_empty() || analysis.onsets.is_empty() {
        return sequences;
    }

    // Group consecutive similar pitches into notes
    let midi_threshold = 1; // Allow 1 semitone variation within same note
    let time_threshold = 0.1; // 100ms minimum note duration

    let mut current_midi: Option<u8> = None;
    let mut current_start = 0.0;
    let mut current_pitches = Vec::new();

    for (i, &pitch_hz) in analysis.pitch_hz.iter().enumerate() {
        let midi = hz_to_midi(pitch_hz);
        let time = analysis.onsets.get(i).copied().unwrap_or(i as f32 * 0.01);

        if let Some(midi_note) = midi {
            match current_midi {
                None => {
                    // Start new note
                    current_midi = Some(midi_note);
                    current_start = time;
                    current_pitches.push(pitch_hz);
                }
                Some(prev_midi) => {
                    if (midi_note as i32 - prev_midi as i32).abs() <= midi_threshold {
                        // Continue current note
                        current_pitches.push(pitch_hz);
                    } else {
                        // Save previous note and start new one
                        if time - current_start >= time_threshold {
                            let avg_pitch =
                                current_pitches.iter().sum::<f32>() / current_pitches.len() as f32;
                            sequences.push(NoteSequence {
                                note_name: midi_to_note_name(prev_midi),
                                midi_note: prev_midi,
                                start_time: current_start,
                                duration: time - current_start,
                                avg_pitch_hz: avg_pitch,
                            });
                        }
                        current_midi = Some(midi_note);
                        current_start = time;
                        current_pitches.clear();
                        current_pitches.push(pitch_hz);
                    }
                }
            }
        }
    }

    // Add final note
    if let Some(midi_note) = current_midi
        && let Some(&last_time) = analysis.onsets.last()
        && last_time - current_start >= time_threshold
        && !current_pitches.is_empty()
    {
        let avg_pitch = current_pitches.iter().sum::<f32>() / current_pitches.len() as f32;
        sequences.push(NoteSequence {
            note_name: midi_to_note_name(midi_note),
            midi_note,
            start_time: current_start,
            duration: last_time - current_start,
            avg_pitch_hz: avg_pitch,
        });
    }

    sequences
}

/// Extract rhythm pattern from onset data
pub fn extract_rhythm_pattern(analysis: &AnalysisResult) -> RhythmPattern {
    let onset_times = analysis.onsets.clone();

    let inter_onset_intervals: Vec<f32> = onset_times.windows(2).map(|w| w[1] - w[0]).collect();

    let avg_interval = if !inter_onset_intervals.is_empty() {
        inter_onset_intervals.iter().sum::<f32>() / inter_onset_intervals.len() as f32
    } else {
        0.0
    };

    // Calculate tempo stability as inverse of coefficient of variation
    let tempo_stability = if avg_interval > 0.0 && inter_onset_intervals.len() > 1 {
        let variance: f32 = inter_onset_intervals
            .iter()
            .map(|&x| (x - avg_interval).powi(2))
            .sum::<f32>()
            / inter_onset_intervals.len() as f32;
        let std_dev = variance.sqrt();
        let cv = std_dev / avg_interval; // coefficient of variation
        (1.0 / (1.0 + cv)).min(1.0) // normalize to 0-1
    } else {
        0.0
    };

    RhythmPattern {
        onset_times,
        inter_onset_intervals,
        avg_interval,
        tempo_stability,
    }
}

/// Compare two recordings and generate detailed metrics
pub fn compare_recordings(
    reference: &AnalysisResult,
    player: &AnalysisResult,
) -> ComparisonMetrics {
    let ref_notes = extract_note_sequence(reference);
    let player_notes = extract_note_sequence(player);

    let ref_rhythm = extract_rhythm_pattern(reference);
    let player_rhythm = extract_rhythm_pattern(player);

    // Calculate note accuracy using simplified Dynamic Time Warping approach
    let (note_accuracy, pitch_errors) = compare_note_sequences(&ref_notes, &player_notes);

    // Calculate timing accuracy
    let (timing_accuracy, timing_errors) = compare_timing(&ref_notes, &player_notes);

    // Calculate rhythm accuracy based on onset patterns
    let rhythm_accuracy = compare_rhythm(&ref_rhythm, &player_rhythm);

    // Calculate pitch accuracy (average cent difference)
    let pitch_accuracy = calculate_pitch_accuracy(&pitch_errors);

    // Find missed and extra notes
    let (missed_notes, extra_notes) = find_note_differences(&ref_notes, &player_notes);

    // Overall similarity is weighted average
    let overall_similarity = 0.3 * note_accuracy
        + 0.25 * pitch_accuracy
        + 0.25 * timing_accuracy
        + 0.2 * rhythm_accuracy;

    ComparisonMetrics {
        pitch_accuracy,
        rhythm_accuracy,
        timing_accuracy,
        note_accuracy,
        overall_similarity,
        missed_notes,
        extra_notes,
        pitch_errors,
        timing_errors,
    }
}

fn compare_note_sequences(
    reference: &[NoteSequence],
    player: &[NoteSequence],
) -> (f32, Vec<PitchError>) {
    if reference.is_empty() || player.is_empty() {
        return (0.0, Vec::new());
    }

    let mut pitch_errors = Vec::new();
    let mut correct_count = 0;
    let max_time_diff = 0.5; // 500ms window for note matching

    for ref_note in reference {
        // Find closest player note in time
        let closest_player = player
            .iter()
            .min_by_key(|p| ((p.start_time - ref_note.start_time).abs() * 1000.0) as i32);

        if let Some(player_note) = closest_player
            && (player_note.start_time - ref_note.start_time).abs() <= max_time_diff
        {
            let cent_diff = pitch_difference_cents(ref_note.avg_pitch_hz, player_note.avg_pitch_hz);

            // Consider correct if within 50 cents (half semitone)
            if cent_diff.abs() <= 50.0 {
                correct_count += 1;
            } else {
                pitch_errors.push(PitchError {
                    time: ref_note.start_time,
                    expected_note: ref_note.note_name.clone(),
                    played_note: player_note.note_name.clone(),
                    cent_difference: cent_diff,
                });
            }
        }
    }

    let accuracy = correct_count as f32 / reference.len() as f32;
    (accuracy, pitch_errors)
}

fn compare_timing(reference: &[NoteSequence], player: &[NoteSequence]) -> (f32, Vec<TimingError>) {
    if reference.is_empty() || player.is_empty() {
        return (0.0, Vec::new());
    }

    let mut timing_errors = Vec::new();
    let mut total_timing_error = 0.0;
    let max_time_diff = 0.5;

    for ref_note in reference {
        let closest_player = player
            .iter()
            .min_by_key(|p| ((p.start_time - ref_note.start_time).abs() * 1000.0) as i32);

        if let Some(player_note) = closest_player {
            let time_diff = (player_note.start_time - ref_note.start_time).abs();
            if time_diff <= max_time_diff {
                total_timing_error += time_diff;

                if time_diff > 0.05 {
                    // Report if more than 50ms off
                    timing_errors.push(TimingError {
                        note: ref_note.note_name.clone(),
                        expected_time: ref_note.start_time,
                        played_time: player_note.start_time,
                        ms_difference: time_diff * 1000.0,
                    });
                }
            }
        }
    }

    // Convert to 0-1 scale (0ms = 1.0, 500ms = 0.0)
    let avg_error = total_timing_error / reference.len() as f32;
    let accuracy = (1.0 - (avg_error / max_time_diff)).max(0.0);

    (accuracy, timing_errors)
}

fn compare_rhythm(reference: &RhythmPattern, player: &RhythmPattern) -> f32 {
    if reference.inter_onset_intervals.is_empty() || player.inter_onset_intervals.is_empty() {
        return 0.0;
    }

    // Compare average intervals (tempo matching)
    let tempo_diff = (reference.avg_interval - player.avg_interval).abs();
    let tempo_similarity = (1.0 - (tempo_diff / reference.avg_interval.max(0.1))).max(0.0);

    // Compare tempo stability
    let stability_similarity = 1.0 - (reference.tempo_stability - player.tempo_stability).abs();

    // Weighted average
    0.6 * tempo_similarity + 0.4 * stability_similarity
}

fn calculate_pitch_accuracy(pitch_errors: &[PitchError]) -> f32 {
    if pitch_errors.is_empty() {
        return 1.0;
    }

    // Average cent difference, normalize to 0-1 (0 cents = 1.0, 100+ cents = 0.0)
    let avg_cents = pitch_errors
        .iter()
        .map(|e| e.cent_difference.abs())
        .sum::<f32>()
        / pitch_errors.len() as f32;

    (1.0 - (avg_cents / 100.0)).max(0.0)
}

fn find_note_differences(
    reference: &[NoteSequence],
    player: &[NoteSequence],
) -> (Vec<String>, Vec<String>) {
    let max_time_diff = 0.5;
    let mut missed_notes = Vec::new();
    let mut extra_notes = Vec::new();

    // Find missed notes (in reference but not in player)
    for ref_note in reference {
        let found = player.iter().any(|p| {
            (p.start_time - ref_note.start_time).abs() <= max_time_diff
                && p.note_name == ref_note.note_name
        });

        if !found {
            missed_notes.push(format!(
                "{} at {:.2}s",
                ref_note.note_name, ref_note.start_time
            ));
        }
    }

    // Find extra notes (in player but not in reference)
    for player_note in player {
        let found = reference.iter().any(|r| {
            (r.start_time - player_note.start_time).abs() <= max_time_diff
                && r.note_name == player_note.note_name
        });

        if !found {
            extra_notes.push(format!(
                "{} at {:.2}s",
                player_note.note_name, player_note.start_time
            ));
        }
    }

    (missed_notes, extra_notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hz_to_midi() {
        assert_eq!(hz_to_midi(440.0), Some(69)); // A4
        assert_eq!(hz_to_midi(261.63), Some(60)); // C4
        assert_eq!(hz_to_midi(0.0), None);
    }

    #[test]
    fn test_midi_to_note_name() {
        assert_eq!(midi_to_note_name(69), "A4");
        assert_eq!(midi_to_note_name(60), "C4");
        assert_eq!(midi_to_note_name(72), "C5");
    }

    #[test]
    fn test_pitch_difference_cents() {
        let diff = pitch_difference_cents(440.0, 440.0);
        assert!((diff - 0.0).abs() < 0.01);

        let diff = pitch_difference_cents(440.0, 466.16); // A4 to A#4
        assert!((diff - 100.0).abs() < 1.0); // Should be ~100 cents
    }
}
