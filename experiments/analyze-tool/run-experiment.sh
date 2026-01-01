#!/bin/bash
# Run analyze tool effectiveness experiment
#
# Usage: ./run-experiment.sh [question_number]
#   question_number: 1-4 (optional, runs all if not specified)
#
# Results are saved to ./results/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
GOOSE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
GOOSE_BIN="$GOOSE_DIR/target/debug/goose"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Ensure we have a fresh build
echo "Building goose..."
cd "$GOOSE_DIR" && cargo build -p goose-cli --quiet
if [ ! -f "$GOOSE_BIN" ]; then
    echo "Error: goose binary not found at $GOOSE_BIN"
    exit 1
fi

# Test questions
QUESTIONS=(
    "What are the main crates in this project and what does each do?"
    "Where is the Agent trait defined and what methods does it have?"
    "What calls the complete method on providers?"
    "How does a message flow from CLI input to LLM response?"
)

mkdir -p "$RESULTS_DIR"

run_single_experiment() {
    local recipe="$1"
    local question="$2"
    local question_num="$3"
    local condition="$4"
    local output_file="$RESULTS_DIR/${TIMESTAMP}_q${question_num}_${condition}.json"
    
    echo "Running: Q${question_num} - ${condition}"
    echo "  Question: ${question:0:60}..."
    echo "  Output: $output_file"
    
    cd "$GOOSE_DIR"
    "$GOOSE_BIN" run \
        --recipe "$recipe" \
        --output-format json \
        --params question="$question" \
        --params target_directory="$GOOSE_DIR" \
        > "$output_file" 2>&1 || true
    
    # Extract key metrics
    if [ -f "$output_file" ]; then
        echo "  Results:"
        jq -r '.metadata | "    Total tokens: \(.total_tokens // "N/A")\n    Input tokens: \(.input_tokens // "N/A")\n    Output tokens: \(.output_tokens // "N/A")"' "$output_file" 2>/dev/null || echo "    (Could not parse JSON)"
    fi
    echo ""
}

run_question() {
    local q_idx=$1
    local question="${QUESTIONS[$q_idx]}"
    local q_num=$((q_idx + 1))
    
    echo "=========================================="
    echo "Question $q_num: $question"
    echo "=========================================="
    echo ""
    
    # Run with analyze (developer extension)
    run_single_experiment \
        "$SCRIPT_DIR/with-analyze.yaml" \
        "$question" \
        "$q_num" \
        "with-analyze"
    
    # Run with map (develop extension)
    run_single_experiment \
        "$SCRIPT_DIR/with-map.yaml" \
        "$question" \
        "$q_num" \
        "with-map"
    
    # Run without analyze (baseline)
    run_single_experiment \
        "$SCRIPT_DIR/without-analyze.yaml" \
        "$question" \
        "$q_num" \
        "without-analyze"
}

# Main
echo "Analyze Tool Effectiveness Experiment"
echo "======================================"
echo "Timestamp: $TIMESTAMP"
echo "Target: $GOOSE_DIR"
echo ""

if [ -n "$1" ]; then
    # Run specific question
    q_idx=$(($1 - 1))
    if [ $q_idx -ge 0 ] && [ $q_idx -lt ${#QUESTIONS[@]} ]; then
        run_question $q_idx
    else
        echo "Error: Question number must be 1-${#QUESTIONS[@]}"
        exit 1
    fi
else
    # Run all questions
    for i in "${!QUESTIONS[@]}"; do
        run_question $i
    done
fi

echo "=========================================="
echo "Experiment complete!"
echo "Results saved to: $RESULTS_DIR"
echo ""
echo "To analyze results:"
echo "  ls -la $RESULTS_DIR/${TIMESTAMP}_*.json"
