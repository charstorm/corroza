pub mod ramp;

pub use ramp::RampGenerator;

/// Represents the current state of a signal generator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratorState {
    /// Generator is still producing samples
    Running,
    /// Generator has completed and will produce no more samples
    Complete,
}

/// Core trait for all signal generators
///
/// Signal generators produce audio samples frame by frame.
/// Each generator is independent and can run in parallel with others.
pub trait SignalGenerator {
    /// Process the next frame of samples
    ///
    /// # Arguments
    /// * `buffer` - Mutable slice to write samples into. The length determines frame size.
    ///
    /// # Returns
    /// * `GeneratorState::Running` if the generator is still active
    /// * `GeneratorState::Complete` if the generator has finished
    ///
    /// # Note
    /// Even when Complete is returned, the buffer should still be filled with valid samples
    /// (typically zeros or the final held value) for the current frame.
    fn process(&mut self, buffer: &mut [f32]) -> GeneratorState;

    /// Check if this generator has completed
    ///
    /// This is a convenience method - generators may still produce samples
    /// after returning Complete from process().
    fn is_complete(&self) -> bool;

    /// Reset the generator to its initial state
    ///
    /// This allows generators to be reused rather than recreated.
    fn reset(&mut self);
}
