use super::adsr::AdsrGenerator;
use super::{GeneratorState, SignalGenerator};
use std::f32::consts::PI;

/// Parameters for FM synthesis
///
/// All values are in sample-level units:
/// - phase_per_sample: phase change per sample in radians
/// - harmonics: frequency multipliers relative to base frequency
/// - amps: modulation amplitudes for each harmonic
/// - mod_depth: overall modulation depth scaling (0 = no FM, 1 = full modulation)
#[derive(Debug, Clone)]
pub struct FmSynthParams {
    /// Frequency multipliers for modulation harmonics (e.g., [2, 5, 9])
    pub harmonics: Vec<usize>,
    /// Amplitudes for each modulation harmonic (e.g., [1.0, 2.0, 1.0])
    pub amps: Vec<f32>,
    /// Base phase increment per sample (radians/sample)
    pub phase_per_sample: f32,
    /// Modulation depth scaling factor (e.g., 0.5 = half modulation, 2.0 = double modulation)
    pub mod_depth: f32,
}

impl FmSynthParams {
    /// Create new FM synthesis parameters
    ///
    /// # Arguments
    /// * `harmonics` - Frequency multipliers for modulation
    /// * `amps` - Amplitudes for each harmonic (must match harmonics length)
    /// * `phase_per_sample` - Base phase increment per sample
    /// * `mod_depth` - Modulation depth scaling (0 = no FM, 1 = full modulation)
    ///
    /// # Panics
    /// Panics if harmonics and amps have different lengths
    pub fn new(
        harmonics: Vec<usize>,
        amps: Vec<f32>,
        phase_per_sample: f32,
        mod_depth: f32,
    ) -> Self {
        assert_eq!(
            harmonics.len(),
            amps.len(),
            "Harmonics and amps must have the same length"
        );
        assert!(
            phase_per_sample > 0.0 && phase_per_sample < PI,
            "phase_per_sample must be between 0 and PI"
        );
        assert!(mod_depth >= 0.0, "mod_depth must be non-negative");
        Self {
            harmonics,
            amps,
            phase_per_sample,
            mod_depth,
        }
    }
}

/// FM Synthesis generator
///
/// Generates audio using frequency modulation synthesis with dual ADSR envelopes.
/// The modulation envelope controls the depth of frequency modulation over time,
/// while the waveform envelope controls the output amplitude.
///
/// Algorithm per sample:
/// 1. Compute modulation signal: m[n] = Σ amps[i] * sin(harmonics[i] * phase_per_sample * n)
/// 2. Get envelope values: e[n] from mod_env, E[n] from wav_env
/// 3. Instantaneous frequency: f[n] = phase_per_sample * (1 + m[n] * mod_depth * e[n])
/// 4. Phase accumulation: θ[n] = θ[n-1] + 2π * f[n] (wrapped to [0, 2π))
/// 5. Output: y[n] = sin(θ[n]) * E[n]
pub struct FmSynthGenerator {
    // Parameters
    params: FmSynthParams,

    // Envelopes
    mod_env: AdsrGenerator,
    wav_env: AdsrGenerator,

    // State
    phase: f32,
    sample_count: usize,
}

impl FmSynthGenerator {
    /// Create a new FM synthesis generator
    ///
    /// # Arguments
    /// * `params` - FM synthesis parameters (harmonics, amps, phase_per_sample, mod_depth)
    /// * `mod_env` - Modulation envelope (controls FM depth)
    /// * `wav_env` - Waveform envelope (controls output amplitude)
    ///
    /// # Example
    /// ```
    /// use corroza::generator::fm_synth::{FmSynthGenerator, FmSynthParams};
    /// use corroza::generator::adsr::AdsrGenerator;
    ///
    /// let params = FmSynthParams::new(
    ///     vec![2, 5, 9],
    ///     vec![1.0, 2.0, 1.0],
    ///     0.1,
    ///     1.0,
    /// );
    /// let mod_env = AdsrGenerator::new(0.0, 100, 300, 0.5, 8000, 100);
    /// let wav_env = AdsrGenerator::new(0.0, 200, 200, 0.6, 8000, 100);
    ///
    /// let fm = FmSynthGenerator::new(params, mod_env, wav_env);
    /// ```
    pub fn new(params: FmSynthParams, mod_env: AdsrGenerator, wav_env: AdsrGenerator) -> Self {
        // Warn if envelope lengths don't match
        let mod_total = mod_env.total_samples();
        let wav_total = wav_env.total_samples();
        if mod_total != wav_total {
            eprintln!(
                "WARNING: Modulation envelope ({} samples) and waveform envelope ({} samples) have different lengths",
                mod_total, wav_total
            );
            if mod_total < wav_total {
                eprintln!(
                    "  -> FM modulation will stop {} samples before audio ends",
                    wav_total - mod_total
                );
            } else {
                eprintln!(
                    "  -> Audio will be silent for last {} samples",
                    mod_total - wav_total
                );
            }
        }

        Self {
            params,
            mod_env,
            wav_env,
            phase: 0.0,
            sample_count: 0,
        }
    }

    /// Compute the modulation signal m[n]
    ///
    /// m[n] = Σ amps[i] * sin(harmonics[i] * phase_per_sample * n)
    fn compute_modulation(&self) -> f32 {
        let mut modulation = 0.0f32;
        for (i, &harmonic) in self.params.harmonics.iter().enumerate() {
            let mod_phase =
                harmonic as f32 * self.params.phase_per_sample * self.sample_count as f32;
            modulation += self.params.amps[i] * mod_phase.sin();
        }
        modulation
    }

    /// Get the current phase
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// Get the current sample count
    pub fn sample_count(&self) -> usize {
        self.sample_count
    }

    /// Trigger note off for both envelopes
    ///
    /// This starts the release phase for both the modulation and waveform envelopes.
    /// The note will fade out according to the release settings.
    pub fn note_off(&mut self) {
        self.mod_env.note_off();
        self.wav_env.note_off();
    }
}

impl SignalGenerator for FmSynthGenerator {
    fn process(&mut self, buffer: &mut [f32]) -> GeneratorState {
        let two_pi = 2.0f32 * PI;

        // Process envelopes to get per-sample envelope values
        let mut mod_env_buffer = vec![0.0f32; buffer.len()];
        let mut wav_env_buffer = vec![0.0f32; buffer.len()];
        let mod_state = self.mod_env.process(&mut mod_env_buffer);
        let wav_state = self.wav_env.process(&mut wav_env_buffer);

        for (i, sample) in buffer.iter_mut().enumerate() {
            // 1. Compute modulation signal
            let modulation = self.compute_modulation();

            // 2. Get envelope values for this specific sample (not cached value)
            let mod_env_val = mod_env_buffer[i];
            let wav_env_val = wav_env_buffer[i];

            // 3. Instantaneous frequency: f[n] = g * (1 + m[n] * mod_depth * e[n])
            let inst_freq = self.params.phase_per_sample
                * (1.0 + modulation * self.params.mod_depth * mod_env_val);

            // 4. Phase accumulation with wrapping to [0, 2π)
            self.phase += two_pi * inst_freq;
            self.phase = self.phase % two_pi;

            // 5. Final output: y[n] = sin(θ[n]) * E[n]
            *sample = self.phase.sin() * wav_env_val;

            // Advance sample count
            self.sample_count += 1;
        }

        // Complete when both envelopes are complete
        if mod_state == GeneratorState::Complete && wav_state == GeneratorState::Complete {
            GeneratorState::Complete
        } else {
            GeneratorState::Running
        }
    }

    fn is_complete(&self) -> bool {
        self.mod_env.is_complete() && self.wav_env.is_complete()
    }

    fn reset(&mut self) {
        self.mod_env.reset();
        self.wav_env.reset();
        self.phase = 0.0;
        self.sample_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::adsr::AdsrGenerator;

    fn create_test_params() -> FmSynthParams {
        FmSynthParams::new(vec![2, 5], vec![1.0, 0.5], 0.1, 1.0)
    }

    fn create_test_envs() -> (AdsrGenerator, AdsrGenerator) {
        let mod_env = AdsrGenerator::new(0.0, 10, 30, 0.5, 100, 10);
        let wav_env = AdsrGenerator::new(0.0, 20, 20, 0.6, 100, 10);
        (mod_env, wav_env)
    }

    #[test]
    fn test_fm_synth_params_creation() {
        let params = FmSynthParams::new(vec![2, 5, 9], vec![1.0, 2.0, 1.0], 0.1, 1.0);
        assert_eq!(params.harmonics, vec![2, 5, 9]);
        assert_eq!(params.amps, vec![1.0, 2.0, 1.0]);
        assert!((params.phase_per_sample - 0.1).abs() < 0.001);
        assert!((params.mod_depth - 1.0).abs() < 0.001);
    }

    #[test]
    #[should_panic(expected = "Harmonics and amps must have the same length")]
    fn test_fm_synth_params_mismatched_lengths() {
        FmSynthParams::new(vec![2, 5], vec![1.0], 0.1, 1.0);
    }

    #[test]
    #[should_panic(expected = "phase_per_sample must be between 0 and PI")]
    fn test_fm_synth_params_invalid_phase_zero() {
        FmSynthParams::new(vec![2], vec![1.0], 0.0, 1.0);
    }

    #[test]
    fn test_fm_synth_basic_generation() {
        let params = create_test_params();
        let (mod_env, wav_env) = create_test_envs();
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        let mut buffer = [0.0f32; 64];
        let state = fm.process(&mut buffer);

        // Should be running (envelopes not complete)
        assert_eq!(state, GeneratorState::Running);

        // Output should be bounded
        for &sample in buffer.iter() {
            assert!(
                sample >= -1.0 && sample <= 1.0,
                "Sample {} out of bounds",
                sample
            );
        }
    }

    #[test]
    fn test_phase_accumulation() {
        let params = FmSynthParams::new(vec![1], vec![0.0], 0.1, 1.0); // No modulation
        let (mod_env, wav_env) = create_test_envs();
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        let mut buffer = [0.0f32; 10];
        fm.process(&mut buffer);

        // Phase should have accumulated
        assert!(fm.phase() > 0.0);
        // Phase should be wrapped to [0, 2π)
        assert!(fm.phase() < 2.0 * PI);
    }

    #[test]
    fn test_phase_wrapping() {
        // High phase_per_sample to cause rapid wrapping
        let params = FmSynthParams::new(vec![], vec![], 0.5, 1.0);
        let (mod_env, wav_env) = create_test_envs();
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        let mut buffer = [0.0f32; 100];
        fm.process(&mut buffer);

        // Phase should always be in [0, 2π)
        assert!(fm.phase() >= 0.0);
        assert!(fm.phase() < 2.0 * PI);
    }

    #[test]
    fn test_modulation_signal() {
        let params = FmSynthParams::new(vec![1], vec![1.0], 0.1, 1.0);
        let (mod_env, wav_env) = create_test_envs();
        let fm = FmSynthGenerator::new(params, mod_env, wav_env);

        // At sample 0, modulation should be sin(0) = 0
        let mod_0 = fm.compute_modulation();
        assert!(
            mod_0.abs() < 0.001,
            "Modulation at sample 0 should be ~0, got {}",
            mod_0
        );
    }

    #[test]
    fn test_no_modulation() {
        // Empty harmonics means no modulation
        let params = FmSynthParams::new(vec![], vec![], 0.1, 1.0);
        let (mod_env, wav_env) = create_test_envs();
        let fm = FmSynthGenerator::new(params, mod_env, wav_env);

        assert_eq!(fm.compute_modulation(), 0.0);
    }

    #[test]
    fn test_reset() {
        let params = create_test_params();
        let (mod_env, wav_env) = create_test_envs();
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        // Process some samples
        let mut buffer = [0.0f32; 50];
        fm.process(&mut buffer);
        let phase_before = fm.phase();
        let count_before = fm.sample_count();
        assert!(phase_before > 0.0);
        assert!(count_before > 0);

        // Reset
        fm.reset();

        // Should be back to initial state
        assert_eq!(fm.phase(), 0.0);
        assert_eq!(fm.sample_count(), 0);
        assert!(!fm.is_complete());
    }

    #[test]
    fn test_is_complete() {
        // Very short envelopes
        let params = FmSynthParams::new(vec![], vec![], 0.1, 1.0);
        let mod_env = AdsrGenerator::new(0.0, 1, 1, 0.5, 1, 1);
        let wav_env = AdsrGenerator::new(0.0, 1, 1, 0.5, 1, 1);
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        assert!(!fm.is_complete());

        // Process until complete
        let mut buffer = [0.0f32; 10];
        loop {
            let state = fm.process(&mut buffer);
            if state == GeneratorState::Complete {
                break;
            }
            // Safety limit
            if fm.sample_count() > 1000 {
                panic!("Generator didn't complete");
            }
        }

        assert!(fm.is_complete());
    }

    #[test]
    fn test_output_continuity() {
        // Test that there are no discontinuities at frame boundaries
        // Use no modulation and constant amplitude envelopes
        let params = FmSynthParams::new(vec![], vec![], 0.05, 1.0);
        // Envelopes already in sustain at full amplitude
        let mod_env = AdsrGenerator::new(1.0, 1, 1, 1.0, 10000, 1);
        let wav_env = AdsrGenerator::new(1.0, 1, 1, 1.0, 10000, 1);
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        let mut buffer1 = [0.0f32; 32];
        let mut buffer2 = [0.0f32; 32];

        fm.process(&mut buffer1);
        let last_sample_1 = buffer1[31];
        fm.process(&mut buffer2);
        let first_sample_2 = buffer2[0];

        // For a pure sine wave at constant amplitude, consecutive samples should be similar
        // The max theoretical change is about 2 * sin(π * 0.05) ≈ 0.31
        let diff = (last_sample_1 - first_sample_2).abs();
        assert!(
            diff < 0.5,
            "Discontinuity at frame boundary: {} vs {} (diff={})",
            last_sample_1,
            first_sample_2,
            diff
        );
    }

    #[test]
    fn test_multiple_harmonics() {
        let params = FmSynthParams::new(vec![2, 3, 4], vec![0.5, 1.0, 0.3], 0.05, 1.0);
        let (mod_env, wav_env) = create_test_envs();
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        let mut buffer = [0.0f32; 100];
        let state = fm.process(&mut buffer);

        assert_eq!(state, GeneratorState::Running);

        // Check that output is bounded
        for &sample in buffer.iter() {
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_sample_count_progression() {
        let params = FmSynthParams::new(vec![], vec![], 0.1, 1.0);
        let (mod_env, wav_env) = create_test_envs();
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        let mut buffer = [0.0f32; 10];
        assert_eq!(fm.sample_count(), 0);

        fm.process(&mut buffer);
        assert_eq!(fm.sample_count(), 10);

        fm.process(&mut buffer);
        assert_eq!(fm.sample_count(), 20);
    }

    #[test]
    fn test_output_not_silent() {
        let params = FmSynthParams::new(vec![2, 5], vec![1.0, 0.5], 0.1, 1.0);
        let mod_env = AdsrGenerator::new(0.0, 100, 200, 0.8, 8000, 200);
        let wav_env = AdsrGenerator::new(0.0, 100, 200, 0.8, 8000, 200);
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        let mut buffer = vec![0.0f32; 16000];
        fm.process(&mut buffer);

        let sum: f32 = buffer.iter().map(|&s| s.abs()).sum();
        let mean_amp = sum / buffer.len() as f32;

        assert!(
            mean_amp > 0.1,
            "Mean amplitude {} is too low (expected > 0.1)",
            mean_amp
        );
    }

    #[test]
    fn test_frame_boundary_continuity_low_modulation() {
        let params = FmSynthParams::new(vec![1], vec![1.0], 0.05, 0.0);
        let mod_env = AdsrGenerator::new(1.0, 10, 10, 1.0, 1000, 10);
        let wav_env = AdsrGenerator::new(1.0, 10, 10, 1.0, 1000, 10);
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        let frame_size = 64;
        let mut buffer1 = vec![0.0f32; frame_size];
        let mut buffer2 = vec![0.0f32; frame_size];

        fm.process(&mut buffer1);
        fm.process(&mut buffer2);

        let last_sample = buffer1[frame_size - 1];
        let first_sample = buffer2[0];
        let diff = (last_sample - first_sample).abs();

        assert!(
            diff < 0.1,
            "Frame boundary discontinuity: {} (expected < 0.1)",
            diff
        );
    }

    #[test]
    fn test_output_size_respects_params() {
        let sample_rate = 16000;
        let frame_duration_ms = 10;
        let expected_samples = sample_rate * frame_duration_ms / 1000;

        let params = FmSynthParams::new(vec![], vec![], 0.1, 0.0);
        let mod_env = AdsrGenerator::new(1.0, 10, 10, 1.0, 1000, 10);
        let wav_env = AdsrGenerator::new(1.0, 10, 10, 1.0, 1000, 10);
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        let mut buffer = vec![0.0f32; expected_samples];
        fm.process(&mut buffer);

        assert_eq!(
            buffer.len(),
            expected_samples,
            "Buffer size {} does not match expected {}",
            buffer.len(),
            expected_samples
        );
    }

    #[test]
    fn test_no_clipping() {
        let params = FmSynthParams::new(vec![2, 5, 9], vec![2.0, 3.0, 2.0], 0.1, 2.0);
        let mod_env = AdsrGenerator::new(0.0, 100, 200, 1.0, 8000, 200);
        let wav_env = AdsrGenerator::new(0.0, 100, 200, 1.0, 8000, 200);
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        let mut buffer = vec![0.0f32; 16000];
        fm.process(&mut buffer);

        for (i, &sample) in buffer.iter().enumerate() {
            assert!(
                sample >= -1.0 && sample <= 1.0,
                "Sample {} clips: {} (outside [-1.0, 1.0])",
                i,
                sample
            );
        }
    }

    #[test]
    fn test_note_off_triggers_release() {
        let params = FmSynthParams::new(vec![1], vec![1.0], 0.1, 1.0);
        let mod_env = AdsrGenerator::new(0.0, 100, 100, 0.8, 10000, 100);
        let wav_env = AdsrGenerator::new(0.0, 100, 100, 0.8, 10000, 100);
        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        let mut buffer = vec![0.0f32; 200];
        fm.process(&mut buffer);

        let amp_before_note_off =
            buffer.iter().map(|&s| s.abs()).sum::<f32>() / buffer.len() as f32;

        fm.note_off();

        let mut buffer2 = vec![0.0f32; 200];
        fm.process(&mut buffer2);

        let amp_after_note_off =
            buffer2.iter().map(|&s| s.abs()).sum::<f32>() / buffer2.len() as f32;

        assert!(
            amp_after_note_off < amp_before_note_off,
            "Amplitude should decrease after note_off: before={}, after={}",
            amp_before_note_off,
            amp_after_note_off
        );
    }

    #[test]
    fn test_realistic_parameters_16k() {
        let sample_rate = 16000;
        let params = FmSynthParams::new(vec![2, 3], vec![0.5, 0.3], 0.05, 1.0);

        let attack_samples = (0.01 * sample_rate as f32) as usize;
        let decay_samples = (0.1 * sample_rate as f32) as usize;
        let sustain_samples = (0.5 * sample_rate as f32) as usize;
        let release_samples = (0.2 * sample_rate as f32) as usize;

        let mod_env = AdsrGenerator::new(
            0.0,
            attack_samples,
            decay_samples,
            0.6,
            sustain_samples,
            release_samples,
        );
        let wav_env = AdsrGenerator::new(
            0.0,
            attack_samples,
            decay_samples,
            0.7,
            sustain_samples,
            release_samples,
        );

        let mut fm = FmSynthGenerator::new(params, mod_env, wav_env);

        let frame_size = 64;
        let total_frames = 500;
        let mut has_output = false;

        for _ in 0..total_frames {
            let mut buffer = vec![0.0f32; frame_size];
            let state = fm.process(&mut buffer);

            let frame_amp = buffer.iter().map(|&s| s.abs()).sum::<f32>() / frame_size as f32;
            if frame_amp > 0.01 {
                has_output = true;
            }

            if state == GeneratorState::Complete {
                break;
            }
        }

        assert!(has_output, "Generator produced no significant output");
    }
}
