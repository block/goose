#!/bin/bash
# Wrapper script for LLM judge post-processing and final score calculation
# This script is called by the benchmark runner with the eval results file as an argument

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Get the eval results file path from the first argument
EVAL_RESULTS_FILE="$1"

# Extract the working directory from the eval results file path
WORKING_DIR="$(dirname "$EVAL_RESULTS_FILE")"

# Change to the working directory
cd "$WORKING_DIR"

# Determine the evaluation name from the eval-results.json
EVAL_NAME=$(python3 -c "import json; print(json.load(open('eval-results.json'))['name'])")

# Set the output file name and prompt file based on the evaluation
if [ "$EVAL_NAME" = "blog_summary" ]; then
    OUTPUT_FILE="blog_summary_output.txt"
    PROMPT_FILE="$SCRIPT_DIR/blog_summary_prompt.txt"
elif [ "$EVAL_NAME" = "restaurant_research" ]; then
    OUTPUT_FILE="restaurant_research_output.txt"
    PROMPT_FILE="$SCRIPT_DIR/restaurant_research_prompt.txt"
else
    echo "Error: Unknown evaluation name: $EVAL_NAME"
    exit 1
fi

# Run the LLM judge script with the appropriate arguments
python3 "$SCRIPT_DIR/llm_judge.py" "$OUTPUT_FILE" --prompt-file "$PROMPT_FILE"

# Check if LLM judge succeeded
if [ $? -ne 0 ]; then
    echo "Error: LLM judge failed"
    exit 1
fi

# Calculate the final score
python3 "$SCRIPT_DIR/calculate_final_scores_vibes.py" "$EVAL_NAME"

# Check if score calculation succeeded
if [ $? -ne 0 ]; then
    echo "Error: Final score calculation failed"
    exit 1
fi

echo "Successfully completed post-processing for $EVAL_NAME"
