use corroza::generator::adsr::AdsrGenerator;
use corroza::generator::{GeneratorState, RampGenerator, SignalGenerator};

fn demo_ramp() {
    println!("\n=== Ramp Generator Demo ===\n");

    // Create a 100ms ramp at 44.1kHz
    let sample_rate = 44100.0;
    let duration_ms = 100.0;
    let frame_size = 64; // Process in 64-sample frames

    let mut ramp = RampGenerator::new(duration_ms, sample_rate);
    let total_samples = ramp.duration();

    println!("Configuration:");
    println!("  Sample rate: {} Hz", sample_rate);
    println!("  Duration: {} ms", duration_ms);
    println!("  Total samples: {}", total_samples);
    println!("  Frame size: {} samples", frame_size);
    println!();

    // Process frame by frame and show progress
    let mut frame_buffer = vec![0.0f32; frame_size];
    let mut frame_count = 0;

    loop {
        let state = ramp.process(&mut frame_buffer);
        frame_count += 1;

        // Print first few samples of each frame
        let start_sample = (frame_count - 1) * frame_size;
        if frame_count <= 3 || state == GeneratorState::Complete {
            print!(
                "Frame {} (samples {}-{}): [",
                frame_count,
                start_sample,
                (start_sample + frame_size).min(total_samples)
            );

            // Show up to 5 samples from the frame
            let samples_to_show = frame_buffer.len().min(5);
            for (i, &sample) in frame_buffer.iter().take(samples_to_show).enumerate() {
                if i > 0 {
                    print!(", ");
                }
                print!("{:.3}", sample);
            }
            if frame_buffer.len() > 5 {
                print!(", ...");
            }
            println!("] {:?}", state);
        } else if frame_count == 4 {
            println!(
                "  ... (processing {} more frames) ...",
                (total_samples / frame_size) - 6
            );
        }

        if state == GeneratorState::Complete {
            break;
        }
    }

    println!("\nCompleted in {} frames", frame_count);

    // Demonstrate reset capability
    println!("\n--- Resetting generator ---\n");
    ramp.reset();

    // Process one frame to show it started over
    ramp.process(&mut frame_buffer[..10.min(frame_size)]);
    print!("After reset - first 10 samples: [");
    for (i, &sample) in frame_buffer[..10].iter().enumerate() {
        if i > 0 {
            print!(", ");
        }
        print!("{:.3}", sample);
    }
    println!("]");
}

fn demo_adsr() {
    println!("\n=== ADSR Envelope Generator Demo ===\n");

    let sample_rate = 44100.0;
    let frame_size = 64;

    // Create ADSR: 100ms attack, 200ms decay, 70% sustain, 2s max sustain, 300ms release
    let mut adsr = AdsrGenerator::new(
        0.0,    // initial amplitude
        100.0,  // attack: 100ms
        200.0,  // decay: 200ms
        0.7,    // sustain: 70%
        2000.0, // max sustain: 2 seconds
        300.0,  // release: 300ms
        sample_rate,
    );

    println!("Configuration:");
    println!("  Attack: 100ms");
    println!("  Decay: 200ms");
    println!("  Sustain: 70%");
    println!("  Max Sustain: 2 seconds");
    println!("  Release: 300ms");
    println!("  Sample Rate: {} Hz", sample_rate);
    println!("  Frame Size: {} samples", frame_size);
    println!();

    let mut frame_buffer = vec![0.0f32; frame_size];
    let mut frame_count = 0;
    let mut last_phase = adsr.phase();

    println!("Envelope Progress:");
    println!(
        "{:<6} {:<12} {:<12} {:<12}",
        "Frame", "Phase", "Amp Start", "Amp End"
    );
    println!("{}", "-".repeat(50));

    // Process through the envelope with manual note_off trigger
    loop {
        let state = adsr.process(&mut frame_buffer);
        frame_count += 1;

        let current_phase = adsr.phase();
        let amp_start = frame_buffer[0];
        let amp_end = frame_buffer[frame_size - 1];

        // Print when phase changes or at interesting points
        if current_phase != last_phase
            || frame_count <= 3
            || (frame_count % 50 == 0 && state != GeneratorState::Complete)
        {
            let phase_str = format!("{:?}", last_phase);
            println!(
                "{:<6} {:<12} {:<12.3} {:<12.3}",
                frame_count, phase_str, amp_start, amp_end
            );
            last_phase = current_phase;
        }

        // Trigger note_off after ~50 frames (sustain phase)
        use corroza::generator::adsr::AdsrPhase;
        if frame_count == 100 && adsr.phase() == AdsrPhase::Sustain {
            println!("\n  [Triggering note_off at frame {}]", frame_count);
            adsr.note_off();
        }

        if state == GeneratorState::Complete {
            println!(
                "{:<6} {:<12} {:<12.3} {:<12.3}",
                frame_count, "Complete", amp_start, 0.0
            );
            break;
        }
    }

    println!("\nCompleted in {} frames", frame_count);
    println!("Final amplitude: {:.3}", adsr.current_amplitude());

    // Demo early release
    println!("\n--- Early Release Demo ---\n");

    let mut adsr2 = AdsrGenerator::new(
        0.0,
        500.0,
        500.0,
        0.5,
        2000.0,
        300.0, // Longer attack/decay
        sample_rate,
    );

    println!("Triggering note_off during Attack phase:");

    // Process just a few frames (still in attack)
    for i in 0..3 {
        adsr2.process(&mut frame_buffer);
        println!(
            "  Frame {}: phase={:?}, amp={:.3}",
            i + 1,
            adsr2.phase(),
            adsr2.current_amplitude()
        );
    }

    println!("\n  [Triggering note_off]");
    adsr2.note_off();

    // Next frame shows transition to release
    adsr2.process(&mut frame_buffer);
    println!(
        "  Frame 4: phase={:?}, amp={:.3} (started from {:.3})",
        adsr2.phase(),
        frame_buffer[0],
        adsr2.current_amplitude()
    );

    println!("\n✓ ADSR demo complete!");
}

fn main() {
    println!("Corroza Audio Synthesis Library");
    println!("===============================");

    demo_ramp();
    demo_adsr();

    println!("\n===============================");
    println!("All demos complete!");
}
