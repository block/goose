#!/bin/bash

# Exit on error
set -e

# Trap for script interruption
trap 'echo "Script interrupted. Cleaning up..."; exit 1' INT TERM

# Function to check if a directory exists
check_dir() {
    if [ ! -d "$1" ]; then
        echo "Error: Directory '$1' does not exist"
        exit 1
    fi
}

# Function to run Python script with error handling
run_python_script() {
    local script=$1
    local args=$2
    echo "Running $script..."
    if ! PYTHONPATH=$PYTHONPATH:. python3 "$script" $args; then
        echo "Error: Failed to run $script"
        exit 1
    fi
}

# Default values
BENCHMARKS_DIR="$(pwd)"
OUTPUT_DIR="$BENCHMARKS_DIR/goosebench-evals-processed"

# Parse named arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --benchmarks-dir)
            BENCHMARKS_DIR="$2"
            shift 2
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        *)
            echo "Unknown argument: $1"
            echo "Usage: $0 [--benchmarks-dir DIR] [--output-dir DIR]"
            exit 1
            ;;
    esac
done

echo "Using benchmarks directory: $BENCHMARKS_DIR"
echo "Using output directory: $OUTPUT_DIR"

# Check if input directories exist
check_dir "$BENCHMARKS_DIR"

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR" || {
    echo "Error: Failed to create output directory"
    exit 1
}

# Run all analyses
run_python_script "$BENCHMARKS_DIR/scripts/goosebench_result_processing/run_all_analyses.py" \
    "--benchmarks-dir $BENCHMARKS_DIR --output-dir $OUTPUT_DIR"

# Check if analysis output exists
if [ ! -d "$OUTPUT_DIR" ] || [ -z "$(ls -A $OUTPUT_DIR)" ]; then
    echo "Error: No analysis results found in $OUTPUT_DIR/"
    exit 1
fi

# Generate leaderboard
run_python_script "$BENCHMARKS_DIR/scripts/goosebench_result_processing/generate_leaderboard.py" \
    "--input-dir $OUTPUT_DIR --output $OUTPUT_DIR/leaderboard.csv"

# Check if leaderboard was created
if [ ! -f "$OUTPUT_DIR/leaderboard.csv" ]; then
    echo "Error: Failed to generate leaderboard.csv"
    exit 1
fi

echo "Analysis complete! Results are in $OUTPUT_DIR/" 