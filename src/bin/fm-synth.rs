use corroza::generator::adsr::AdsrGenerator;
use corroza::generator::fm_synth::{FmSynthGenerator, FmSynthParams};
use corroza::generator::{GeneratorState, SignalGenerator};
use corroza::wav::write_wav_16bit;

/// Parse comma-separated integers (e.g., "2,5,9")
fn parse_harmonics(s: &str) -> Vec<usize> {
    s.split(',')
        .map(|s| s.trim().parse().expect("Invalid harmonic value"))
        .collect()
}

/// Parse comma-separated floats (e.g., "1.0,2.0,1.0")
fn parse_amps(s: &str) -> Vec<f32> {
    s.split(',')
        .map(|s| s.trim().parse().expect("Invalid amplitude value"))
        .collect()
}

/// Generate FM synthesis audio with silence padding
fn generate_fm_synth(
    params: FmSynthParams,
    mod_env: AdsrGenerator,
    wav_env: AdsrGenerator,
    silence_samples: usize,
) -> Vec<f32> {
    let mut generator = FmSynthGenerator::new(params, mod_env, wav_env);
    let frame_size = 64;
    let mut frame_buffer = vec![0.0f32; frame_size];

    let mut audio_samples: Vec<f32> = Vec::new();

    loop {
        let state = generator.process(&mut frame_buffer);
        audio_samples.extend_from_slice(&frame_buffer);

        if state == GeneratorState::Complete {
            break;
        }

        if audio_samples.len() > 1_000_000 {
            eprintln!("Warning: Generator exceeded maximum duration, stopping");
            break;
        }
    }

    let total_samples = silence_samples + audio_samples.len() + silence_samples;
    let mut output = vec![0.0f32; total_samples];
    output[silence_samples..silence_samples + audio_samples.len()].copy_from_slice(&audio_samples);

    output
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 18 {
        eprintln!("FM Synthesis Audio Generator");
        eprintln!("Usage: {} <sample_rate> <silence_secs> <harmonics> <amps> <phase> <mod_depth> <mod_attack> <mod_decay> <mod_sustain_level> <mod_sustain_time> <mod_release> <wav_attack> <wav_decay> <wav_sustain_level> <wav_sustain_time> <wav_release> <output>", args[0]);
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  sample_rate       - Sample rate in Hz (e.g., 16000, 44100)");
        eprintln!("  silence_secs      - Silence padding at start/end in seconds (e.g., 0.5)");
        eprintln!("  harmonics         - Comma-separated harmonics (e.g., '2,5,9')");
        eprintln!("  amps              - Comma-separated amplitudes (e.g., '1.0,2.0,1.0')");
        eprintln!("  phase             - Phase per sample (e.g., 0.05)");
        eprintln!("  mod_depth         - Modulation depth scaling (e.g., 0.5 = half, 1.0 = full, 2.0 = double)");
        eprintln!("  mod_attack        - Modulation envelope attack samples");
        eprintln!("  mod_decay         - Modulation envelope decay samples");
        eprintln!("  mod_sustain_level - Modulation envelope sustain level (0-1)");
        eprintln!("  mod_sustain_time  - Modulation envelope sustain time samples");
        eprintln!("  mod_release       - Modulation envelope release samples");
        eprintln!("  wav_attack        - Waveform envelope attack samples");
        eprintln!("  wav_decay         - Waveform envelope decay samples");
        eprintln!("  wav_sustain_level - Waveform envelope sustain level (0-1)");
        eprintln!("  wav_sustain_time  - Waveform envelope sustain time samples");
        eprintln!("  wav_release       - Waveform envelope release samples");
        eprintln!("  output            - Output WAV file path");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {} 16000 0.5 '2,5,9' '1.0,2.0,1.0' 0.05 1.0 100 300 0.5 8000 100 200 200 0.6 8000 100 /tmp/fm_synth.wav", args[0]);
        std::process::exit(1);
    }

    // Parse sample rate and calculate silence samples
    let sample_rate: u32 = args[1].parse().expect("Invalid sample rate");
    let silence_secs: f32 = args[2].parse().expect("Invalid silence seconds");
    let silence_samples = (silence_secs * sample_rate as f32) as usize;

    // Parse FM parameters
    let harmonics = parse_harmonics(&args[3]);
    let amps = parse_amps(&args[4]);
    let phase_per_sample: f32 = args[5].parse().expect("Invalid phase per sample");
    let mod_depth: f32 = args[6].parse().expect("Invalid modulation depth");

    // Parse modulation envelope parameters
    let mod_attack: usize = args[7].parse().expect("Invalid mod attack");
    let mod_decay: usize = args[8].parse().expect("Invalid mod decay");
    let mod_sustain_level: f32 = args[9].parse().expect("Invalid mod sustain level");
    let mod_sustain_time: usize = args[10].parse().expect("Invalid mod sustain time");
    let mod_release: usize = args[11].parse().expect("Invalid mod release");

    // Parse waveform envelope parameters
    let wav_attack: usize = args[12].parse().expect("Invalid wav attack");
    let wav_decay: usize = args[13].parse().expect("Invalid wav decay");
    let wav_sustain_level: f32 = args[14].parse().expect("Invalid wav sustain level");
    let wav_sustain_time: usize = args[15].parse().expect("Invalid wav sustain time");
    let wav_release: usize = args[16].parse().expect("Invalid wav release");

    // Output path
    let output_path = &args[17];

    println!("FM Synthesis Audio Generator");
    println!("============================");
    println!("Sample Rate: {} Hz", sample_rate);
    println!(
        "Silence Padding: {} samples ({} seconds)",
        silence_samples, silence_secs
    );
    println!();

    let params = FmSynthParams::new(harmonics, amps, phase_per_sample, mod_depth);
    let mod_env = AdsrGenerator::new(
        0.0,
        mod_attack,
        mod_decay,
        mod_sustain_level,
        mod_sustain_time,
        mod_release,
    );
    let wav_env = AdsrGenerator::new(
        0.0,
        wav_attack,
        wav_decay,
        wav_sustain_level,
        wav_sustain_time,
        wav_release,
    );

    println!("FM Parameters:");
    println!("  Harmonics: {:?}", params.harmonics);
    println!("  Amplitudes: {:?}", params.amps);
    println!("  Phase per sample: {}", params.phase_per_sample);
    println!("  Modulation depth: {}", params.mod_depth);
    println!();

    println!("Modulation Envelope:");
    println!("  Attack: {} samples", mod_attack);
    println!("  Decay: {} samples", mod_decay);
    println!("  Sustain level: {}", mod_sustain_level);
    println!("  Sustain time: {} samples", mod_sustain_time);
    println!("  Release: {} samples", mod_release);
    println!();

    println!("Waveform Envelope:");
    println!("  Attack: {} samples", wav_attack);
    println!("  Decay: {} samples", wav_decay);
    println!("  Sustain level: {}", wav_sustain_level);
    println!("  Sustain time: {} samples", wav_sustain_time);
    println!("  Release: {} samples", wav_release);
    println!();

    // Generate audio
    println!("Generating FM synthesis audio...");
    let samples = generate_fm_synth(params, mod_env, wav_env, silence_samples);
    println!("  Generated {} total samples", samples.len());
    println!(
        "  Audio duration: {} samples",
        samples.len() - 2 * silence_samples
    );
    println!();

    // Check for sharp changes
    println!("Checking for discontinuities...");
    let threshold = 0.5f32;
    let check_start = silence_samples + 500;
    let check_end = samples.len() - silence_samples - 200;
    if let Some((idx, diff)) = find_sharp_change(&samples[check_start..check_end], threshold) {
        eprintln!(
            "  WARNING: Sharp change detected at sample {}: diff={:.4}",
            check_start + idx,
            diff
        );
    } else {
        println!(
            "  No sharp changes detected in sustained audio (threshold: {})",
            threshold
        );
    }
    println!();

    // Write to WAV file
    println!("Writing to {}...", output_path);
    match write_wav_16bit(output_path, &samples, sample_rate) {
        Ok(()) => {
            println!("  Success!");
            println!();
            println!("Output: {}", output_path);
            println!("  Sample rate: {} Hz", sample_rate);
            println!("  Bit depth: 16-bit PCM");
            println!("  Channels: Mono");
            println!(
                "  Duration: {:.2} seconds",
                samples.len() as f32 / sample_rate as f32
            );
        }
        Err(e) => {
            eprintln!("  Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn find_sharp_change(samples: &[f32], threshold: f32) -> Option<(usize, f32)> {
    for i in 1..samples.len() {
        let diff = (samples[i] - samples[i - 1]).abs();
        if diff > threshold {
            // Show context around the discontinuity
            eprintln!("  Context around discontinuity:");
            let start = i.saturating_sub(5);
            let end = (i + 5).min(samples.len());
            for j in start..end {
                let marker = if j == i {
                    " <--"
                } else if j == i - 1 {
                    ""
                } else {
                    ""
                };
                eprintln!("    [{}]: {:.6}{}", j, samples[j], marker);
            }
            return Some((i, diff));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_harmonics() {
        assert_eq!(parse_harmonics("2,5,9"), vec![2usize, 5, 9]);
        assert_eq!(parse_harmonics("1"), vec![1usize]);
        assert_eq!(parse_harmonics("1, 2, 3"), vec![1usize, 2, 3]);
    }

    #[test]
    fn test_parse_amps() {
        assert_eq!(parse_amps("1.0,2.0,1.0"), vec![1.0, 2.0, 1.0]);
        assert_eq!(parse_amps("0.5"), vec![0.5]);
        assert_eq!(parse_amps("1.5, 2.5, 3.5"), vec![1.5, 2.5, 3.5]);
    }

    #[test]
    fn test_generate_fm_synth_produces_output() {
        let params = FmSynthParams::new(vec![2], vec![1.0], 0.1, 1.0);
        let mod_env = AdsrGenerator::new(0.0, 10, 30, 0.5, 100, 10);
        let wav_env = AdsrGenerator::new(0.0, 20, 20, 0.6, 100, 10);
        let silence_samples = 100;

        let samples = generate_fm_synth(params, mod_env, wav_env, silence_samples);

        assert!(
            samples.len() >= 2 * silence_samples + 50,
            "Should have at least {} samples of silence padding plus some audio",
            2 * silence_samples
        );

        for i in 0..silence_samples {
            assert_eq!(samples[i], 0.0, "Sample {} at start should be silence", i);
        }

        for i in samples.len() - silence_samples..samples.len() {
            assert_eq!(samples[i], 0.0, "Sample {} at end should be silence", i);
        }
    }

    #[test]
    fn test_generate_fm_synth_middle_has_audio() {
        let params = FmSynthParams::new(vec![1], vec![1.0], 0.1, 1.0);
        let mod_env = AdsrGenerator::new(0.0, 100, 100, 1.0, 1000, 100);
        let wav_env = AdsrGenerator::new(0.0, 100, 100, 1.0, 1000, 100);
        let silence_samples = 200;

        let samples = generate_fm_synth(params, mod_env, wav_env, silence_samples);

        let middle_start = silence_samples + 200;
        let middle_end = samples.len() - silence_samples - 200;

        let has_non_zero = samples[middle_start..middle_end]
            .iter()
            .any(|&s| s.abs() > 0.01);

        assert!(has_non_zero, "Middle section should contain non-zero audio");
    }

    #[test]
    fn test_find_sharp_change_detects_discontinuity() {
        let samples = vec![0.0f32, 0.01, 0.02, 0.5, 0.51, 0.52];
        let result = find_sharp_change(&samples, 0.1);
        assert!(result.is_some());
        let (idx, diff) = result.unwrap();
        assert_eq!(idx, 3);
        assert!((diff - 0.48).abs() < 0.01);
    }

    #[test]
    fn test_find_sharp_change_returns_none_for_smooth_signal() {
        let samples: Vec<f32> = (0..100).map(|i| (i as f32 * 0.001).sin()).collect();
        let result = find_sharp_change(&samples, 0.1);
        assert!(result.is_none());
    }
}
