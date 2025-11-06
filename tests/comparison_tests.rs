use audio_ai::audio_analysis::{AnalysisResult, analyze_audio};
use audio_ai::comparison::{
    compare_recordings, extract_note_sequence, extract_rhythm_pattern, hz_to_midi, hz_to_note_name,
    midi_to_note_name, pitch_difference_cents,
};

#[test]
fn test_note_name_conversion() {
    // Test standard notes
    assert_eq!(hz_to_note_name(440.0), "A4"); // A4
    assert_eq!(hz_to_note_name(261.63), "C4"); // C4 (approximately)
    assert_eq!(hz_to_note_name(329.63), "E4"); // E4 (approximately)
    assert_eq!(hz_to_note_name(196.0), "G3"); // G3 (approximately)
}

#[test]
fn test_midi_conversion() {
    assert_eq!(hz_to_midi(440.0), Some(69)); // A4
    assert_eq!(midi_to_note_name(69), "A4");
    assert_eq!(midi_to_note_name(60), "C4");
    assert_eq!(midi_to_note_name(48), "C3");
}

#[test]
fn test_pitch_difference() {
    // Same note should be 0 cents
    let diff = pitch_difference_cents(440.0, 440.0);
    assert!((diff - 0.0).abs() < 0.01);

    // One semitone (A4 to A#4) should be ~100 cents
    let diff = pitch_difference_cents(440.0, 466.16);
    assert!((diff - 100.0).abs() < 1.0);

    // One octave (A4 to A5) should be 1200 cents
    let diff = pitch_difference_cents(440.0, 880.0);
    assert!((diff - 1200.0).abs() < 1.0);
}

#[test]
fn test_extract_note_sequence() {
    // Create a simple analysis result with a few notes
    let analysis = AnalysisResult {
        pitch_hz: vec![440.0, 440.0, 440.0, 494.0, 494.0, 523.25, 523.25],
        tempo_bpm: Some(120.0),
        onsets: vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
        spectral_centroid: vec![1000.0; 7],
        streaming: None,
    };

    let notes = extract_note_sequence(&analysis);

    // Should extract distinct notes (A4, B4, C5)
    assert!(notes.len() >= 2, "Should extract at least 2 distinct notes");

    // First note should be around A4
    if let Some(first_note) = notes.first() {
        assert!(first_note.note_name.starts_with("A"));
    }
}

#[test]
fn test_extract_rhythm_pattern() {
    let analysis = AnalysisResult {
        pitch_hz: vec![440.0; 10],
        tempo_bpm: Some(120.0),
        onsets: vec![0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5],
        spectral_centroid: vec![1000.0; 10],
        streaming: None,
    };

    let rhythm = extract_rhythm_pattern(&analysis);

    assert_eq!(rhythm.onset_times.len(), 10);
    assert_eq!(rhythm.inter_onset_intervals.len(), 9);

    // Should have consistent 0.5s intervals
    assert!((rhythm.avg_interval - 0.5).abs() < 0.01);

    // Tempo stability should be high (consistent rhythm)
    assert!(
        rhythm.tempo_stability > 0.8,
        "Consistent rhythm should have high stability"
    );
}

#[test]
fn test_compare_identical_recordings() {
    let analysis = AnalysisResult {
        pitch_hz: vec![440.0, 494.0, 523.25],
        tempo_bpm: Some(120.0),
        onsets: vec![0.0, 0.5, 1.0],
        spectral_centroid: vec![1000.0; 3],
        streaming: None,
    };

    let metrics = compare_recordings(&analysis, &analysis);

    // Comparing identical recordings should give perfect scores
    assert!(
        metrics.overall_similarity > 0.95,
        "Identical recordings should be very similar"
    );
    assert!(
        metrics.note_accuracy > 0.95,
        "Note accuracy should be very high"
    );
    assert_eq!(metrics.missed_notes.len(), 0, "Should have no missed notes");
    assert_eq!(metrics.extra_notes.len(), 0, "Should have no extra notes");
}

#[test]
fn test_compare_different_recordings() {
    let reference = AnalysisResult {
        pitch_hz: vec![440.0, 494.0, 523.25],
        tempo_bpm: Some(120.0),
        onsets: vec![0.0, 0.5, 1.0],
        spectral_centroid: vec![1000.0; 3],
        streaming: None,
    };

    let player = AnalysisResult {
        pitch_hz: vec![440.0, 523.25], // Missing middle note
        tempo_bpm: Some(115.0),
        onsets: vec![0.0, 1.1], // Different timing
        spectral_centroid: vec![1000.0; 2],
        streaming: None,
    };

    let metrics = compare_recordings(&reference, &player);

    // Different recordings should have lower similarity
    assert!(
        metrics.overall_similarity < 0.95,
        "Different recordings should be less similar"
    );
    assert!(
        metrics.note_accuracy < 1.0,
        "Note accuracy should be less than perfect"
    );
}

#[test]
fn test_pitch_accuracy_with_out_of_tune_notes() {
    let reference = AnalysisResult {
        pitch_hz: vec![440.0, 440.0, 440.0], // A4 perfectly in tune
        tempo_bpm: Some(120.0),
        onsets: vec![0.0, 0.5, 1.0],
        spectral_centroid: vec![1000.0; 3],
        streaming: None,
    };

    let player = AnalysisResult {
        pitch_hz: vec![466.16, 466.16, 466.16], // A#4 - one semitone sharp (~100 cents)
        tempo_bpm: Some(120.0),
        onsets: vec![0.0, 0.5, 1.0],
        spectral_centroid: vec![1000.0; 3],
        streaming: None,
    };

    let metrics = compare_recordings(&reference, &player);

    // Should detect pitch errors (notes are completely wrong)
    assert!(
        metrics.pitch_accuracy < 1.0,
        "Should detect out-of-tune notes"
    );
    assert!(
        !metrics.pitch_errors.is_empty(),
        "Should report pitch errors"
    );
}

#[test]
fn test_timing_accuracy() {
    let reference = AnalysisResult {
        pitch_hz: vec![440.0, 494.0, 523.25],
        tempo_bpm: Some(120.0),
        onsets: vec![0.0, 0.5, 1.0],
        spectral_centroid: vec![1000.0; 3],
        streaming: None,
    };

    let player = AnalysisResult {
        pitch_hz: vec![440.0, 494.0, 523.25],
        tempo_bpm: Some(120.0),
        onsets: vec![0.0, 0.6, 1.1], // Slightly late
        spectral_centroid: vec![1000.0; 3],
        streaming: None,
    };

    let metrics = compare_recordings(&reference, &player);

    // Should detect timing issues
    assert!(metrics.timing_accuracy < 1.0, "Should detect timing errors");
    assert!(
        !metrics.timing_errors.is_empty(),
        "Should report timing errors"
    );
}

#[test]
fn test_empty_analysis() {
    let empty = AnalysisResult {
        pitch_hz: vec![],
        tempo_bpm: None,
        onsets: vec![],
        spectral_centroid: vec![],
        streaming: None,
    };

    let notes = extract_note_sequence(&empty);
    assert_eq!(notes.len(), 0, "Empty analysis should produce no notes");

    let rhythm = extract_rhythm_pattern(&empty);
    assert_eq!(
        rhythm.onset_times.len(),
        0,
        "Empty analysis should have no onsets"
    );
}
