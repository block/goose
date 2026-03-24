#!/usr/bin/env bash
# Script to parse goose-bench results and check for failures

set -e

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <benchmark-result-json-file>"
  exit 1
fi

RESULT_FILE="$1"

if [ ! -f "$RESULT_FILE" ]; then
  echo "Error: Result file not found: $RESULT_FILE"
  exit 1
fi

# Extract basic information
PROVIDER=$(jq -r '.provider' "$RESULT_FILE")
START_TIME=$(jq -r '.start_time' "$RESULT_FILE")
SUITE_COUNT=$(jq '.suites | length' "$RESULT_FILE")

echo "Benchmark Results Analysis"
echo "-------------------------"
echo "Provider: $PROVIDER"
echo "Start Time: $START_TIME"
echo "Number of Suites: $SUITE_COUNT"
echo ""

# Initialize counters
TOTAL_EVALS=0
TOTAL_METRICS=0
FAILED_METRICS=0
PASSED_METRICS=0

# Process each suite
for i in $(seq 0 $((SUITE_COUNT-1))); do
  SUITE_NAME=$(jq -r ".suites[$i].name" "$RESULT_FILE")
  EVAL_COUNT=$(jq ".suites[$i].evaluations | length" "$RESULT_FILE")
  TOTAL_EVALS=$((TOTAL_EVALS + EVAL_COUNT))
  
  echo "Suite: $SUITE_NAME ($EVAL_COUNT evaluations)"
  
  # Process each evaluation in this suite
  for j in $(seq 0 $((EVAL_COUNT-1))); do
    EVAL_NAME=$(jq -r ".suites[$i].evaluations[$j].name" "$RESULT_FILE")
    METRIC_COUNT=$(jq ".suites[$i].evaluations[$j].metrics | length" "$RESULT_FILE")
    TOTAL_METRICS=$((TOTAL_METRICS + METRIC_COUNT))
    
    # Check for failures in this evaluation
    # This assumes metrics with names containing "success", "pass", or "correct" 
    # and boolean values of false indicate failures
    FAILURES=$(jq -r ".suites[$i].evaluations[$j].metrics[] | 
      select(
        (.[0] | test(\"success|pass|correct\"; \"i\")) and 
        (.[1] == false or .[1] == \"false\" or .[1] == 0 or .[1] == \"0\")
      ) | .[0]" "$RESULT_FILE" | wc -l | tr -d ' ')
    
    if [ "$FAILURES" -gt 0 ]; then
      FAILED_METRICS=$((FAILED_METRICS + FAILURES))
      echo "  ❌ $EVAL_NAME: $FAILURES failures detected"
      
      # Print the specific failing metrics
      FAILING_METRICS=$(jq -r ".suites[$i].evaluations[$j].metrics[] | 
        select(
          (.[0] | test(\"success|pass|correct\"; \"i\")) and 
          (.[1] == false or .[1] == \"false\" or .[1] == 0 or .[1] == \"0\")
        ) | \"    - \" + .[0]" "$RESULT_FILE")
      echo "$FAILING_METRICS"
    else
      PASSED_METRICS=$((PASSED_METRICS + METRIC_COUNT))
      echo "  ✅ $EVAL_NAME: All metrics passed"
    fi
  done
  echo ""
done

# Print summary
echo "Summary:"
echo "-------"
echo "Total Evaluations: $TOTAL_EVALS"
echo "Total Metrics: $TOTAL_METRICS"
echo "Passed Metrics: $PASSED_METRICS"
echo "Failed Metrics: $FAILED_METRICS"

# Set exit code based on failures
if [ "$FAILED_METRICS" -gt 0 ]; then
  echo "❌ Benchmark has $FAILED_METRICS failures"
  exit 1
else
  echo "✅ All metrics passed successfully"
  exit 0
fi