//! Scheduler and Pipeline orchestrator
//!
//! Coordinates event scheduling, frame-based processing, and audio generation.
//! The pipeline processes events at frame boundaries and generates audio samples.

use crate::pipeline::parser::TimedEvents;
use crate::pipeline::voicemgr::{VoiceConfig, VoiceManager};
use crate::wav::write_wav_16bit;

/// Configuration for the audio pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of samples per frame
    pub frame_size: usize,
    /// Number of samples per timestep
    pub timestep_samples: usize,
    /// Voice configuration (FM params, ADSR)
    pub voice_config: VoiceConfig,
    /// Base frequency for 1C (Hz)
    pub base_frequency: f32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            frame_size: 64,
            timestep_samples: 1000, // ≈22.7ms at 44.1kHz
            voice_config: VoiceConfig::default(),
            base_frequency: 110.0, // 1C = 110 Hz
        }
    }
}

/// Pipeline for processing musical events and generating audio
pub struct Pipeline {
    config: PipelineConfig,
    voice_manager: VoiceManager,
    events: Vec<TimedEvents>,
    /// Current sample position
    current_sample: usize,
    /// Current event index
    event_index: usize,
    /// Samples until next event
    samples_to_next_event: usize,
    /// Whether there are more events to process
    has_more_events: bool,
}

impl Pipeline {
    /// Create a new pipeline
    ///
    /// # Arguments
    /// * `config` - Pipeline configuration
    /// * `events` - Parsed events in chronological order
    pub fn new(config: PipelineConfig, events: Vec<TimedEvents>) -> Self {
        let voice_manager = VoiceManager::new(
            config.voice_config.clone(),
            config.base_frequency,
            config.sample_rate,
        );

        let has_more_events = !events.is_empty();
        let samples_to_next_event = if has_more_events {
            events[0].delta * config.timestep_samples
        } else {
            0
        };

        Self {
            config,
            voice_manager,
            events,
            current_sample: 0,
            event_index: 0,
            samples_to_next_event,
            has_more_events,
        }
    }

    /// Check if there are more events or active voices
    pub fn is_active(&self) -> bool {
        self.has_more_events || self.voice_manager.has_active_voices()
    }

    /// Process pending events at the current frame boundary
    fn process_events(&mut self) {
        while self.has_more_events && self.samples_to_next_event == 0 {
            // Process all events at this timestep
            let timed = &self.events[self.event_index];
            for event in &timed.events {
                self.voice_manager
                    .handle_event(&event.note, event.direction);
            }

            // Move to next event
            self.event_index += 1;

            if self.event_index >= self.events.len() {
                self.has_more_events = false;
                self.samples_to_next_event = usize::MAX;
            } else {
                self.samples_to_next_event =
                    self.events[self.event_index].delta * self.config.timestep_samples;
            }
        }
    }

    /// Advance time and decrement countdown to next event
    fn advance_time(&mut self, samples: usize) {
        self.current_sample += samples;

        if self.has_more_events && self.samples_to_next_event > 0 {
            self.samples_to_next_event = self.samples_to_next_event.saturating_sub(samples);
        }
    }

    /// Process one frame of audio
    ///
    /// Returns the samples for this frame.
    pub fn process_frame(&mut self, buffer: &mut [f32]) {
        // Process events at frame boundary (start of frame)
        self.process_events();

        // Generate audio
        self.voice_manager.process_frame(buffer);

        // Advance time
        self.advance_time(buffer.len());
    }

    /// Generate complete audio and write to WAV file
    ///
    /// # Arguments
    /// * `output_path` - Path for output WAV file
    pub fn generate_wav(&mut self, output_path: &str) -> std::io::Result<()> {
        let mut samples = Vec::new();
        let mut frame_buffer = vec![0.0f32; self.config.frame_size];

        // Process until all events and voices complete
        let max_samples = self.events.iter().map(|e| e.delta).sum::<usize>()
            * self.config.timestep_samples
            + self.config.voice_config.release_samples * 2;
        let mut safety_counter = 0;
        let max_iterations = max_samples / self.config.frame_size + 1000;

        while self.is_active() && safety_counter < max_iterations {
            self.process_frame(&mut frame_buffer);
            samples.extend_from_slice(&frame_buffer);
            safety_counter += 1;
        }

        // Add trailing silence for releases to complete
        // (VoiceManager should handle this, but let's be safe)
        let trailing_frames = 10;
        for _ in 0..trailing_frames {
            if !self.voice_manager.has_active_voices() {
                break;
            }
            self.process_frame(&mut frame_buffer);
            samples.extend_from_slice(&frame_buffer);
        }

        write_wav_16bit(output_path, &samples, self.config.sample_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::parser::{KeyDirection, Note, PitchClass};

    fn create_simple_event(delta: usize, note: Note, direction: KeyDirection) -> TimedEvents {
        TimedEvents {
            delta,
            events: vec![super::super::parser::Event { note, direction }],
        }
    }

    fn c4() -> Note {
        Note {
            octave: 4,
            pitch_class: PitchClass::C,
        }
    }

    #[test]
    fn test_pipeline_single_note() {
        let config = PipelineConfig {
            timestep_samples: 100,
            frame_size: 32,
            ..Default::default()
        };

        // Simple: note down at t=0, note up at t=100 timesteps
        let events = vec![
            create_simple_event(0, c4(), KeyDirection::Down),
            create_simple_event(100, c4(), KeyDirection::Up),
        ];

        let mut pipeline = Pipeline::new(config, events);

        // Should be active initially
        assert!(pipeline.is_active());

        // Process a few frames
        let mut buffer = vec![0.0f32; 32];
        for _ in 0..5 {
            pipeline.process_frame(&mut buffer);
        }

        // Should still have active voice
        assert!(pipeline.voice_manager.voice_count() > 0);
    }

    #[test]
    fn test_pipeline_event_timing() {
        let config = PipelineConfig {
            timestep_samples: 100,
            frame_size: 32,
            ..Default::default()
        };

        let events = vec![
            create_simple_event(1, c4(), KeyDirection::Down),
            create_simple_event(2, c4(), KeyDirection::Up),
        ];

        let mut pipeline = Pipeline::new(config, events);

        // First event at 1 * 100 = 100 samples
        assert_eq!(pipeline.samples_to_next_event, 100);

        // Process first frame (32 samples)
        let mut buffer = vec![0.0f32; 32];
        pipeline.process_frame(&mut buffer);

        // Should have decremented
        assert_eq!(pipeline.samples_to_next_event, 68);
    }
}
