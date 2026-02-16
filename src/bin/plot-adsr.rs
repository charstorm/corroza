use corroza::generator::adsr::AdsrGenerator;
use corroza::generator::{GeneratorState, SignalGenerator};
use plotters::prelude::*;

const SAMPLE_RATE: f32 = 1000.0; // 1ms = 1 sample
const FRAME_SIZE: usize = 64;
const DISCONTINUITY_THRESHOLD: f32 = 0.15;

struct Args {
    attack_ms: f32,
    decay_ms: f32,
    sustain_level: f32,
    release_ms: f32,
    note_off_ms: Option<f32>,
    output_path: String,
}

fn print_usage() {
    eprintln!("Usage: plot-adsr <attack_ms> <decay_ms> <sustain_level> <release_ms> <note_off_ms> <output.svg>");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  plot-adsr 100 200 0.7 300 640 output.svg  # note_off at 640ms");
    eprintln!(
        "  plot-adsr 50 100 0.5 200 50 output.svg    # early release at 50ms (during attack)"
    );
}

fn parse_args() -> Result<Args, Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 7 {
        print_usage();
        return Err("Invalid number of arguments".into());
    }

    let attack_ms: f32 = args[1].parse()?;
    let decay_ms: f32 = args[2].parse()?;
    let sustain_level: f32 = args[3].parse()?;
    let release_ms: f32 = args[4].parse()?;
    let note_off_ms: f32 = args[5].parse()?;
    let output_path = args[6].clone();

    // Validate inputs
    if attack_ms < 0.0 || decay_ms < 0.0 || release_ms < 0.0 {
        return Err("Time values must be non-negative".into());
    }
    if sustain_level < 0.0 || sustain_level > 1.0 {
        return Err("Sustain level must be between 0.0 and 1.0".into());
    }
    if note_off_ms < 0.0 {
        return Err("Note off time must be non-negative".into());
    }

    Ok(Args {
        attack_ms,
        decay_ms,
        sustain_level,
        release_ms,
        note_off_ms: Some(note_off_ms),
        output_path,
    })
}

fn compute_expected_duration(args: &Args) -> f32 {
    // Total duration is always note_off_time + release_time
    // since release starts at note_off
    args.note_off_ms.unwrap() + args.release_ms
}

fn generate_adsr(args: &Args) -> Result<(Vec<f32>, Vec<String>), Box<dyn std::error::Error>> {
    let mut generator = AdsrGenerator::new(
        0.0,
        args.attack_ms,
        args.decay_ms,
        args.sustain_level,
        10000.0, // max sustain (we control via note_off)
        args.release_ms,
        SAMPLE_RATE,
    );

    let note_off_sample = (args.note_off_ms.unwrap() * SAMPLE_RATE / 1000.0) as usize;
    let mut samples = Vec::new();
    let mut phases = Vec::new();
    let mut frame_buffer = vec![0.0f32; FRAME_SIZE];
    let mut total_samples = 0usize;
    let mut note_off_triggered = false;

    loop {
        let state = generator.process(&mut frame_buffer);
        let phase = generator.phase();
        let phase_str = format!("{:?}", phase);

        // Process each sample in the frame, checking for note_off at exact sample position
        for i in 0..frame_buffer.len() {
            let sample_idx = total_samples + i;

            // Check if we should trigger note_off at this exact sample
            if !note_off_triggered
                && sample_idx >= note_off_sample
                && phase != corroza::generator::adsr::AdsrPhase::Release
                && phase != corroza::generator::adsr::AdsrPhase::Complete
            {
                generator.note_off();
                note_off_triggered = true;
            }

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
                "DISCONTINUITY at sample {} ({}ms): {} -> {} (diff = {})",
                max_diff_idx,
                max_diff_idx as f32 / SAMPLE_RATE * 1000.0,
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

    let time_ms: Vec<f32> = samples.iter().enumerate().map(|(i, _)| i as f32).collect();
    let max_time = *time_ms.last().unwrap_or(&0.0);

    // Compute title with parameters
    let title = format!(
        "ADSR: A={}ms, D={}ms, S={:.2}, R={}ms, note_off={}ms",
        args.attack_ms,
        args.decay_ms,
        args.sustain_level,
        args.release_ms,
        args.note_off_ms.unwrap()
    );

    let mut chart = ChartBuilder::on(&root)
        .caption(&title, ("sans-serif", 20))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(0f32..max_time, 0f32..1.1f32)?;

    chart
        .configure_mesh()
        .x_desc("Time (ms)")
        .y_desc("Amplitude")
        .x_labels(10)
        .y_labels(10)
        .draw()?;

    // Draw the envelope
    chart.draw_series(LineSeries::new(
        time_ms.iter().zip(samples.iter()).map(|(&t, &s)| (t, s)),
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
    let note_off_ms = args.note_off_ms.unwrap();
    chart.draw_series(std::iter::once(plotters::element::Circle::new(
        (note_off_ms, args.sustain_level),
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
    println!("  Attack: {}ms", args.attack_ms);
    println!("  Decay: {}ms", args.decay_ms);
    println!("  Sustain: {:.2}", args.sustain_level);
    println!("  Release: {}ms", args.release_ms);
    println!("  Note Off: {}ms", args.note_off_ms.unwrap());
    println!();

    // Pre-compute expected duration
    let expected_dur = compute_expected_duration(&args);
    let expected_samples = (expected_dur * SAMPLE_RATE / 1000.0) as usize;
    println!(
        "  Expected duration: {:.1}ms ({} samples)",
        expected_dur, expected_samples
    );

    // Generate ADSR
    print!("  Generating envelope... ");
    let (samples, phases) = generate_adsr(&args)?;
    let actual_dur = samples.len() as f32 / SAMPLE_RATE * 1000.0;
    println!("done ({} samples, {:.1}ms)", samples.len(), actual_dur);

    // Check duration
    let duration_diff = (actual_dur - expected_dur).abs();
    if duration_diff > 100.0 {
        // Allow 100ms tolerance
        return Err(format!(
            "Duration mismatch: expected {:.1}ms but got {:.1}ms (diff={:.1}ms)",
            expected_dur, actual_dur, duration_diff
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
