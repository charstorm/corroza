#!/bin/bash
# ADSR Plot Generator - Simple Template
# Edit the variables below and run the script

# ADSR Parameters (in milliseconds)
ATTACK_MS=100
DECAY_MS=200
SUSTAIN_LEVEL=0.3
RELEASE_MS=50
NOTE_OFF_MS=200

# Output file
OUTPUT_FILE="tmp/adsr_plot.svg"

# Run the plot generator
cargo run --quiet --bin plot-adsr -- \
    "$ATTACK_MS" \
    "$DECAY_MS" \
    "$SUSTAIN_LEVEL" \
    "$RELEASE_MS" \
    "$NOTE_OFF_MS" \
    "$OUTPUT_FILE"
