mod ai_client;
mod audio_analysis;
mod comparison;
mod processor;
mod streaming;

use ai_client::{AIClient, OpenAIClient};
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
        eprintln!("Usage:");
        eprintln!(
            "  {} <audio_file>                     - Analyze a single file",
            args[0]
        );
        eprintln!(
            "  {} <reference_file> <player_file>  - Compare player to reference",
            args[0]
        );
        eprintln!(
            "  {} --stream                         - Start streaming analysis",
            args[0]
        );
        return Ok(());
    }

    if args[1] == "--stream" {
        println!("Starting streaming guitar analysis...");
        streaming::start_streaming_analysis()?;
        return Ok(());
    }

    // Check if we're doing comparison (2 files) or single file analysis
    let is_comparison = args.len() >= 3;

    if is_comparison {
        // Comparison mode: reference vs player
        let reference_path = &args[1];
        let player_path = &args[2];

        println!("=== Comparison Mode ===");
        println!("Reference: {}", reference_path);
        println!("Player: {}", player_path);
        println!();

        // Analyze both files
        use crate::audio_analysis::analyze_audio;
        use crate::comparison::compare_recordings;
        use crate::processor::export_optimized_for_gpt;

        println!("Analyzing reference recording...");
        let reference_analysis = analyze_audio(reference_path)?;

        println!("Analyzing player recording...");
        let player_analysis = analyze_audio(player_path)?;

        // Generate comparison metrics
        println!("Computing comparison metrics...");
        let metrics = compare_recordings(&reference_analysis, &player_analysis);

        // Display quick summary
        println!("\n=== Quick Summary ===");
        println!(
            "Overall Similarity: {:.1}%",
            metrics.overall_similarity * 100.0
        );
        println!("Note Accuracy: {:.1}%", metrics.note_accuracy * 100.0);
        println!("Pitch Accuracy: {:.1}%", metrics.pitch_accuracy * 100.0);
        println!("Timing Accuracy: {:.1}%", metrics.timing_accuracy * 100.0);
        println!("Rhythm Accuracy: {:.1}%", metrics.rhythm_accuracy * 100.0);

        if !metrics.missed_notes.is_empty() {
            println!(
                "\nMissed Notes ({}): {:?}",
                metrics.missed_notes.len(),
                metrics.missed_notes.iter().take(5).collect::<Vec<_>>()
            );
        }
        if !metrics.extra_notes.is_empty() {
            println!(
                "Extra Notes ({}): {:?}",
                metrics.extra_notes.len(),
                metrics.extra_notes.iter().take(5).collect::<Vec<_>>()
            );
        }

        // Export optimized comparison data
        export_optimized_for_gpt(
            &player_analysis,
            "analysis_optimized.json",
            Some(&reference_analysis),
        )?;
        println!("\nExported optimized comparison to analysis_optimized.json");

        // Send to AI for detailed feedback
        if let Ok(client) = OpenAIClient::new() {
            match client
                .send_comparison(&metrics, reference_path, player_path)
                .await
            {
                Ok(feedback) => {
                    println!("\n=== AI Feedback ===");
                    println!("{}", feedback.content);
                }
                Err(e) => {
                    eprintln!("Failed to get AI feedback: {}", e);
                }
            }
        } else {
            println!("\nSkipping AI feedback (OPENAI_API_KEY not set)");
        }
    } else {
        // Single file analysis mode
        let file_path = &args[1];
        println!("Analyzing guitar audio file: {}", file_path);

        // Open file for basic info
        let file = File::open(file_path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if file_path.ends_with(".wav") {
            hint.with_extension("wav");
        }

        let probed = get_probe().format(&hint, mss, &Default::default(), &Default::default())?;
        let mut format = probed.format;

        let track = format
            .default_track()
            .ok_or(Error::DecodeError("No default track"))?;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;

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

        // Analyze audio
        use crate::audio_analysis::analyze_audio;
        use crate::comparison::extract_note_sequence;
        use crate::processor::{export_for_gpt, export_optimized_for_gpt};

        let analysis = analyze_audio(file_path)?;

        // Export both old and new formats
        export_for_gpt(&analysis, "analysis_gpt.json")?;
        println!("Exported legacy format to analysis_gpt.json");

        export_optimized_for_gpt(&analysis, "analysis_optimized.json", None)?;
        println!("Exported optimized format to analysis_optimized.json");

        // Display summary
        let note_seq = extract_note_sequence(&analysis);
        let detected_pitch = format!("{:.2} Hz", analysis.pitch_hz.first().unwrap_or(&0.0));
        let detected_tempo = analysis
            .tempo_bpm
            .map(|t| format!("{:.1} bpm", t))
            .unwrap_or("N/A".to_string());
        let detected_onsets = analysis.onsets.len();

        println!("\n=== Analysis Summary ===");
        println!(
            "Features -> Pitch: {}, Tempo: {}, Onsets: {}",
            detected_pitch, detected_tempo, detected_onsets
        );
        println!("Detected {} distinct notes", note_seq.len());
        if !note_seq.is_empty() {
            println!(
                "First few notes: {:?}",
                note_seq
                    .iter()
                    .take(5)
                    .map(|n| &n.note_name)
                    .collect::<Vec<_>>()
            );
        }

        // Send to AI for analysis
        if let Ok(client) = OpenAIClient::new() {
            match client.send_single_analysis(&analysis, file_path).await {
                Ok(feedback) => {
                    println!("\n=== AI Feedback ===");
                    println!("{}", feedback.content);
                }
                Err(e) => {
                    eprintln!("Failed to get AI feedback: {}", e);
                }
            }
        } else {
            println!("\nSkipping AI feedback (OPENAI_API_KEY not set)");
        }
    }

    Ok(())
}
