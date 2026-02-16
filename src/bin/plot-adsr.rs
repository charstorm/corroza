use corroza::generator::adsr::AdsrGenerator;
use corroza::generator::{GeneratorState, SignalGenerator};
use plotters::prelude::*;

const DISCONTINUITY_THRESHOLD: f32 = 0.15;

struct Args {
    attack_samples: usize,
    decay_samples: usize,
    sustain_level: f32,
    release_samples: usize,
    note_off_sample: Option<usize>,
    frame_size: usize,
    output_path: String,
}

fn print_usage() {
    eprintln!("Usage: plot-adsr <attack_samples> <decay_samples> <sustain_level> <release_samples> <note_off_sample> <frame_size> <output.svg>");
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  attack_samples    - Attack phase duration in samples");
    eprintln!("  decay_samples     - Decay phase duration in samples");
    eprintln!("  sustain_level     - Sustain amplitude (0.0 to 1.0)");
    eprintln!("  release_samples   - Release phase duration in samples");
    eprintln!("  note_off_sample   - Sample at which to trigger note off");
    eprintln!("  frame_size        - Frame size for processing (e.g., 64)");
    eprintln!("  output.svg        - Output SVG file path");
    eprintln!();
    eprintln!("Example:");
    eprintln!("  plot-adsr 100 200 0.7 300 640 64 output.svg");
    eprintln!("    attack: 100 samples, decay: 200 samples, sustain: 0.7, release: 300 samples");
    eprintln!("    note_off at sample 640, frame_size: 64, output to output.svg");
}

fn parse_args() -> Result<Args, Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 8 {
        print_usage();
        return Err("Invalid number of arguments".into());
    }

    let attack_samples: usize = args[1].parse()?;
    let decay_samples: usize = args[2].parse()?;
    let sustain_level: f32 = args[3].parse()?;
    let release_samples: usize = args[4].parse()?;
    let note_off_sample: usize = args[5].parse()?;
    let frame_size: usize = args[6].parse()?;
    let output_path = args[7].clone();

    // Validate inputs
    if sustain_level < 0.0 || sustain_level > 1.0 {
        return Err("Sustain level must be between 0.0 and 1.0".into());
    }
    if frame_size == 0 {
        return Err("Frame size must be greater than 0".into());
    }

    Ok(Args {
        attack_samples,
        decay_samples,
        sustain_level,
        release_samples,
        note_off_sample: Some(note_off_sample),
        frame_size,
        output_path,
    })
}

fn compute_expected_duration(args: &Args) -> usize {
    // Total duration is note_off_sample + release_duration
    // Release duration is rounded up to nearest frame boundary since we process full frames
    let release_frames = (args.release_samples + args.frame_size - 1) / args.frame_size;
    let aligned_release_samples = release_frames * args.frame_size;
    args.note_off_sample.unwrap() + aligned_release_samples
}

fn generate_adsr(args: &Args) -> Result<(Vec<f32>, Vec<String>), Box<dyn std::error::Error>> {
    let mut generator = AdsrGenerator::new(
        0.0,
        args.attack_samples,
        args.decay_samples,
        args.sustain_level,
        args.note_off_sample.unwrap() + args.release_samples * 10, // max sustain well beyond note_off
        args.release_samples,
    );

    let note_off_sample = args.note_off_sample.unwrap();
    let mut samples = Vec::new();
    let mut phases = Vec::new();
    let mut frame_buffer = vec![0.0f32; args.frame_size];
    let mut total_samples = 0usize;
    let mut note_off_triggered = false;

    loop {
        // Check if note_off_sample falls within this frame
        // If so, trigger note_off before processing so Release starts at the right sample
        let frame_start = total_samples;
        let frame_end = total_samples + args.frame_size;

        if !note_off_triggered
            && note_off_sample >= frame_start
            && note_off_sample < frame_end
            && generator.phase() != corroza::generator::adsr::AdsrPhase::Release
            && generator.phase() != corroza::generator::adsr::AdsrPhase::Complete
        {
            generator.note_off();
            note_off_triggered = true;
        }

        let state = generator.process(&mut frame_buffer);
        let phase = generator.phase();
        let phase_str = format!("{:?}", phase);

        // Record phase for each sample in the frame
        for _ in 0..frame_buffer.len() {
            phases.push(phase_str.clone());
        }

        samples.extend_from_slice(&frame_buffer);
        total_samples += frame_buffer.len();

        if state == GeneratorState::Complete {
            break;
        }

        // Safety: prevent infinite loops
        if total_samples > 100000 {
            return Err("Envelope exceeded maximum duration".into());
        }
    }

    Ok((samples, phases))
}

fn check_discontinuities(samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
    let mut max_diff: f32 = 0.0;
    let mut max_diff_idx: usize = 0;

    for i in 1..samples.len() {
        let diff = (samples[i] - samples[i - 1]).abs();
        if diff > max_diff {
            max_diff = diff;
            max_diff_idx = i;
        }
        if diff > DISCONTINUITY_THRESHOLD {
            return Err(format!(
                "DISCONTINUITY at sample {}: {} -> {} (diff = {})",
                max_diff_idx,
                samples[max_diff_idx - 1],
                samples[max_diff_idx],
                max_diff
            )
            .into());
        }
    }

    println!(
        "  ✓ Max discontinuity: {:.6} at sample {} (below threshold {})",
        max_diff, max_diff_idx, DISCONTINUITY_THRESHOLD
    );
    Ok(())
}

fn create_plot(
    args: &Args,
    samples: &[f32],
    phases: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let root = SVGBackend::new(&args.output_path, (800, 400)).into_drawing_area();
    root.fill(&WHITE)?;

    let sample_indices: Vec<f32> = samples.iter().enumerate().map(|(i, _)| i as f32).collect();
    let max_sample = *sample_indices.last().unwrap_or(&0.0);

    // Compute title with parameters
    let title = format!(
        "ADSR: A={}, D={}, S={:.2}, R={}, note_off={}",
        args.attack_samples,
        args.decay_samples,
        args.sustain_level,
        args.release_samples,
        args.note_off_sample.unwrap()
    );

    let mut chart = ChartBuilder::on(&root)
        .caption(&title, ("sans-serif", 20))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0f32..max_sample, 0f32..1.1f32)?;

    chart
        .configure_mesh()
        .x_desc("Sample")
        .y_desc("Amplitude")
        .x_labels(10)
        .y_labels(10)
        .draw()?;

    // Draw the envelope
    chart.draw_series(LineSeries::new(
        sample_indices
            .iter()
            .zip(samples.iter())
            .map(|(&t, &s)| (t, s)),
        BLUE.stroke_width(2),
    ))?;

    // Draw phase transitions as vertical lines
    let mut current_phase = "";
    for (i, phase) in phases.iter().enumerate() {
        if phase != current_phase && !current_phase.is_empty() {
            chart.draw_series(std::iter::once(plotters::element::Cross::new(
                (i as f32, 0.5),
                8,
                BLACK.filled(),
            )))?;
        }
        current_phase = phase;
    }

    // Draw note_off marker
    let note_off_sample = args.note_off_sample.unwrap() as f32;
    chart.draw_series(std::iter::once(plotters::element::Circle::new(
        (note_off_sample, args.sustain_level),
        5,
        RED.filled(),
    )))?;

    root.present()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;

    println!("ADSR Plot Generator");
    println!("==================");
    println!("  Attack: {} samples", args.attack_samples);
    println!("  Decay: {} samples", args.decay_samples);
    println!("  Sustain: {:.2}", args.sustain_level);
    println!("  Release: {} samples", args.release_samples);
    println!("  Note Off: sample {}", args.note_off_sample.unwrap());
    println!("  Frame Size: {} samples", args.frame_size);
    println!();

    // Pre-compute expected duration
    let expected_samples = compute_expected_duration(&args);
    println!("  Expected duration: {} samples", expected_samples);

    // Generate ADSR
    print!("  Generating envelope... ");
    let (samples, phases) = generate_adsr(&args)?;
    let actual_samples = samples.len();
    println!("done ({} samples)", actual_samples);

    // Check duration
    let duration_diff = (actual_samples as isize - expected_samples as isize).abs();
    if duration_diff > 100 {
        // Allow 100 sample tolerance
        return Err(format!(
            "Duration mismatch: expected {} samples but got {} samples (diff={})",
            expected_samples, actual_samples, duration_diff
        )
        .into());
    }
    println!("  ✓ Duration matches expected (within tolerance)");

    // Check for discontinuities
    check_discontinuities(&samples)?;

    // Generate plot
    print!("  Creating plot... ");
    create_plot(&args, &samples, &phases)?;
    println!("done");

    println!();
    println!("Output: {}", args.output_path);

    Ok(())
}
