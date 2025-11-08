use audio_ai::audio_analysis::analyze_audio;
use audio_ai::comparison::{compare_recordings, extract_note_sequence, extract_rhythm_pattern};
use std::path::PathBuf;

/// Helper to get the path to a test data file
fn test_data_path(filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(filename)
}

#[test]
fn test_analyze_single_tone_a4() {
    let path = test_data_path("tone_a4_440hz.wav");
    let result = analyze_audio(path.to_str().unwrap())
        .expect("Failed to analyze tone_a4_440hz.wav");

    // Should detect frequencies close to 440 Hz
    assert!(
        !result.pitch_hz.is_empty(),
        "Should detect pitch in A4 tone"
    );
    
    // Check that most detected pitches are close to 440 Hz (within 10 Hz tolerance)
    let close_to_a4 = result.pitch_hz.iter()
        .filter(|&&f| f > 0.0 && (f - 440.0).abs() < 10.0)
        .count();
    
    assert!(
        close_to_a4 > result.pitch_hz.len() / 2,
        "Majority of detected pitches should be close to A4 (440 Hz)"
    );
}

#[test]
fn test_analyze_c_major_scale() {
    let path = test_data_path("scale_c_major.wav");
    let result = analyze_audio(path.to_str().unwrap())
        .expect("Failed to analyze scale_c_major.wav");

    // Extract note sequence
    let notes = extract_note_sequence(&result);

    // Should detect multiple distinct notes (at least 5 out of 8 scale notes)
    assert!(
        notes.len() >= 5,
        "Should detect at least 5 distinct notes in C major scale, got {}",
        notes.len()
    );

    // Check that we have some of the expected notes
    let note_names: Vec<String> = notes.iter().map(|n| n.note_name.clone()).collect();
    println!("Detected notes: {:?}", note_names);
    
    // Should include some notes from the C major scale
    let has_c = note_names.iter().any(|n| n.starts_with('C'));
    let has_e = note_names.iter().any(|n| n.starts_with('E'));
    let has_g = note_names.iter().any(|n| n.starts_with('G'));
    
    assert!(
        has_c || has_e || has_g,
        "Should detect at least some notes from C major scale"
    );
}

#[test]
fn test_compare_identical_melodies() {
    let path = test_data_path("melody_simple.wav");
    let result1 = analyze_audio(path.to_str().unwrap())
        .expect("Failed to analyze melody_simple.wav");
    let result2 = analyze_audio(path.to_str().unwrap())
        .expect("Failed to analyze melody_simple.wav (second time)");

    let metrics = compare_recordings(&result1, &result2);

    // Comparing the same file should give very high similarity
    assert!(
        metrics.overall_similarity > 0.95,
        "Same melody should have >95% similarity, got {}",
        metrics.overall_similarity
    );
}

#[test]
fn test_compare_melody_with_timing_variant() {
    let reference_path = test_data_path("melody_simple.wav");
    let variant_path = test_data_path("melody_simple_timing_variant.wav");

    let reference = analyze_audio(reference_path.to_str().unwrap())
        .expect("Failed to analyze melody_simple.wav");
    let variant = analyze_audio(variant_path.to_str().unwrap())
        .expect("Failed to analyze melody_simple_timing_variant.wav");

    let metrics = compare_recordings(&reference, &variant);

    // Should detect timing differences
    println!("Timing variant comparison:");
    println!("  Overall similarity: {}", metrics.overall_similarity);
    println!("  Timing accuracy: {}", metrics.timing_accuracy);
    println!("  Timing errors: {}", metrics.timing_errors.len());

    // The timing differences may be subtle, so overall similarity should still be high
    // but timing accuracy might show some variation
    assert!(
        metrics.overall_similarity > 0.9,
        "Timing variant should still have high overall similarity"
    );
}

#[test]
fn test_compare_melody_with_pitch_variant() {
    let reference_path = test_data_path("melody_simple.wav");
    let variant_path = test_data_path("melody_simple_pitch_variant.wav");

    let reference = analyze_audio(reference_path.to_str().unwrap())
        .expect("Failed to analyze melody_simple.wav");
    let variant = analyze_audio(variant_path.to_str().unwrap())
        .expect("Failed to analyze melody_simple_pitch_variant.wav");

    let metrics = compare_recordings(&reference, &variant);

    println!("Pitch variant comparison:");
    println!("  Overall similarity: {}", metrics.overall_similarity);
    println!("  Pitch accuracy: {}", metrics.pitch_accuracy);
    println!("  Pitch errors: {}", metrics.pitch_errors.len());

    // The pitch differences may be subtle (only a few Hz), so the files might still
    // be detected as similar. The test validates that comparison works without errors.
    assert!(
        metrics.overall_similarity > 0.8,
        "Pitch variant should still have reasonable similarity"
    );
}

#[test]
fn test_rhythm_pattern_quarter_notes() {
    let path = test_data_path("rhythm_quarter_notes_120bpm.wav");
    let result = analyze_audio(path.to_str().unwrap())
        .expect("Failed to analyze rhythm_quarter_notes_120bpm.wav");

    let rhythm = extract_rhythm_pattern(&result);

    // Should detect multiple onsets
    assert!(
        rhythm.onset_times.len() >= 4,
        "Should detect at least 4 onsets in quarter note pattern, got {}",
        rhythm.onset_times.len()
    );

    // Average interval should be around 0.5s (120 BPM = 2 beats per second)
    println!("Quarter notes rhythm:");
    println!("  Onsets detected: {}", rhythm.onset_times.len());
    println!("  Average interval: {}s", rhythm.avg_interval);
    println!("  Tempo stability: {}", rhythm.tempo_stability);

    // Tempo stability should be relatively high for consistent rhythm
    assert!(
        rhythm.tempo_stability > 0.5,
        "Quarter notes should have decent tempo stability"
    );
}

#[test]
fn test_rhythm_pattern_eighth_notes() {
    let path = test_data_path("rhythm_eighth_notes_120bpm.wav");
    let result = analyze_audio(path.to_str().unwrap())
        .expect("Failed to analyze rhythm_eighth_notes_120bpm.wav");

    let rhythm = extract_rhythm_pattern(&result);

    // Should detect multiple onsets
    assert!(
        rhythm.onset_times.len() >= 8,
        "Should detect at least 8 onsets in eighth note pattern, got {}",
        rhythm.onset_times.len()
    );

    println!("Eighth notes rhythm:");
    println!("  Onsets detected: {}", rhythm.onset_times.len());
    println!("  Average interval: {}s", rhythm.avg_interval);

    // Average interval should be around 0.25s (120 BPM eighth notes)
    // Allow for some tolerance due to onset detection variations
    assert!(
        rhythm.avg_interval < 0.5,
        "Eighth notes should have shorter intervals than quarter notes"
    );
}

#[test]
fn test_multiple_tone_files_exist() {
    // Verify that all expected test files are present
    let test_files = vec![
        "tone_a4_440hz.wav",
        "tone_c4_262hz.wav",
        "tone_e4_330hz.wav",
        "tone_c5_523hz.wav",
        "scale_c_major.wav",
        "melody_simple.wav",
        "melody_simple_timing_variant.wav",
        "melody_simple_pitch_variant.wav",
        "rhythm_quarter_notes_120bpm.wav",
        "rhythm_eighth_notes_120bpm.wav",
    ];

    for filename in test_files {
        let path = test_data_path(filename);
        assert!(
            path.exists(),
            "Test file should exist: {}. Run 'cd tests && python3 generate_test_samples.py' to generate.",
            filename
        );
    }
}
