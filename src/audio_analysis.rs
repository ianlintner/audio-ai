use aubio::{Onset, Pitch, Tempo};
use hound::WavReader;
use rustfft::{FftPlanner, num_complex::Complex};
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct NoteEvent {
    pub time: f32,
    pub pitch_hz: f32,
    pub confidence: f32,
}

/// Compare two audio files (player vs reference) and return similarity score
#[allow(dead_code)]
pub fn compare_to_reference(player_path: &str, reference_path: &str) -> anyhow::Result<f32> {
    let player_analysis = analyze_audio(player_path)?;
    let reference_analysis = analyze_audio(reference_path)?;

    // Simple similarity: cosine similarity of spectral centroids
    let min_len = player_analysis
        .spectral_centroid
        .len()
        .min(reference_analysis.spectral_centroid.len());
    if min_len == 0 {
        return Ok(0.0);
    }

    let a = &player_analysis.spectral_centroid[..min_len];
    let b = &reference_analysis.spectral_centroid[..min_len];

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 {
        Ok(dot / (norm_a * norm_b))
    } else {
        Ok(0.0)
    }
}

#[derive(Serialize, Debug)]
pub struct StreamingState {
    pub current_time: f32,
    pub detected_notes: Vec<NoteEvent>,
}

#[derive(Serialize, Debug)]
pub struct AnalysisResult {
    pub pitch_hz: Vec<f32>,
    pub tempo_bpm: Option<f32>,
    pub onsets: Vec<f32>,
    pub spectral_centroid: Vec<f32>,
    pub streaming: Option<StreamingState>,
}

pub fn analyze_audio(file_path: &str) -> anyhow::Result<AnalysisResult> {
    // Load WAV file
    let mut reader = WavReader::open(file_path)?;
    let spec = reader.spec();
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();

    let sample_rate = spec.sample_rate as usize;
    let hop_size = 512;
    let win_size = 1024;

    // Aubio pitch, tempo, onset
    let mut pitch = Pitch::new(
        aubio::PitchMode::Yin,
        win_size,
        hop_size,
        sample_rate as u32,
    )?;
    let mut tempo = Tempo::new(
        aubio::OnsetMode::Complex,
        win_size,
        hop_size,
        sample_rate as u32,
    )?;
    let mut onset = Onset::new(
        aubio::OnsetMode::Complex,
        win_size,
        hop_size,
        sample_rate as u32,
    )?;

    // Configure pitch detection
    pitch.set_unit(aubio::PitchUnit::Hz);
    pitch.set_silence(-40.0); // dB threshold

    let mut pitches = Vec::new();
    let mut onsets = Vec::new();
    let mut spectral_centroid = Vec::new();
    let mut tempo_bpm = None;

    // FFT planner
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(win_size);

    for (i, frame) in samples.chunks(hop_size).enumerate() {
        let mut input = vec![0.0; win_size];
        for (j, &s) in frame.iter().enumerate() {
            if j < win_size {
                input[j] = s;
            }
        }

        // Pitch detection with Hann window
        let hann: Vec<f32> = (0..win_size)
            .map(|n| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * n as f32 / win_size as f32).cos())
            .collect();
        let windowed: Vec<f32> = input.iter().zip(hann.iter()).map(|(x, w)| x * w).collect();

        let p = pitch.do_result(&windowed)?;
        if p > 0.0 {
            pitches.push(p);
        }

        // Onset detection
        let onset_val = onset.do_result(&input)?;
        if onset_val > 0.0 {
            onsets.push(i as f32 * hop_size as f32 / sample_rate as f32);
        }

        // Tempo detection
        let tempo_val = tempo.do_result(&input)?;
        if tempo_val > 0.0 {
            tempo_bpm = Some(tempo.get_bpm());
        }

        // Spectral centroid
        let mut buffer: Vec<Complex<f32>> =
            input.iter().map(|&x| Complex { re: x, im: 0.0 }).collect();
        fft.process(&mut buffer);
        let mags: Vec<f32> = buffer.iter().map(|c| c.norm()).collect();
        let freqs: Vec<f32> = (0..mags.len())
            .map(|k| k as f32 * sample_rate as f32 / win_size as f32)
            .collect();
        let num: f32 = mags.iter().zip(freqs.iter()).map(|(m, f)| m * f).sum();
        let den: f32 = mags.iter().sum();
        if den > 0.0 {
            spectral_centroid.push(num / den);
        }
    }

    Ok(AnalysisResult {
        pitch_hz: pitches,
        tempo_bpm,
        onsets,
        spectral_centroid,
        streaming: None,
    })
}

/// Incremental streaming analysis for live audio chunks
pub fn analyze_stream_chunk(
    chunk: &[f32],
    sample_rate: usize,
    state: &mut StreamingState,
    pitch: &mut Pitch,
    onset: &mut Onset,
) -> Option<NoteEvent> {
    let p = pitch.do_result(chunk).ok()?;
    let onset_val = onset.do_result(chunk).ok()?;

    state.current_time += chunk.len() as f32 / sample_rate as f32;

    if p > 0.0 {
        let note = NoteEvent {
            time: state.current_time,
            pitch_hz: p,
            confidence: 1.0,
        };
        state.detected_notes.push(note.clone());
        return Some(note);
    }

    if onset_val > 0.0 {
        // Could extend with onset-specific events
    }

    None
}
