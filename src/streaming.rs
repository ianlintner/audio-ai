use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use num_traits::ToPrimitive;
use aubio::{Onset, Pitch};
use crate::audio_analysis::{StreamingState, analyze_stream_chunk};

/// Starts real-time streaming analysis using CPAL for live guitar input
pub fn start_streaming_analysis() -> anyhow::Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No input device available"))?;
    let config = device.default_input_config()?;

    let sample_rate = config.sample_rate().0 as usize;
    let mut state = StreamingState {
        current_time: 0.0,
        detected_notes: Vec::new(),
    };

    // Aubio pitch and onset detectors
    let win_size = 1024;
    let hop_size = 512;
    let mut pitch = Pitch::new(aubio::PitchMode::Yin, win_size, hop_size, sample_rate as u32)?;
    pitch.set_unit(aubio::PitchUnit::Hz);
    pitch.set_silence(-40.0);

    let mut onset = Onset::new(aubio::OnsetMode::Complex, win_size, hop_size, sample_rate as u32)?;

    let err_fn = |err| eprintln!("Stream error: {}", err);

    let stream = match config.sample_format() {
        SampleFormat::F32 => build_input_stream::<f32>(&device, &config.into(), sample_rate, &mut state, &mut pitch, &mut onset, err_fn)?,
        SampleFormat::I16 => build_input_stream::<i16>(&device, &config.into(), sample_rate, &mut state, &mut pitch, &mut onset, err_fn)?,
        SampleFormat::U16 => build_input_stream::<u16>(&device, &config.into(), sample_rate, &mut state, &mut pitch, &mut onset, err_fn)?,
        _ => return Err(anyhow::anyhow!("Unsupported sample format")),
    };

    stream.play()?;
    println!("Streaming analysis started. Play your guitar...");

    std::thread::sleep(std::time::Duration::from_secs(30));
    Ok(())
}

fn build_input_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_rate: usize,
    state: &mut StreamingState,
    _pitch: &mut Pitch,
    _onset: &mut Onset,
    err_fn: impl Fn(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, anyhow::Error>
where
    T: cpal::Sample + cpal::SizedSample + ToPrimitive,
{
    let channels = config.channels as usize;

    // Wrap state in Arc<Mutex<>> so it can be safely shared across threads
    use std::sync::{Arc, Mutex};
    let state: Arc<Mutex<StreamingState>> = Arc::new(Mutex::new(StreamingState {
        current_time: state.current_time,
        detected_notes: state.detected_notes.clone(),
    }));

    let stream = device.build_input_stream(
        config,
        {
            let state = Arc::clone(&state);

            move |data: &[T], _: &cpal::InputCallbackInfo| {
                use aubio::{Onset, Pitch};

                let mono: Vec<f32> = data
                    .chunks(channels)
                    .map(|frame| frame[0].to_f32().unwrap_or(0.0))
                    .collect();

                if let Ok(mut state) = state.lock() {
                    // Recreate pitch and onset detectors inside the callback (thread-local)
                    let mut pitch = Pitch::new(aubio::PitchMode::Yin, 1024, 512, sample_rate as u32).unwrap();
                    pitch.set_unit(aubio::PitchUnit::Hz);
                    pitch.set_silence(-40.0);

                    let mut onset = Onset::new(aubio::OnsetMode::Complex, 1024, 512, sample_rate as u32).unwrap();

                    if let Some(note) = analyze_stream_chunk(&mono, sample_rate, &mut state, &mut pitch, &mut onset) {
                        println!("Detected note: {:?}", note);
                    }
                }
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}
