//! Audio processing pipeline
//!
//! Provides a complete event-driven audio synthesis pipeline:
//! - Parser: Parse musical transcription format
//! - VoiceManager: Polyphonic voice management
//! - Scheduler: Frame-based event scheduling and audio generation

pub mod parser;
pub mod scheduler;
pub mod voicemgr;

pub use parser::{parse_transcription, Event, KeyDirection, Note, ParseError, TimedEvents};
pub use scheduler::{Pipeline, PipelineConfig};
pub use voicemgr::{VoiceConfig, VoiceManager};
