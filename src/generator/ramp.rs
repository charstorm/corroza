use super::{GeneratorState, SignalGenerator};

/// A simple linear ramp generator for testing
///
/// Ramps from 0.0 to 1.0 over a specified duration, then completes.
/// This serves as a simple example of implementing the SignalGenerator trait.
pub struct RampGenerator {
    /// Current sample position
    position: usize,
    /// Total duration in samples
    duration: usize,
    /// Whether the generator has completed
    completed: bool,
}

impl RampGenerator {
    /// Create a new ramp generator
    ///
    /// # Arguments
    /// * `duration_ms` - Duration of the ramp in milliseconds
    /// * `sample_rate` - Sample rate in Hz (e.g., 44100.0, 48000.0)
    ///
    /// # Example
    /// ```
    /// use corroza::generator::RampGenerator;
    ///
    /// let ramp = RampGenerator::new(1000.0, 44100.0); // 1 second ramp at 44.1kHz
    /// ```
    pub fn new(duration_ms: f32, sample_rate: f32) -> Self {
        let duration = ((duration_ms / 1000.0) * sample_rate) as usize;
        Self {
            position: 0,
            duration: duration.max(1), // Ensure at least 1 sample
            completed: false,
        }
    }

    /// Get the current position in samples
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get the total duration in samples
    pub fn duration(&self) -> usize {
        self.duration
    }
}

impl SignalGenerator for RampGenerator {
    fn process(&mut self, buffer: &mut [f32]) -> GeneratorState {
        if self.completed {
            // Fill with final value (1.0) after completion
            for sample in buffer.iter_mut() {
                *sample = 1.0;
            }
            return GeneratorState::Complete;
        }

        let samples_to_process = buffer.len();
        let remaining = self.duration.saturating_sub(self.position);

        for (i, sample) in buffer.iter_mut().enumerate() {
            if i < remaining {
                // Linear interpolation from 0.0 to 1.0
                let t = (self.position + i) as f32 / (self.duration - 1).max(1) as f32;
                *sample = t;
            } else {
                // Past the end - fill with 1.0
                *sample = 1.0;
            }
        }

        self.position += samples_to_process;

        if self.position >= self.duration {
            self.completed = true;
            GeneratorState::Complete
        } else {
            GeneratorState::Running
        }
    }

    fn is_complete(&self) -> bool {
        self.completed
    }

    fn reset(&mut self) {
        self.position = 0;
        self.completed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ramp_basic() {
        // Create a 10ms ramp at 1000Hz = 10 samples
        let mut ramp = RampGenerator::new(10.0, 1000.0);

        let mut buffer = [0.0f32; 5];

        // First frame (5 samples)
        let state = ramp.process(&mut buffer);
        assert_eq!(state, GeneratorState::Running);
        assert!(!ramp.is_complete());

        // Values should be 0.0, 0.111..., 0.222..., 0.333..., 0.444...
        assert_eq!(buffer[0], 0.0);
        assert!((buffer[4] - 0.44444445).abs() < 0.001);

        // Second frame (5 samples) - completes
        let state = ramp.process(&mut buffer);
        assert_eq!(state, GeneratorState::Complete);
        assert!(ramp.is_complete());

        // Should end at 1.0
        assert_eq!(buffer[4], 1.0);
    }

    #[test]
    fn test_ramp_reset() {
        let mut ramp = RampGenerator::new(10.0, 1000.0);
        let mut buffer = [0.0f32; 10];

        // Process to completion
        ramp.process(&mut buffer);
        assert!(ramp.is_complete());

        // Reset and verify we can run again
        ramp.reset();
        assert!(!ramp.is_complete());
        assert_eq!(ramp.position(), 0);

        // Should produce the same values
        let mut buffer2 = [0.0f32; 10];
        ramp.process(&mut buffer2);

        assert_eq!(buffer, buffer2);
    }

    #[test]
    fn test_ramp_post_completion() {
        let mut ramp = RampGenerator::new(5.0, 1000.0); // 5 samples
        let mut buffer = [0.0f32; 10]; // Larger frame

        // Process - should complete
        let state = ramp.process(&mut buffer);
        assert_eq!(state, GeneratorState::Complete);

        // First 5 samples should be the ramp, last 5 should be 1.0
        assert_eq!(buffer[0], 0.0);
        assert_eq!(buffer[4], 1.0);
        assert_eq!(buffer[5], 1.0);
        assert_eq!(buffer[9], 1.0);
    }
}
