/// Integration tests for AI client functionality with stubbed responses
use audio_ai::ai_client::{AIClient, MockAIClient};
use audio_ai::audio_analysis::analyze_audio;
use audio_ai::comparison::compare_recordings;
use std::path::PathBuf;

/// Helper to get the path to a test data file
fn test_data_path(filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(filename)
}

#[tokio::test]
async fn test_ai_feedback_for_single_file_analysis() {
    // Load a test audio file
    let path = test_data_path("tone_a4_440hz.wav");
    let analysis = analyze_audio(path.to_str().unwrap())
        .expect("Failed to analyze test file");

    // Create a mock AI client with a stubbed response
    let expected_feedback = "Excellent pitch stability on the A4 note! Your intonation is spot-on at 440Hz. \
        Try varying your dynamics to add more expression.";
    
    let mock_client = MockAIClient::new()
        .with_single_response(expected_feedback.to_string());

    // Send analysis to mock AI
    let result = mock_client
        .send_single_analysis(&analysis, "tone_a4_440hz.wav")
        .await
        .expect("Failed to get AI feedback");

    // Verify the stubbed response was returned
    assert_eq!(result.content, expected_feedback);
    assert_eq!(mock_client.single_call_count(), 1);
}

#[tokio::test]
async fn test_ai_feedback_for_comparison() {
    // Load two test audio files for comparison
    let reference_path = test_data_path("melody_simple.wav");
    let variant_path = test_data_path("melody_simple_pitch_variant.wav");

    let reference = analyze_audio(reference_path.to_str().unwrap())
        .expect("Failed to analyze reference file");
    let variant = analyze_audio(variant_path.to_str().unwrap())
        .expect("Failed to analyze variant file");

    // Generate comparison metrics
    let metrics = compare_recordings(&reference, &variant);

    // Create a mock AI client with a stubbed comparison response
    let expected_feedback = "Good effort! Your overall similarity is 85%. \
        I noticed some pitch variations on notes 2 and 4 - they're slightly sharp. \
        Practice those specific notes with a tuner. Your timing is excellent though!";
    
    let mock_client = MockAIClient::new()
        .with_comparison_response(expected_feedback.to_string());

    // Send comparison to mock AI
    let result = mock_client
        .send_comparison(&metrics, "reference.wav", "student.wav")
        .await
        .expect("Failed to get AI feedback");

    // Verify the stubbed response was returned
    assert_eq!(result.content, expected_feedback);
    assert_eq!(mock_client.comparison_call_count(), 1);
}

#[tokio::test]
async fn test_ai_feedback_for_multiple_comparisons() {
    // Test that the mock client can handle multiple calls
    let mock_client = MockAIClient::new()
        .with_comparison_response("First feedback response".to_string());

    // Create a simple metrics object
    let metrics = audio_ai::comparison::ComparisonMetrics {
        overall_similarity: 0.90,
        note_accuracy: 0.95,
        pitch_accuracy: 0.88,
        timing_accuracy: 0.92,
        rhythm_accuracy: 0.90,
        missed_notes: vec![],
        extra_notes: vec![],
        pitch_errors: vec![],
        timing_errors: vec![],
    };

    // Make multiple calls
    for i in 1..=3 {
        let result = mock_client
            .send_comparison(&metrics, "ref.wav", &format!("student_{}.wav", i))
            .await
            .expect("Failed to get AI feedback");
        
        assert_eq!(result.content, "First feedback response");
    }

    assert_eq!(mock_client.comparison_call_count(), 3);
}

#[tokio::test]
async fn test_ai_feedback_for_scale_analysis() {
    // Load the C major scale test file
    let path = test_data_path("scale_c_major.wav");
    let analysis = analyze_audio(path.to_str().unwrap())
        .expect("Failed to analyze scale");

    // Create a mock AI client with scale-specific feedback
    let expected_feedback = "Nice C major scale! All 8 notes are present and clearly articulated. \
        Your ascending pattern shows good finger coordination. \
        The tempo is consistent at 120 BPM. \
        For improvement, focus on making the transitions between notes smoother, \
        especially between B4 and C5.";
    
    let mock_client = MockAIClient::new()
        .with_single_response(expected_feedback.to_string());

    // Send analysis to mock AI
    let result = mock_client
        .send_single_analysis(&analysis, "scale_c_major.wav")
        .await
        .expect("Failed to get AI feedback");

    // Verify the response
    assert_eq!(result.content, expected_feedback);
    assert_eq!(mock_client.single_call_count(), 1);
}

#[tokio::test]
async fn test_ai_feedback_for_rhythm_analysis() {
    // Load a rhythm test file
    let path = test_data_path("rhythm_quarter_notes_120bpm.wav");
    let analysis = analyze_audio(path.to_str().unwrap())
        .expect("Failed to analyze rhythm pattern");

    // Create a mock AI client with rhythm-specific feedback
    let expected_feedback = "Excellent rhythm work! Your quarter notes are evenly spaced at 120 BPM. \
        The tempo stability is very high, showing good internal timing. \
        This is fundamental to good musicianship - keep it up!";
    
    let mock_client = MockAIClient::new()
        .with_single_response(expected_feedback.to_string());

    // Send analysis to mock AI
    let result = mock_client
        .send_single_analysis(&analysis, "rhythm_quarter_notes_120bpm.wav")
        .await
        .expect("Failed to get AI feedback");

    // Verify the response
    assert_eq!(result.content, expected_feedback);
}

#[tokio::test]
async fn test_ai_feedback_handles_poor_performance() {
    // Create metrics representing a poor performance
    let metrics = audio_ai::comparison::ComparisonMetrics {
        overall_similarity: 0.45,
        note_accuracy: 0.50,
        pitch_accuracy: 0.40,
        timing_accuracy: 0.48,
        rhythm_accuracy: 0.42,
        missed_notes: vec!["E4 at 1.2s".to_string(), "G4 at 2.5s".to_string()],
        extra_notes: vec!["F#4 at 1.8s".to_string()],
        pitch_errors: vec![],
        timing_errors: vec![],
    };

    // Create a mock AI client with constructive critical feedback
    let expected_feedback = "This piece needs more practice. Your overall similarity is 45%, \
        which indicates several areas for improvement:\n\
        1. You missed 2 notes (E4 and G4) - make sure to play all notes in the piece\n\
        2. You played an extra F# that isn't in the original\n\
        3. Pitch accuracy is at 40% - work with a tuner to improve intonation\n\
        4. Timing is off - practice with a metronome at a slower tempo first\n\
        \n\
        Don't be discouraged! Break the piece into smaller sections and master each one \
        before putting it all together. You've got this!";
    
    let mock_client = MockAIClient::new()
        .with_comparison_response(expected_feedback.to_string());

    // Send comparison to mock AI
    let result = mock_client
        .send_comparison(&metrics, "reference.wav", "student.wav")
        .await
        .expect("Failed to get AI feedback");

    // Verify the constructive criticism was returned
    assert_eq!(result.content, expected_feedback);
    assert!(result.content.contains("45%"));
    assert!(result.content.contains("improvement"));
    assert!(result.content.contains("practice"));
}

#[tokio::test]
async fn test_ai_feedback_for_excellent_performance() {
    // Create metrics representing an excellent performance
    let metrics = audio_ai::comparison::ComparisonMetrics {
        overall_similarity: 0.97,
        note_accuracy: 0.98,
        pitch_accuracy: 0.96,
        timing_accuracy: 0.98,
        rhythm_accuracy: 0.97,
        missed_notes: vec![],
        extra_notes: vec![],
        pitch_errors: vec![],
        timing_errors: vec![],
    };

    // Create a mock AI client with positive feedback
    let expected_feedback = "Outstanding performance! Your overall similarity is 97%, \
        which is excellent. You've clearly put in the practice time:\n\
        - Note accuracy: 98% - nearly perfect note selection\n\
        - Pitch accuracy: 96% - great intonation\n\
        - Timing accuracy: 98% - very tight timing\n\
        - Rhythm accuracy: 97% - solid rhythmic feel\n\
        \n\
        You're playing at a very high level. To reach perfection, focus on the few \
        remaining pitch variations - they're very subtle but worth addressing. \
        Consider recording yourself and comparing to hear the tiny differences. \
        Excellent work overall!";
    
    let mock_client = MockAIClient::new()
        .with_comparison_response(expected_feedback.to_string());

    // Send comparison to mock AI
    let result = mock_client
        .send_comparison(&metrics, "reference.wav", "student.wav")
        .await
        .expect("Failed to get AI feedback");

    // Verify the positive feedback was returned
    assert_eq!(result.content, expected_feedback);
    assert!(result.content.contains("97%"));
    assert!(result.content.contains("Outstanding") || result.content.contains("excellent"));
}
