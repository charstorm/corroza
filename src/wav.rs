//! WAV file writer utility
//!
//! Provides simple WAV file writing for 16-bit PCM audio.
//! Note: Sample rate is only used for the file header, not for any processing.

use std::fs::File;
use std::io::{self, Write};

/// Write a 16-bit PCM WAV file
///
/// # Arguments
/// * `path` - Output file path
/// * `samples` - Audio samples (f32, range [-1.0, 1.0])
/// * `sample_rate` - Sample rate in Hz (only for header)
///
/// # Returns
/// Result indicating success or IO error
///
/// # Example
/// ```
/// use corroza::wav::write_wav_16bit;
///
/// let samples = vec![0.0f32; 16000]; // 1 second of silence at 16kHz
/// write_wav_16bit("/tmp/output.wav", &samples, 16000).unwrap();
/// ```
pub fn write_wav_16bit(path: &str, samples: &[f32], sample_rate: u32) -> io::Result<()> {
    let mut file = File::create(path)?;

    // Convert f32 samples to i16
    let i16_samples: Vec<i16> = samples
        .iter()
        .map(|&s| {
            // Clamp to [-1.0, 1.0] then convert to i16
            let clamped = s.clamp(-1.0, 1.0);
            if clamped >= 0.0 {
                (clamped * i16::MAX as f32) as i16
            } else {
                // For negative values, use i16::MIN to ensure -1.0 maps to -32768
                (clamped * (i16::MIN as i32).abs() as f32) as i16
            }
        })
        .collect();

    let num_channels: u16 = 1; // Mono
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * num_channels as u32 * (bits_per_sample / 8) as u32;
    let block_align = num_channels * (bits_per_sample / 8);
    let data_size = i16_samples.len() as u32 * 2; // 2 bytes per i16 sample
    let file_size = 36 + data_size; // 44 - 8 (header excluding RIFF and size)

    // RIFF chunk
    file.write_all(b"RIFF")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt subchunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // Subchunk size
    file.write_all(&1u16.to_le_bytes())?; // Audio format (PCM)
    file.write_all(&num_channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;

    // data subchunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;

    // Write sample data
    for sample in i16_samples {
        file.write_all(&sample.to_le_bytes())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_write_wav_silence() {
        let temp_path = "/tmp/test_silence.wav";
        let samples = vec![0.0f32; 100];
        write_wav_16bit(temp_path, &samples, 16000).unwrap();

        // Verify file exists and has reasonable size
        let metadata = fs::metadata(temp_path).unwrap();
        assert!(metadata.len() > 44); // Minimum WAV header size

        // Clean up
        fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn test_write_wav_full_scale() {
        let temp_path = "/tmp/test_full_scale.wav";
        let samples = vec![1.0f32, -1.0f32, 0.5f32, -0.5f32, 0.0f32];
        write_wav_16bit(temp_path, &samples, 16000).unwrap();

        // Read back and verify header
        let data = fs::read(temp_path).unwrap();

        // Check RIFF header
        assert_eq!(&data[0..4], b"RIFF");
        assert_eq!(&data[8..12], b"WAVE");

        // Check fmt chunk
        assert_eq!(&data[12..16], b"fmt ");
        assert_eq!(u16::from_le_bytes([data[20], data[21]]), 1); // PCM format
        assert_eq!(u16::from_le_bytes([data[22], data[23]]), 1); // Mono
        assert_eq!(
            u32::from_le_bytes([data[24], data[25], data[26], data[27]]),
            16000
        ); // Sample rate

        // Clean up
        fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn test_write_wav_clamping() {
        let temp_path = "/tmp/test_clamping.wav";
        // Values outside [-1.0, 1.0] should be clamped
        let samples = vec![2.0f32, -2.0f32, 1.5f32, -1.5f32];
        write_wav_16bit(temp_path, &samples, 16000).unwrap();

        // Read back and check sample values
        let data = fs::read(temp_path).unwrap();

        // First sample should be clamped to i16::MAX
        let first_sample = i16::from_le_bytes([data[44], data[45]]);
        assert_eq!(first_sample, i16::MAX);

        // Second sample should be clamped to i16::MIN
        let second_sample = i16::from_le_bytes([data[46], data[47]]);
        assert_eq!(second_sample, i16::MIN);

        // Clean up
        fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn test_write_wav_correct_size() {
        let temp_path = "/tmp/test_size.wav";
        let num_samples = 1000;
        let samples = vec![0.0f32; num_samples];
        write_wav_16bit(temp_path, &samples, 16000).unwrap();

        let data = fs::read(temp_path).unwrap();

        // Check data chunk size
        let data_chunk_size = u32::from_le_bytes([data[40], data[41], data[42], data[43]]);
        assert_eq!(data_chunk_size, (num_samples * 2) as u32);

        // Check total file size
        let riff_chunk_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        assert_eq!(riff_chunk_size, 36 + data_chunk_size);

        // Clean up
        fs::remove_file(temp_path).unwrap();
    }
}
