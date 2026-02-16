#!/bin/bash
# ADSR Plot Generator - Simple Template
# Edit the variables below and run the script

# ADSR Parameters (in samples)
ATTACK_SAMPLES=100
DECAY_SAMPLES=200
SUSTAIN_LEVEL=0.3
RELEASE_SAMPLES=50
NOTE_OFF_SAMPLE=50
FRAME_SIZE=10

# Output file
OUTPUT_FILE="tmp/adsr_plot.svg"

# Run the plot generator
cargo run --quiet --bin plot-adsr -- \
    "$ATTACK_SAMPLES" \
    "$DECAY_SAMPLES" \
    "$SUSTAIN_LEVEL" \
    "$RELEASE_SAMPLES" \
    "$NOTE_OFF_SAMPLE" \
    "$FRAME_SIZE" \
    "$OUTPUT_FILE"
