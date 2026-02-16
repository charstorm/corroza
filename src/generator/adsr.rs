use super::{GeneratorState, SignalGenerator};

/// ADSR (Attack-Decay-Sustain-Release) envelope generator
///
/// Produces an amplitude envelope with four phases:
/// 1. Attack: ramps from initial amplitude to peak (1.0)
/// 2. Decay: ramps from peak to sustain level
/// 3. Sustain: holds at sustain level until note_off or max duration
/// 4. Release: ramps from current amplitude to 0.0
///
/// All transitions are smooth (no discontinuity in amplitude).
/// External events (note_off) are processed at frame boundaries.
pub struct AdsrGenerator {
    // Configuration
    initial_amplitude: f32,
    attack_duration: usize,
    decay_duration: usize,
    sustain_level: f32,
    sustain_max_duration: usize,
    release_duration: usize,

    // State
    phase: AdsrPhase,
    position: usize,
    sustain_position: usize,
    current_amplitude: f32,
    release_start_amplitude: f32,

    // Event queue
    pending_note_off: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdsrPhase {
    Attack,
    Decay,
    Sustain,
    Release,
    Complete,
}

impl AdsrGenerator {
    /// Create a new ADSR envelope generator
    ///
    /// # Arguments
    /// * `initial_amplitude` - Starting amplitude (typically 0.0)
    /// * `attack_ms` - Attack phase duration in milliseconds
    /// * `decay_ms` - Decay phase duration in milliseconds
    /// * `sustain_level` - Sustain phase amplitude level (0.0 to 1.0)
    /// * `sustain_max_ms` - Maximum sustain duration (default: 2000.0 ms)
    /// * `release_ms` - Release phase duration in milliseconds
    /// * `sample_rate` - Sample rate in Hz
    ///
    /// # Example
    /// ```
    /// use corroza::generator::adsr::AdsrGenerator;
    ///
    /// let adsr = AdsrGenerator::new(
    ///     0.0,      // initial amplitude
    ///     100.0,    // attack: 100ms
    ///     200.0,    // decay: 200ms
    ///     0.7,      // sustain: 70% amplitude
    ///     2000.0,   // max sustain: 2 seconds
    ///     500.0,    // release: 500ms
    ///     44100.0,  // sample rate
    /// );
    /// ```
    pub fn new(
        initial_amplitude: f32,
        attack_ms: f32,
        decay_ms: f32,
        sustain_level: f32,
        sustain_max_ms: f32,
        release_ms: f32,
        sample_rate: f32,
    ) -> Self {
        let samples_from_ms = |ms: f32| ((ms / 1000.0) * sample_rate) as usize;

        Self {
            initial_amplitude: initial_amplitude.clamp(0.0, 1.0),
            attack_duration: samples_from_ms(attack_ms).max(1),
            decay_duration: samples_from_ms(decay_ms).max(1),
            sustain_level: sustain_level.clamp(0.0, 1.0),
            sustain_max_duration: samples_from_ms(sustain_max_ms).max(1),
            release_duration: samples_from_ms(release_ms).max(1),
            phase: AdsrPhase::Attack,
            position: 0,
            sustain_position: 0,
            current_amplitude: initial_amplitude.clamp(0.0, 1.0),
            release_start_amplitude: 0.0,
            pending_note_off: false,
        }
    }

    /// Queue a note off event
    ///
    /// The event will be processed at the start of the next frame.
    /// This triggers the Release phase from the current amplitude.
    pub fn note_off(&mut self) {
        self.pending_note_off = true;
    }

    /// Get the current amplitude
    ///
    /// Useful for debugging, visualization, or chaining generators.
    pub fn current_amplitude(&self) -> f32 {
        self.current_amplitude
    }

    /// Get the current phase
    pub fn phase(&self) -> AdsrPhase {
        self.phase
    }

    /// Check if the envelope has completed (Release phase finished)
    pub fn is_complete(&self) -> bool {
        self.phase == AdsrPhase::Complete
    }

    /// Process pending events at frame boundary
    fn process_events(&mut self) {
        if self.pending_note_off {
            self.pending_note_off = false;
            match self.phase {
                AdsrPhase::Attack | AdsrPhase::Decay | AdsrPhase::Sustain => {
                    self.phase = AdsrPhase::Release;
                    self.release_start_amplitude = self.current_amplitude;
                    self.position = 0;
                }
                _ => {}
            }
        }

        // Check sustain max duration
        if self.phase == AdsrPhase::Sustain && self.sustain_position >= self.sustain_max_duration {
            self.phase = AdsrPhase::Release;
            self.release_start_amplitude = self.current_amplitude;
            self.position = 0;
        }
    }

    /// Generate samples for the Attack phase
    fn process_attack(&mut self, buffer: &mut [f32]) -> GeneratorState {
        let start_amp = self.initial_amplitude;
        let end_amp = 1.0f32;
        let total_samples = self.attack_duration;

        for (i, sample) in buffer.iter_mut().enumerate() {
            let global_pos = self.position + i;
            if global_pos < total_samples {
                let t = global_pos as f32 / (total_samples - 1).max(1) as f32;
                self.current_amplitude = start_amp + (end_amp - start_amp) * t;
                *sample = self.current_amplitude;
            } else {
                // Attack complete - transition to Decay and process remaining samples
                self.phase = AdsrPhase::Decay;
                self.position = 0;
                // Process remaining buffer as Decay
                self.process_decay(&mut buffer[i..]);
                return GeneratorState::Running;
            }
        }

        self.position += buffer.len();

        // Check if we completed the phase exactly at frame end
        if self.position >= total_samples {
            self.phase = AdsrPhase::Decay;
            self.position = 0;
        }

        GeneratorState::Running
    }

    /// Generate samples for the Decay phase
    fn process_decay(&mut self, buffer: &mut [f32]) -> GeneratorState {
        let start_amp = 1.0f32;
        let end_amp = self.sustain_level;
        let total_samples = self.decay_duration;

        for (i, sample) in buffer.iter_mut().enumerate() {
            let global_pos = self.position + i;
            if global_pos < total_samples {
                let t = global_pos as f32 / (total_samples - 1).max(1) as f32;
                self.current_amplitude = start_amp + (end_amp - start_amp) * t;
                *sample = self.current_amplitude;
            } else {
                // Decay complete - transition to Sustain and process remaining samples
                self.phase = AdsrPhase::Sustain;
                self.position = 0;
                self.sustain_position = 0;
                // Process remaining buffer as Sustain
                self.process_sustain(&mut buffer[i..]);
                return GeneratorState::Running;
            }
        }

        self.position += buffer.len();

        // Check if we completed the phase exactly at frame end
        if self.position >= total_samples {
            self.phase = AdsrPhase::Sustain;
            self.position = 0;
            self.sustain_position = 0;
        }

        GeneratorState::Running
    }

    /// Generate samples for the Sustain phase
    fn process_sustain(&mut self, buffer: &mut [f32]) -> GeneratorState {
        // Hold at sustain level
        self.current_amplitude = self.sustain_level;
        for sample in buffer.iter_mut() {
            *sample = self.sustain_level;
        }

        self.sustain_position += buffer.len();

        // Check if max sustain duration exceeded (will be handled at next frame boundary)
        if self.sustain_position >= self.sustain_max_duration {
            // Transition happens at next process_events call
        }

        GeneratorState::Running
    }

    /// Generate samples for the Release phase
    fn process_release(&mut self, buffer: &mut [f32]) -> GeneratorState {
        let start_amp = self.release_start_amplitude;
        let end_amp = 0.0f32;
        let total_samples = self.release_duration;

        for (i, sample) in buffer.iter_mut().enumerate() {
            let global_pos = self.position + i;
            if global_pos < total_samples {
                let t = global_pos as f32 / (total_samples - 1).max(1) as f32;
                self.current_amplitude = start_amp + (end_amp - start_amp) * t;
                *sample = self.current_amplitude;
            } else {
                // Release complete - transition to Complete and fill remaining with zeros
                self.phase = AdsrPhase::Complete;
                self.current_amplitude = end_amp;
                for j in i..buffer.len() {
                    buffer[j] = end_amp;
                }
                return GeneratorState::Complete;
            }
        }

        self.position += buffer.len();

        // Check if we completed the phase exactly at frame end
        if self.position >= total_samples {
            self.phase = AdsrPhase::Complete;
            return GeneratorState::Complete;
        }

        GeneratorState::Running
    }
}

impl SignalGenerator for AdsrGenerator {
    fn process(&mut self, buffer: &mut [f32]) -> GeneratorState {
        // Process events at frame boundary (start of frame)
        self.process_events();

        match self.phase {
            AdsrPhase::Attack => self.process_attack(buffer),
            AdsrPhase::Decay => self.process_decay(buffer),
            AdsrPhase::Sustain => self.process_sustain(buffer),
            AdsrPhase::Release => self.process_release(buffer),
            AdsrPhase::Complete => {
                // Fill with zeros after completion
                self.current_amplitude = 0.0;
                for sample in buffer.iter_mut() {
                    *sample = 0.0;
                }
                GeneratorState::Complete
            }
        }
    }

    fn is_complete(&self) -> bool {
        self.is_complete()
    }

    fn reset(&mut self) {
        self.phase = AdsrPhase::Attack;
        self.position = 0;
        self.sustain_position = 0;
        self.current_amplitude = self.initial_amplitude;
        self.release_start_amplitude = 0.0;
        self.pending_note_off = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 1000.0; // 1ms = 1 sample for easy testing

    fn create_adsr(
        initial: f32,
        attack_ms: f32,
        decay_ms: f32,
        sustain: f32,
        sustain_max_ms: f32,
        release_ms: f32,
    ) -> AdsrGenerator {
        AdsrGenerator::new(
            initial,
            attack_ms,
            decay_ms,
            sustain,
            sustain_max_ms,
            release_ms,
            SAMPLE_RATE,
        )
    }

    #[test]
    fn test_adsr_full_envelope() {
        // 100ms attack, 100ms decay, 50% sustain, 100ms release
        let mut adsr = create_adsr(0.0, 100.0, 100.0, 0.5, 2000.0, 100.0);
        let mut buffer = [0.0f32; 100];

        // Attack phase: 0.0 -> 1.0 over 100 samples
        let state = adsr.process(&mut buffer);
        assert_eq!(state, GeneratorState::Running);
        assert_eq!(adsr.phase(), AdsrPhase::Decay);
        assert_eq!(buffer[0], 0.0);
        assert_eq!(buffer[99], 1.0);
        assert!((adsr.current_amplitude() - 1.0).abs() < 0.001);

        // Decay phase: 1.0 -> 0.5 over 100 samples
        let state = adsr.process(&mut buffer);
        assert_eq!(state, GeneratorState::Running);
        assert_eq!(adsr.phase(), AdsrPhase::Sustain);
        assert!((buffer[0] - 1.0).abs() < 0.001);
        assert!((buffer[99] - 0.5).abs() < 0.001);
        assert!((adsr.current_amplitude() - 0.5).abs() < 0.001);

        // Sustain phase: hold at 0.5
        let state = adsr.process(&mut buffer);
        assert_eq!(state, GeneratorState::Running);
        assert_eq!(adsr.phase(), AdsrPhase::Sustain);
        assert_eq!(buffer[0], 0.5);
        assert_eq!(buffer[99], 0.5);
        assert_eq!(adsr.current_amplitude(), 0.5);

        // Trigger note off
        adsr.note_off();

        // Next frame should start Release - use smaller buffer to see intermediate state
        let mut small_buffer = [0.0f32; 50];
        let state = adsr.process(&mut small_buffer);
        assert_eq!(state, GeneratorState::Running);
        assert_eq!(adsr.phase(), AdsrPhase::Release);
        assert!((small_buffer[0] - 0.5).abs() < 0.001); // Start from sustain level

        // Process remaining release
        adsr.process(&mut small_buffer);
        let state = adsr.process(&mut small_buffer);
        assert_eq!(state, GeneratorState::Complete);
        assert_eq!(adsr.phase(), AdsrPhase::Complete);
        assert_eq!(adsr.current_amplitude(), 0.0);
    }

    #[test]
    fn test_amplitude_bounds() {
        let mut adsr = create_adsr(0.0, 100.0, 100.0, 0.5, 2000.0, 100.0);
        let mut buffer = [0.0f32; 50];

        // Process entire envelope in small chunks and check bounds
        while adsr.phase() != AdsrPhase::Complete {
            adsr.process(&mut buffer);
            for &sample in buffer.iter() {
                assert!(sample >= 0.0, "Sample {} below 0.0", sample);
                assert!(sample <= 1.0, "Sample {} above 1.0", sample);
            }
            assert!(
                adsr.current_amplitude() >= 0.0 && adsr.current_amplitude() <= 1.0,
                "Current amplitude {} out of bounds",
                adsr.current_amplitude()
            );
        }
    }

    #[test]
    fn test_no_discontinuity_at_transitions() {
        let mut adsr = create_adsr(0.0, 100.0, 100.0, 0.5, 2000.0, 100.0);
        let mut buffer = [0.0f32; 100];

        // Attack -> Decay
        adsr.process(&mut buffer);
        let last_attack = buffer[99];
        adsr.process(&mut buffer);
        let first_decay = buffer[0];
        assert!(
            (last_attack - first_decay).abs() < 0.001,
            "Discontinuity at Attack->Decay: {} vs {}",
            last_attack,
            first_decay
        );

        // Decay -> Sustain
        let last_decay = buffer[99];
        adsr.process(&mut buffer);
        let first_sustain = buffer[0];
        assert!(
            (last_decay - first_sustain).abs() < 0.001,
            "Discontinuity at Decay->Sustain: {} vs {}",
            last_decay,
            first_sustain
        );

        // Sustain -> Release
        adsr.note_off();
        adsr.process(&mut buffer);
        let first_release = buffer[0];
        assert!(
            (last_decay - first_release).abs() < 0.001,
            "Discontinuity at Sustain->Release: {} vs {}",
            last_decay,
            first_release
        );
    }

    #[test]
    fn test_early_release_during_attack() {
        let mut adsr = create_adsr(0.0, 100.0, 100.0, 0.5, 2000.0, 100.0);
        let mut buffer = [0.0f32; 50];

        // Process first half of attack (0.0 -> 0.5)
        adsr.process(&mut buffer);
        let amp_at_release = adsr.current_amplitude();
        assert!(amp_at_release < 1.0 && amp_at_release > 0.0);

        // Trigger note_off
        adsr.note_off();

        // Next frame should start Release from current amplitude
        let _state = adsr.process(&mut buffer);
        assert_eq!(adsr.phase(), AdsrPhase::Release);
        assert!(
            (buffer[0] - amp_at_release).abs() < 0.001,
            "Release should start from current amplitude: {} vs {}",
            buffer[0],
            amp_at_release
        );
    }

    #[test]
    fn test_early_release_during_decay() {
        let mut adsr = create_adsr(0.0, 100.0, 100.0, 0.5, 2000.0, 100.0);
        let mut buffer = [0.0f32; 100];

        // Complete attack
        adsr.process(&mut buffer);
        assert_eq!(adsr.phase(), AdsrPhase::Decay);

        // Process first half of decay (1.0 -> 0.75)
        let mut small_buffer = [0.0f32; 50];
        adsr.process(&mut small_buffer);
        let amp_at_release = adsr.current_amplitude();
        assert!(amp_at_release < 1.0 && amp_at_release > 0.5);

        // Trigger note_off
        adsr.note_off();

        // Next frame should start Release from current amplitude
        adsr.process(&mut small_buffer);
        assert_eq!(adsr.phase(), AdsrPhase::Release);
        assert!(
            (small_buffer[0] - amp_at_release).abs() < 0.001,
            "Release should start from current amplitude: {} vs {}",
            small_buffer[0],
            amp_at_release
        );
    }

    #[test]
    fn test_sustain_max_duration() {
        // 10ms sustain max
        let mut adsr = create_adsr(0.0, 10.0, 10.0, 0.5, 10.0, 10.0);
        let mut buffer = [0.0f32; 10];

        // Attack
        adsr.process(&mut buffer);
        // Decay
        adsr.process(&mut buffer);
        // Sustain (10 samples)
        adsr.process(&mut buffer);
        assert_eq!(adsr.phase(), AdsrPhase::Sustain);

        // This should auto-trigger release - use smaller buffer to see Release phase
        let mut small_buffer = [0.0f32; 5];
        adsr.process(&mut small_buffer);
        assert_eq!(adsr.phase(), AdsrPhase::Release);
    }

    #[test]
    fn test_custom_sustain_max() {
        // 5ms sustain max instead of default
        let mut adsr = create_adsr(0.0, 10.0, 10.0, 0.5, 5.0, 10.0);
        let mut buffer = [0.0f32; 10];

        // Attack + Decay (20 samples)
        adsr.process(&mut buffer);
        adsr.process(&mut buffer);
        assert_eq!(adsr.phase(), AdsrPhase::Sustain);

        // Only 5 samples of sustain before auto-release
        let mut small_buffer = [0.0f32; 5];
        adsr.process(&mut small_buffer);
        // Check that next frame triggers release
        adsr.process(&mut small_buffer);
        assert_eq!(adsr.phase(), AdsrPhase::Release);
    }

    #[test]
    fn test_initial_amplitude() {
        // Start from 0.5 instead of 0
        let mut adsr = create_adsr(0.5, 100.0, 100.0, 0.5, 2000.0, 100.0);
        let mut buffer = [0.0f32; 10];

        adsr.process(&mut buffer);
        assert!(
            (buffer[0] - 0.5).abs() < 0.001,
            "Should start from initial amplitude"
        );
    }

    #[test]
    fn test_zero_durations() {
        // Zero attack (should still have 1 sample minimum)
        let mut adsr = create_adsr(0.0, 0.0, 100.0, 0.5, 2000.0, 100.0);
        let mut buffer = [0.0f32; 10];

        adsr.process(&mut buffer);
        // Should immediately go to decay or sustain
        assert!(adsr.phase() == AdsrPhase::Decay || adsr.phase() == AdsrPhase::Sustain);
    }

    #[test]
    fn test_sustain_level_edge_cases() {
        // Sustain level = 0
        let mut adsr = create_adsr(0.0, 10.0, 10.0, 0.0, 2000.0, 10.0);
        let mut buffer = [0.0f32; 10];

        adsr.process(&mut buffer); // Attack
        adsr.process(&mut buffer); // Decay
        adsr.process(&mut buffer); // Sustain
        assert_eq!(buffer[0], 0.0);
        assert_eq!(adsr.current_amplitude(), 0.0);

        // Sustain level = 1.0 (same as peak)
        let mut adsr2 = create_adsr(0.0, 10.0, 10.0, 1.0, 2000.0, 10.0);
        adsr2.process(&mut buffer); // Attack
        adsr2.process(&mut buffer); // Decay (should stay at 1.0)
        adsr2.process(&mut buffer); // Sustain
        assert!((buffer[0] - 1.0).abs() < 0.001);
        assert!((adsr2.current_amplitude() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_reset() {
        let mut adsr = create_adsr(0.0, 100.0, 100.0, 0.5, 2000.0, 100.0);
        let mut buffer = [0.0f32; 100];

        // Process through to completion
        while adsr.phase() != AdsrPhase::Complete {
            adsr.process(&mut buffer);
        }

        // Reset
        adsr.reset();

        // Should be back at Attack phase with initial amplitude
        assert_eq!(adsr.phase(), AdsrPhase::Attack);
        assert_eq!(adsr.current_amplitude(), 0.0);
        assert!(!adsr.is_complete());
    }

    #[test]
    fn test_frame_boundary_event_processing() {
        let mut adsr = create_adsr(0.0, 100.0, 100.0, 0.5, 2000.0, 100.0);
        let mut buffer = [0.0f32; 100];

        // Start attack
        adsr.process(&mut buffer);
        assert_eq!(adsr.phase(), AdsrPhase::Decay);

        // In decay phase now
        let mut small_buffer = [0.0f32; 30];
        adsr.process(&mut small_buffer);
        assert_eq!(adsr.phase(), AdsrPhase::Decay);

        // Trigger note_off
        adsr.note_off();

        // Still in decay until frame boundary
        assert_eq!(adsr.phase(), AdsrPhase::Decay);

        // Next process call triggers event at frame start
        adsr.process(&mut small_buffer);
        assert_eq!(adsr.phase(), AdsrPhase::Release);
    }

    #[test]
    fn test_release_from_various_amplitudes() {
        let test_cases = vec![
            (0.0, 0.25), // Release from quarter way through attack
            (0.0, 0.5),  // Release from half way through attack
            (0.0, 0.75), // Release from three quarters through attack
            (0.0, 1.0),  // Release from peak (normal)
        ];

        for (initial, progress) in test_cases {
            let mut adsr = create_adsr(initial, 100.0, 100.0, 0.5, 2000.0, 100.0);
            let buffer_size = (100.0 * progress) as usize;
            let mut buffer = vec![0.0f32; buffer_size];

            // Process to specific point in attack
            if buffer_size > 0 {
                adsr.process(&mut buffer);
            }

            let amp_before = adsr.current_amplitude();
            adsr.note_off();

            // Next frame should start from exact amplitude
            let mut release_buffer = [0.0f32; 10];
            adsr.process(&mut release_buffer);

            assert!(
                (release_buffer[0] - amp_before).abs() < 0.001,
                "Release at progress {} should start from {} not {}",
                progress,
                amp_before,
                release_buffer[0]
            );
        }
    }

    #[test]
    fn test_final_value_reached() {
        let mut adsr = create_adsr(0.0, 10.0, 10.0, 0.5, 2000.0, 10.0);
        let mut buffer = [0.0f32; 10];

        // Run through entire envelope
        adsr.process(&mut buffer); // Attack (0->1)
        adsr.process(&mut buffer); // Decay (1->0.5)
        adsr.note_off();
        adsr.process(&mut buffer); // Release (0.5->0)
        adsr.process(&mut buffer); // Complete

        assert_eq!(adsr.current_amplitude(), 0.0);
        assert_eq!(buffer[9], 0.0); // Last sample is 0
    }
}
