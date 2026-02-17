//! Voice manager for polyphonic synthesis
//!
//! Manages active FM synthesizer voices, handling note allocation,
//! note release, and cleanup of completed voices.

use crate::generator::adsr::AdsrGenerator;
use crate::generator::fm_synth::{FmSynthGenerator, FmSynthParams};
use crate::generator::{GeneratorState, SignalGenerator};
use crate::pipeline::parser::{KeyDirection, Note};

/// Configuration for all voices (common settings)
#[derive(Debug, Clone)]
pub struct VoiceConfig {
    /// FM synthesis parameters (shared by all voices)
    pub fm_params: FmSynthParams,
    /// ADSR attack duration in samples
    pub attack_samples: usize,
    /// ADSR decay duration in samples
    pub decay_samples: usize,
    /// ADSR sustain level (0.0 to 1.0)
    pub sustain_level: f32,
    /// ADSR release duration in samples
    pub release_samples: usize,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        // Default FM params: [2,5,9] harmonics with [1.0,2.0,1.0] amps
        let fm_params = FmSynthParams::new(
            vec![2, 5, 9],
            vec![1.0, 2.0, 1.0],
            0.1, // placeholder, will be set per note
            1.0,
        );

        Self {
            fm_params,
            attack_samples: 4410, // 100ms at 44.1kHz
            decay_samples: 8820,  // 200ms at 44.1kHz
            sustain_level: 0.7,
            release_samples: 13230, // 300ms at 44.1kHz
        }
    }
}

/// An active voice with its associated note and synthesizer
struct Voice {
    note: Note,
    synth: FmSynthGenerator,
    is_releasing: bool,
}

/// Manages polyphonic voices
pub struct VoiceManager {
    config: VoiceConfig,
    active_voices: Vec<Voice>,
    base_frequency: f32,
    sample_rate: u32,
}

impl VoiceManager {
    /// Create a new voice manager
    ///
    /// # Arguments
    /// * `config` - Voice configuration (FM params, ADSR settings)
    /// * `base_frequency` - Frequency of 1C in Hz (e.g., 110.0)
    /// * `sample_rate` - Sample rate in Hz
    pub fn new(config: VoiceConfig, base_frequency: f32, sample_rate: u32) -> Self {
        Self {
            config,
            active_voices: Vec::new(),
            base_frequency,
            sample_rate,
        }
    }

    /// Calculate frequency for a note
    ///
    /// Formula: f = base_freq * 2^((octave-1) + semitone/12)
    fn note_frequency(&self, note: &Note) -> f32 {
        let octave_offset = (note.octave as f32 - 1.0) * 12.0;
        let semitone_offset = note.pitch_class.semitone() as f32;
        let total_semitones = octave_offset + semitone_offset;
        self.base_frequency * 2f32.powf(total_semitones / 12.0)
    }

    /// Calculate phase increment per sample for a frequency
    fn phase_per_sample(&self, frequency: f32) -> f32 {
        2.0 * std::f32::consts::PI * frequency / self.sample_rate as f32
    }

    /// Create a new FM synthesizer for a note
    fn create_synth(&self, note: &Note) -> FmSynthGenerator {
        let frequency = self.note_frequency(note);
        let phase_per_sample = self.phase_per_sample(frequency);

        // Clone base params and set phase_per_sample for this note's frequency
        let mut fm_params = self.config.fm_params.clone();
        fm_params.phase_per_sample = phase_per_sample;

        // Create ADSR envelopes - both with same settings
        // Use a large but not max value for sustain to avoid overflow
        let max_sustain = self.sample_rate as usize * 60 * 60; // 1 hour max
        let mod_env = AdsrGenerator::new(
            0.0,
            self.config.attack_samples,
            self.config.decay_samples,
            self.config.sustain_level,
            max_sustain, // Large max sustain - effectively wait for note_off
            self.config.release_samples,
        );

        let wav_env = AdsrGenerator::new(
            0.0,
            self.config.attack_samples,
            self.config.decay_samples,
            self.config.sustain_level,
            max_sustain, // Large max sustain - effectively wait for note_off
            self.config.release_samples,
        );

        FmSynthGenerator::new(fm_params, mod_env, wav_env)
    }

    /// Handle a note event (key down or key up)
    pub fn handle_event(&mut self, note: &Note, direction: KeyDirection) {
        match direction {
            KeyDirection::Down => {
                // Check if note is already active (ignore duplicates)
                if self
                    .active_voices
                    .iter()
                    .any(|v| v.note == *note && !v.is_releasing)
                {
                    return;
                }

                // Create new voice
                let synth = self.create_synth(note);
                let voice = Voice {
                    note: *note,
                    synth,
                    is_releasing: false,
                };
                self.active_voices.push(voice);
            }
            KeyDirection::Up => {
                // Find the active voice for this note and trigger release
                if let Some(voice) = self
                    .active_voices
                    .iter_mut()
                    .find(|v| v.note == *note && !v.is_releasing)
                {
                    voice.synth.note_off();
                    voice.is_releasing = true;
                }
            }
        }
    }

    /// Process one frame and mix all active voices
    ///
    /// Returns the mixed samples for this frame and removes completed voices.
    pub fn process_frame(&mut self, buffer: &mut [f32]) {
        // Clear buffer
        for sample in buffer.iter_mut() {
            *sample = 0.0;
        }

        if self.active_voices.is_empty() {
            return;
        }

        // Temporary buffer for individual voice processing
        let mut voice_buffer = vec![0.0f32; buffer.len()];

        // Track which voices completed this frame
        let mut completed_indices: Vec<usize> = Vec::new();

        // Process each voice and accumulate
        for (i, voice) in self.active_voices.iter_mut().enumerate() {
            // Clear voice buffer
            for sample in voice_buffer.iter_mut() {
                *sample = 0.0;
            }

            // Process voice
            let state = voice.synth.process(&mut voice_buffer);

            // Add to mix
            for (j, sample) in buffer.iter_mut().enumerate() {
                *sample += voice_buffer[j];
            }

            // Check if voice completed
            if state == GeneratorState::Complete {
                completed_indices.push(i);
            }
        }

        // Remove completed voices (in reverse order to maintain indices)
        for &i in completed_indices.iter().rev() {
            self.active_voices.remove(i);
        }

        // Clip to prevent overflow (soft clip)
        for sample in buffer.iter_mut() {
            *sample = soft_clip(*sample);
        }
    }

    /// Check if there are any active voices
    pub fn has_active_voices(&self) -> bool {
        !self.active_voices.is_empty()
    }

    /// Get the number of active voices
    pub fn voice_count(&self) -> usize {
        self.active_voices.len()
    }

    /// Trigger release on all active voices (for early termination)
    pub fn all_notes_off(&mut self) {
        for voice in self.active_voices.iter_mut() {
            if !voice.is_releasing {
                voice.synth.note_off();
                voice.is_releasing = true;
            }
        }
    }

    /// Clear all voices immediately
    pub fn clear(&mut self) {
        self.active_voices.clear();
    }
}

/// Soft clipping to prevent distortion
/// Uses a gentle tanh-like curve for values above threshold
fn soft_clip(sample: f32) -> f32 {
    if sample.abs() <= 1.0 {
        sample
    } else {
        sample.signum() * (1.0 + (sample.abs() - 1.0).tanh() * 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::parser::PitchClass;

    fn create_test_manager() -> VoiceManager {
        VoiceManager::new(VoiceConfig::default(), 110.0, 44100)
    }

    #[test]
    fn test_note_frequency() {
        let mgr = create_test_manager();

        // 1C should be base frequency
        let c1 = Note {
            octave: 1,
            pitch_class: PitchClass::C,
        };
        assert!((mgr.note_frequency(&c1) - 110.0).abs() < 0.01);

        // 2C should be 2x base (one octave up)
        let c2 = Note {
            octave: 2,
            pitch_class: PitchClass::C,
        };
        assert!((mgr.note_frequency(&c2) - 220.0).abs() < 0.01);

        // 4C should be 8x base (three octaves up from 1C)
        let c4 = Note {
            octave: 4,
            pitch_class: PitchClass::C,
        };
        assert!((mgr.note_frequency(&c4) - 880.0).abs() < 0.1);

        // 4A (9 semitones above 4C): 880 * 2^(9/12) ≈ 1480 Hz
        let a4 = Note {
            octave: 4,
            pitch_class: PitchClass::A,
        };
        let freq = mgr.note_frequency(&a4);
        let expected = 880.0 * 2f32.powf(9.0 / 12.0);
        assert!(
            (freq - expected).abs() < 1.0,
            "Expected ~{} Hz, got {} Hz",
            expected,
            freq
        );
    }

    #[test]
    fn test_duplicate_note_ignored() {
        let mut mgr = create_test_manager();
        let note = Note {
            octave: 4,
            pitch_class: PitchClass::C,
        };

        mgr.handle_event(&note, KeyDirection::Down);
        assert_eq!(mgr.voice_count(), 1);

        // Duplicate should be ignored
        mgr.handle_event(&note, KeyDirection::Down);
        assert_eq!(mgr.voice_count(), 1);
    }

    #[test]
    fn test_note_release() {
        let mut mgr = create_test_manager();
        let note = Note {
            octave: 4,
            pitch_class: PitchClass::C,
        };

        mgr.handle_event(&note, KeyDirection::Down);
        assert_eq!(mgr.voice_count(), 1);

        // Release note
        mgr.handle_event(&note, KeyDirection::Up);
        assert_eq!(mgr.voice_count(), 1); // Still there, but releasing

        // Process until complete
        let mut buffer = vec![0.0f32; 64];
        let max_iterations = 10000;
        for _ in 0..max_iterations {
            mgr.process_frame(&mut buffer);
            if !mgr.has_active_voices() {
                break;
            }
        }

        assert!(!mgr.has_active_voices());
    }

    #[test]
    fn test_polyphony() {
        let mut mgr = create_test_manager();

        // Start multiple notes
        let notes = [
            Note {
                octave: 4,
                pitch_class: PitchClass::C,
            },
            Note {
                octave: 4,
                pitch_class: PitchClass::E,
            },
            Note {
                octave: 4,
                pitch_class: PitchClass::G,
            },
        ];

        for note in &notes {
            mgr.handle_event(note, KeyDirection::Down);
        }

        assert_eq!(mgr.voice_count(), 3);

        // Process one frame
        let mut buffer = vec![0.0f32; 64];
        mgr.process_frame(&mut buffer);

        // Should still have 3 voices
        assert_eq!(mgr.voice_count(), 3);
    }
}
