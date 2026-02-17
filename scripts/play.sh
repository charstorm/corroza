#!/bin/bash
# Generate audio from musical transcription file
#
# Usage: ./play.sh <input.txt> [output.wav]
#
# Example: ./play.sh example/happy_birthday.txt

set -e

if [ $# -lt 1 ]; then
    echo "Usage: $0 <input.txt> [output.wav]"
    echo ""
    echo "Arguments:"
    echo "  input.txt     Path to transcription file"
    echo "  output.wav    Output WAV file path (optional, defaults to <input>.wav)"
    exit 1
fi

INPUT_FILE="$1"

if [ ! -f "$INPUT_FILE" ]; then
    echo "Error: Input file '$INPUT_FILE' not found"
    exit 1
fi

if [ $# -ge 2 ]; then
    OUTPUT_FILE="$2"
else
    # Default: replace .txt with .wav
    OUTPUT_FILE="${INPUT_FILE%.txt}.wav"
fi

echo "Input:  $INPUT_FILE"
echo "Output: $OUTPUT_FILE"
echo ""

cd /home/vinay/universe/work/corroza
cargo run --release --bin play -- "$INPUT_FILE" "$OUTPUT_FILE"
