# Corroza

A real-time audio synthesis library in Rust.

## Project Scope

### Overview

This project aims to build a real-time audio synthesis library using an event-driven architecture. The library processes audio generation in response to triggers (such as key presses) and produces high-quality synthesized audio with minimal latency.

### Core Architecture: Signal Generators

The fundamental building block is the **signal generator** – an independent, composable module that produces audio samples. Key characteristics:

- **Composability**: Multiple signal generators can be combined to form more complex audio structures
- **Independence**: Each generator operates autonomously and can run in parallel with others
- **Lifecycle Management**: Generators have clear start and end points, with automatic garbage collection when complete

### Signal Generator Lifecycle

**Start Point (Trigger)**
A generator can be initiated in two ways:
1. **External Event**: Triggered by an event such as a key press or MIDI note on
2. **Concatenation**: Automatically starts when the previous generator in a concatenated sequence completes

**End Points (Multiple Types)**
1. **Fixed Duration**: Generator stops after a predetermined time
2. **Conditional**: Generation continues until specific criteria are met (e.g., amplitude threshold reached)
3. **Event-Driven**: Generator stops when an external stop event occurs (e.g., key release)

### Frame-Based Processing

- Audio is processed in fixed-size frames (typically 32 or 64 samples)
- External events are evaluated at frame boundaries
- This approach balances responsiveness with computational efficiency

### Concatenation and Lazy Evaluation

Signal generators can be chained together to form longer sequences. Since some generators may have indeterminate duration, the system uses lazy evaluation:
- Generators are processed on-demand
- A generator in a concatenation chain automatically starts when its predecessor completes
- Starting values may depend on the ending values of previous generators
- The system adapts dynamically to runtime conditions

### Synthesis Methods

The library will support multiple synthesis techniques:
- **Frequency Modulation (FM) Synthesis**: Modulation signals will themselves have envelopes following the same generator rules
- Additional synthesis methods to be added incrementally

### Active Generator Management

The system maintains a pool of active generators:
- Multiple generators run in parallel as events occur
- Completed generators are automatically removed from the active set
- Memory and CPU resources are reclaimed through garbage collection

## Immediate Scope

The current focus is building the foundation for signal generation, starting with a complete, tested implementation of an ADSR (Attack-Decay-Sustain-Release) envelope generator as the first signal generator type.

**ADSR Components**:
1. **Attack**: Fixed duration, ramps amplitude from initial value to peak
2. **Decay**: Fixed duration, decreases amplitude from peak to sustain level
3. **Sustain**: Variable duration with configurable maximum, holds constant amplitude until release
4. **Release**: Fixed duration, ramps amplitude from current level to 0

**Key Release Handling**:
The envelope responds to early key release events:
- If released during **Attack**: immediately transition to Release phase
- If released during **Decay**: immediately transition to Release phase
- If released during **Sustain**: begin Release phase as normal

All transitions are smooth with no discontinuity in amplitude.

## Current State

### Implemented

**Core Infrastructure**:
- `SignalGenerator` trait defining the interface for all generators
- `GeneratorState` enum for lifecycle management
- Frame-based processing with runtime-configurable frame sizes

**Generators**:
- **RampGenerator**: Linear ramp from 0.0 to 1.0 over configurable duration, serving as the foundational implementation example
- **AdsrGenerator**: Full ADSR envelope with configurable attack, decay, sustain, release phases
  - Configurable initial amplitude
  - Configurable sustain maximum duration (default 2 seconds)
  - Frame boundary event processing for `note_off()` triggers
  - Smooth transitions from current amplitude (no discontinuity)
  - Current amplitude accessor for debugging and chaining

**Testing**:
- Comprehensive unit test coverage for all generators
- Tests for amplitude bounds, continuity, timing accuracy, event handling, and edge cases

### Development Approach

Progress is incremental and methodical:
1. ✓ Establish the signal generator foundation (common interface, lifecycle management)
2. ✓ Implement ADSR as the first concrete generator
3. → Add basic tone generation using the envelope
4. → Add visualization capability for inspecting generated output
5. → Extend with additional generator types and synthesis methods
6. → Build toward the full real-time synthesis system

The focus remains on building a solid, well-tested foundation before expanding functionality.
