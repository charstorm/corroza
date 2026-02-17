//! CLI tool for generating audio from musical transcription
//!
//! Usage: play <input.txt> [output.wav]
//!
//! If output is not specified, generates <input>.wav

use corroza::generator::fm_synth::FmSynthParams;
use corroza::pipeline::parser::parse_transcription;
use corroza::pipeline::scheduler::{Pipeline, PipelineConfig};
use corroza::pipeline::voicemgr::VoiceConfig;
use std::env;
use std::fs;
use std::process;

const USAGE: &str = "Usage: play <input.txt> [output.wav]

Generate audio from musical transcription file.

Arguments:
  input.txt     Path to transcription file
  output.wav    Output WAV file path (optional, defaults to <input>.wav)

Examples:
  play song.txt
  play song.txt output.wav
";

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("{}", USAGE);
        process::exit(1);
    }

    let input_path = &args[1];

    // Determine output path
    let output_path = if args.len() >= 3 {
        args[2].clone()
    } else {
        // Default: replace .txt with .wav or append .wav
        if input_path.ends_with(".txt") {
            format!("{}.wav", &input_path[..input_path.len() - 4])
        } else {
            format!("{}.wav", input_path)
        }
    };

    // Read input file
    let content = match fs::read_to_string(input_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading {}: {}", input_path, e);
            process::exit(1);
        }
    };

    // Parse transcription
    let events = match parse_transcription(&content) {
        Ok(events) => events,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    println!("Parsed {} event groups", events.len());

    // Configure pipeline with defaults
    let fm_params = FmSynthParams::new(
        vec![2, 5, 9],
        vec![1.0, 2.0, 1.0],
        0.1, // phase_per_sample placeholder
        1.0, // mod_depth
    );

    let voice_config = VoiceConfig {
        fm_params,
        attack_samples: 4410, // 100ms at 44.1kHz
        decay_samples: 8820,  // 200ms at 44.1kHz
        sustain_level: 0.7,
        release_samples: 13230, // 300ms at 44.1kHz
    };

    let config = PipelineConfig {
        sample_rate: 44100,
        frame_size: 64,
        timestep_samples: 1000, // ≈22.7ms at 44.1kHz
        voice_config,
        base_frequency: 110.0, // 1C = 110 Hz
    };

    println!("Configuration:");
    println!("  Sample rate: {} Hz", config.sample_rate);
    println!("  Frame size: {} samples", config.frame_size);
    println!("  Timestep: {} samples", config.timestep_samples);
    println!("  Base frequency: {} Hz", config.base_frequency);
    println!();

    // Create pipeline and generate audio
    let mut pipeline = Pipeline::new(config, events);

    println!("Generating audio...");

    match pipeline.generate_wav(&output_path) {
        Ok(_) => {
            println!("✓ Generated {}", output_path);
        }
        Err(e) => {
            eprintln!("Error writing WAV file: {}", e);
            process::exit(1);
        }
    }
}
