#!/bin/bash
# ADSR Test Script
# Generates plots for various ADSR envelope configurations

set -e

OUTPUT_DIR="./test_plots"
mkdir -p "$OUTPUT_DIR"

echo "ADSR Plot Generator - Test Suite"
echo "================================="
echo "Output directory: $OUTPUT_DIR"
echo ""

# Check if plot-adsr is available
if command -v cargo &> /dev/null; then
    PLOT_ADSR="cargo run --quiet --bin plot-adsr --"
else
    echo "Error: cargo not found. Please install Rust."
    exit 1
fi

run_test() {
    local name="$1"
    local attack="$2"
    local decay="$3"
    local sustain="$4"
    local release="$5"
    local note_off="$6"
    local output="$OUTPUT_DIR/${name}.svg"
    
    echo "Test: $name"
    echo "  A=$attack D=$decay S=$sustain R=$release note_off=$note_off"
    $PLOT_ADSR "$attack" "$decay" "$sustain" "$release" "$note_off" "$output"
    echo "  ✓ Generated: $output"
    echo ""
}

echo "Running test cases..."
echo ""

# Test 1: Standard envelope - note_off after full decay
run_test "01_standard" 100 200 0.7 300 640

# Test 2: Early release during attack
run_test "02_early_attack" 100 200 0.7 300 50

# Test 3: Early release during decay (halfway through)
run_test "03_early_decay" 100 200 0.7 300 150

# Test 4: Zero sustain level
run_test "04_zero_sustain" 100 200 0.0 300 640

# Test 5: Maximum sustain level (sustain = 1.0)
run_test "05_max_sustain" 100 200 1.0 300 640

# Test 6: Short timings
run_test "06_short_times" 10 10 0.5 10 50

# Test 7: Long attack with early release
run_test "07_long_attack" 500 200 0.7 300 100

# Test 8: Short release
run_test "08_short_release" 100 200 0.7 50 640

# Test 9: High sustain (0.9)
run_test "09_high_sustain" 100 200 0.9 300 640

# Test 10: Very early release (at 10ms)
run_test "10_very_early" 100 200 0.7 300 10

echo "================================="
echo "All tests passed!"
echo "Plots generated in: $OUTPUT_DIR"
echo ""
echo "View the plots:"
ls -1 "$OUTPUT_DIR"/*.svg | while read f; do
    echo "  - $f"
done
