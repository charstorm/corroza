use corroza::generator::{GeneratorState, RampGenerator, SignalGenerator};

fn main() {
    println!("Corroza Audio Synthesis Library - Basic Demo");
    println!("=============================================\n");

    // Create a 100ms ramp at 44.1kHz
    let sample_rate = 44100.0;
    let duration_ms = 100.0;
    let frame_size = 64; // Process in 64-sample frames

    let mut ramp = RampGenerator::new(duration_ms, sample_rate);
    let total_samples = ramp.duration();

    println!("Ramp Generator Demo:");
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

    println!("\n✓ Demo complete!");
}
