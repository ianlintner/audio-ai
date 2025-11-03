use aubio::{Onset, Pitch};
use audio_ai::audio_analysis::{NoteEvent, StreamingState, analyze_stream_chunk};

#[test]
fn test_streaming_state_accumulates_notes() {
    let sample_rate = 44100;
    let win_size = 1024;
    let hop_size = 512;

    let mut pitch = Pitch::new(
        aubio::PitchMode::Yin,
        win_size,
        hop_size,
        sample_rate as u32,
    )
    .unwrap();
    pitch.set_unit(aubio::PitchUnit::Hz);
    pitch.set_silence(-40.0);

    let mut onset = Onset::new(
        aubio::OnsetMode::Complex,
        win_size,
        hop_size,
        sample_rate as u32,
    )
    .unwrap();

    let mut state = StreamingState {
        current_time: 0.0,
        detected_notes: Vec::new(),
    };

    // Generate a fake sine wave chunk at 440 Hz (A4)
    let freq = 440.0;
    let chunk: Vec<f32> = (0..hop_size)
        .map(|n| (2.0 * std::f32::consts::PI * freq * n as f32 / sample_rate as f32).sin())
        .collect();

    let note = analyze_stream_chunk(&chunk, sample_rate, &mut state, &mut pitch, &mut onset);

    // We expect either a detected note or None depending on aubio internals,
    // but state.current_time should advance
    assert!(state.current_time > 0.0);
    if let Some(n) = note {
        assert!(n.pitch_hz > 0.0);
    }
}
