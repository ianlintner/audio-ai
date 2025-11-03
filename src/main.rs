mod audio_analysis;
mod processor;
mod streaming;

use std::env;
use std::fs::File;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <audio_file|--stream>", args[0]);
        return Ok(());
    }

    if args[1] == "--stream" {
        println!("Starting streaming guitar analysis...");
        streaming::start_streaming_analysis()?;
        return Ok(());
    }

    let file_path = &args[1];
    println!("Analyzing guitar audio file: {}", file_path);

    // Open file
    let file = File::open(file_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint to help the format registry guess what format reader is appropriate.
    let mut hint = Hint::new();
    if file_path.ends_with(".wav") {
        hint.with_extension("wav");
    }

    // Use the default probe to guess the format.
    let probed = get_probe().format(&hint, mss, &Default::default(), &Default::default())?;
    let mut format = probed.format;

    // Get the default track.
    let track = format
        .default_track()
        .ok_or(Error::DecodeError("No default track"))?;

    // Create a decoder for the track.
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    // Decode packets
    if let Ok(packet) = format.next_packet() {
        let decoded = decoder.decode(&packet)?;
        let spec = decoded.spec();
        let duration = decoded.capacity() as f32 / spec.rate as f32;
        println!(
            "Decoded {} samples, duration ~{:.2} sec",
            decoded.capacity(),
            duration
        );
    }

    // Use our audio analysis module
    use crate::audio_analysis::analyze_audio;

    let analysis = analyze_audio(file_path)?;
    // println!("Analysis Results: {:?}", analysis);

    // Export GPT-friendly JSON
    use crate::processor::export_for_gpt;
    export_for_gpt(&analysis, "analysis_gpt.json")?;
    println!("Exported GPT-friendly analysis to analysis_gpt.json");

    let detected_pitch = format!("{:.2} Hz", analysis.pitch_hz.first().unwrap_or(&0.0));
    let detected_tempo = analysis
        .tempo_bpm
        .map(|t| format!("{:.1} bpm", t))
        .unwrap_or("N/A".to_string());
    let detected_onsets = analysis.onsets.len();

    println!(
        "Extracted features -> Pitch: {}, Tempo: {}, Onsets: {}",
        detected_pitch, detected_tempo, detected_onsets
    );

    // Send extracted features to OpenAI for analysis
    let client = reqwest::Client::new();
    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let prompt = format!(
        "Analyze this guitar recording. Provide feedback on timing, accuracy, and tone. \
        Features extracted: Pitch={}, Tempo={}, Onsets={}. \
        File analyzed: {}",
        detected_pitch, detected_tempo, detected_onsets, file_path
    );

    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {"role": "system", "content": "You are a guitar teacher analyzing student recordings."},
            {"role": "user", "content": prompt}
        ]
    });

    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;

    let json: serde_json::Value = res.json().await?;
    if let Some(feedback) = json["choices"][0]["message"]["content"].as_str() {
        println!("AI Feedback: {}", feedback);
    } else {
        println!("AI response: {}", json);
    }

    Ok(())
}
