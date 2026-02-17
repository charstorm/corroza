#!/bin/bash
# FM Synthesis Audio Generator - Simple Template
# Edit the variables below and run the script

# Output Configuration
# Sample rate in Hz (e.g., 16000, 44100, 48000)
SAMPLE_RATE=16000
# Silence padding at start and end in seconds
SILENCE_SECONDS=0.5

# FM Synthesis Parameters
# Harmonics: frequency multipliers (comma-separated, no spaces within the string)
HARMONICS="2,7,20"
# Amplitudes for each harmonic (comma-separated, must match number of harmonics)
AMPS="1.0,2.0,0.5"
# Phase per sample (controls base frequency)
# At 16kHz: 0.05 = ~127 Hz, 0.1 = ~255 Hz, 0.02 = ~51 Hz
PHASE_PER_SAMPLE=0.05

# Modulation Depth (float)
# Controls the overall intensity of FM modulation
# 0.0 = no modulation (pure sine), 1.0 = full modulation, 2.0+ = extreme modulation
MOD_DEPTH=0.3

# Modulation Envelope Parameters (ADSR)
# Controls how much FM modulation is applied over time
MOD_ATTACK=400
MOD_DECAY=800
MOD_SUSTAIN_LEVEL=0.5
MOD_SUSTAIN_TIME=6000
MOD_RELEASE=3200

# Waveform Envelope Parameters (ADSR)
# Controls the output amplitude over time
# NOTE: If these don't match the modulation envelope, a warning will be shown
WAV_ATTACK=400
WAV_DECAY=800
WAV_SUSTAIN_LEVEL=0.5
WAV_SUSTAIN_TIME=6000
WAV_RELEASE=3200

# Output file
OUTPUT_FILE="tmp/fm_synth.wav"

# Calculate envelope totals for reference
MOD_TOTAL=$((MOD_ATTACK + MOD_DECAY + MOD_SUSTAIN_TIME + MOD_RELEASE))
WAV_TOTAL=$((WAV_ATTACK + WAV_DECAY + WAV_SUSTAIN_TIME + WAV_RELEASE))

echo "Envelope Durations:"
echo "  Modulation envelope: $MOD_TOTAL samples"
echo "  Waveform envelope:   $WAV_TOTAL samples"
if [ "$MOD_TOTAL" -ne "$WAV_TOTAL" ]; then
    echo "  WARNING: Envelopes have different lengths!"
fi
echo ""

# Run the FM synthesis generator
cargo run --quiet --bin fm-synth -- \
    "$SAMPLE_RATE" \
    "$SILENCE_SECONDS" \
    "$HARMONICS" \
    "$AMPS" \
    "$PHASE_PER_SAMPLE" \
    "$MOD_DEPTH" \
    "$MOD_ATTACK" \
    "$MOD_DECAY" \
    "$MOD_SUSTAIN_LEVEL" \
    "$MOD_SUSTAIN_TIME" \
    "$MOD_RELEASE" \
    "$WAV_ATTACK" \
    "$WAV_DECAY" \
    "$WAV_SUSTAIN_LEVEL" \
    "$WAV_SUSTAIN_TIME" \
    "$WAV_RELEASE" \
    "$OUTPUT_FILE"
