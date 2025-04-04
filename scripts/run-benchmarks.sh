#!/usr/bin/env bash
# run-benchmarks.sh - Script to run goose benchmarks across multiple provider:model pairs

set -e

# Display usage information
function show_usage() {
  echo "Usage: $0 [options]"
  echo ""
  echo "Options:"
  echo "  -p, --provider-models    Comma-separated list of provider:model pairs (e.g., 'openai:gpt-4o,anthropic:claude-3-5-sonnet')"
  echo "  -s, --suites             Comma-separated list of benchmark suites to run (e.g., 'core,small_models')"
  echo "  -o, --output-dir         Directory to store benchmark results (default: './benchmark-results')"
  echo "  -d, --debug              Use debug build instead of release build"
  echo "  -t, --toolshim           Enable toolshim mode by setting GOOSE_TOOLSHIM=1"
  echo "  -m, --toolshim-model     Set the toolshim model (sets GOOSE_TOOLSHIM_MODEL)"
  echo "  -h, --help               Show this help message"
  echo ""
  echo "Example:"
  echo "  $0 --provider-models 'openai:gpt-4o,anthropic:claude-3-5-sonnet' --suites 'core,small_models'"
}

# Parse command line arguments
PROVIDER_MODELS=""
SUITES=""
OUTPUT_DIR="./benchmark-results"
DEBUG_MODE=false
TOOLSHIM=false
TOOLSHIM_MODEL=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    -p|--provider-models)
      PROVIDER_MODELS="$2"
      shift 2
      ;;
    -s|--suites)
      SUITES="$2"
      shift 2
      ;;
    -o|--output-dir)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    -d|--debug)
      DEBUG_MODE=true
      shift
      ;;
    -t|--toolshim)
      TOOLSHIM=true
      shift
      ;;
    -m|--toolshim-model)
      TOOLSHIM_MODEL="$2"
      shift 2
      ;;
    -h|--help)
      show_usage
      exit 0
      ;;
    *)
      echo "Error: Unknown option: $1"
      show_usage
      exit 1
      ;;
  esac
done

# Validate required parameters
if [[ -z "$PROVIDER_MODELS" ]]; then
  echo "Error: Provider-model pairs must be specified"
  show_usage
  exit 1
fi

if [[ -z "$SUITES" ]]; then
  echo "Error: Benchmark suites must be specified"
  show_usage
  exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Create a results summary file
SUMMARY_FILE="$OUTPUT_DIR/summary.md"
echo "# Benchmark Results Summary" > "$SUMMARY_FILE"
echo "Run date: $(date)" >> "$SUMMARY_FILE"
echo "Suites: $SUITES" >> "$SUMMARY_FILE"
if [ "$DEBUG_MODE" = true ]; then
  echo "Mode: Debug" >> "$SUMMARY_FILE"
else
  echo "Mode: Release" >> "$SUMMARY_FILE"
fi
if [ "$TOOLSHIM" = true ]; then
  echo "Toolshim: Enabled" >> "$SUMMARY_FILE"
  if [[ -n "$TOOLSHIM_MODEL" ]]; then
    echo "Toolshim Model: $TOOLSHIM_MODEL" >> "$SUMMARY_FILE"
  fi
fi
echo "" >> "$SUMMARY_FILE"

# Determine which binary to use
GOOSE_CMD="goose"
if [ "$DEBUG_MODE" = true ]; then
  if [ -f "./target/debug/goose" ]; then
    GOOSE_CMD="./target/debug/goose"
    echo "Using debug binary: $GOOSE_CMD"
  else
    echo "Warning: Debug binary not found at ./target/debug/goose. Falling back to system-installed goose."
  fi
else
  if [ -f "./target/release/goose" ]; then
    GOOSE_CMD="./target/release/goose"
    echo "Using release binary: $GOOSE_CMD"
  else
    echo "Warning: Release binary not found at ./target/release/goose. Falling back to system-installed goose."
  fi
fi

# Parse provider:model pairs
PROVIDERS=()
MODELS=()

# Read provider:model pairs
IFS=',' read -ra PAIRS <<< "$PROVIDER_MODELS"
for pair in "${PAIRS[@]}"; do
  # Split by colon
  IFS=':' read -r provider model <<< "$pair"
  if [[ -n "$provider" && -n "$model" ]]; then
    PROVIDERS+=("$provider")
    MODELS+=("$model")
  else
    echo "Warning: Invalid provider:model pair: $pair. Skipping."
  fi
done

# Track overall success
OVERALL_SUCCESS=true
COUNT=${#PROVIDERS[@]}

echo "Running benchmarks for $COUNT provider:model pairs..."
echo "Benchmark suites: $SUITES"
echo ""

# Loop through each provider-model pair
for ((i=0; i<$COUNT; i++)); do
  provider="${PROVIDERS[i]}"
  model="${MODELS[i]}"
  
  echo "=========================================================="
  echo "Provider: $provider, Model: $model"
  echo "=========================================================="
  
  echo "## Provider: $provider, Model: $model" >> "$SUMMARY_FILE"
  
  # Set environment variables for this provider/model instead of using configure
  export GOOSE_PROVIDER="$provider"
  export GOOSE_MODEL="$model"
  
  # Set toolshim environment variables if enabled
  if [ "$TOOLSHIM" = true ]; then
    export GOOSE_TOOLSHIM=1
    if [[ -n "$TOOLSHIM_MODEL" ]]; then
      export GOOSE_TOOLSHIM_OLLAMA_MODEL="$TOOLSHIM_MODEL"
    fi
  fi
  
  # Run the benchmark and save results to JSON
  echo "Running benchmark for $provider/$model with suites: $SUITES"
  OUTPUT_FILE="$OUTPUT_DIR/${provider}-${model}.json"
  ANALYSIS_FILE="$OUTPUT_DIR/${provider}-${model}-analysis.txt"
  
  if $GOOSE_CMD bench --suites "$SUITES" --output "$OUTPUT_FILE" --format json; then
    echo "✅ Benchmark completed successfully" | tee -a "$SUMMARY_FILE"
    
    # Parse the JSON to check for failures
    if [ -f "$OUTPUT_FILE" ]; then
      # Check if jq is installed
      if ! command -v jq &> /dev/null; then
        echo "Warning: jq not found. Cannot parse JSON results."
        echo "⚠️ Could not parse results (jq not installed)" >> "$SUMMARY_FILE"
      else
        # Basic validation of the JSON file
        if jq empty "$OUTPUT_FILE" 2>/dev/null; then
          # Extract basic information
          PROVIDER_NAME=$(jq -r '.provider' "$OUTPUT_FILE")
          START_TIME=$(jq -r '.start_time' "$OUTPUT_FILE")
          SUITE_COUNT=$(jq '.suites | length' "$OUTPUT_FILE")
          
          echo "Benchmark Results Analysis" > "$ANALYSIS_FILE"
          echo "-------------------------" >> "$ANALYSIS_FILE"
          echo "Provider: $PROVIDER_NAME" >> "$ANALYSIS_FILE"
          echo "Start Time: $START_TIME" >> "$ANALYSIS_FILE"
          echo "Number of Suites: $SUITE_COUNT" >> "$ANALYSIS_FILE"
          echo "" >> "$ANALYSIS_FILE"
          
          # Initialize counters
          TOTAL_EVALS=0
          TOTAL_METRICS=0
          FAILED_METRICS=0
          PASSED_METRICS=0
          OTHER_METRICS=0
          TOTAL_ERRORS=0
          
          # Process each suite
          for j in $(seq 0 $((SUITE_COUNT-1))); do
            SUITE_NAME=$(jq -r ".suites[$j].name" "$OUTPUT_FILE")
            EVAL_COUNT=$(jq ".suites[$j].evaluations | length" "$OUTPUT_FILE")
            TOTAL_EVALS=$((TOTAL_EVALS + EVAL_COUNT))
            
            echo "Suite: $SUITE_NAME ($EVAL_COUNT evaluations)" >> "$ANALYSIS_FILE"
            
            # Process each evaluation in this suite
            for k in $(seq 0 $((EVAL_COUNT-1))); do
              EVAL_NAME=$(jq -r ".suites[$j].evaluations[$k].name" "$OUTPUT_FILE")
              METRIC_COUNT=$(jq ".suites[$j].evaluations[$k].metrics | length" "$OUTPUT_FILE")
              TOTAL_METRICS=$((TOTAL_METRICS + METRIC_COUNT))
              
              # Check for errors in this evaluation
              ERROR_COUNT=$(jq ".suites[$j].evaluations[$k].errors | length" "$OUTPUT_FILE")
              TOTAL_ERRORS=$((TOTAL_ERRORS + ERROR_COUNT))
              
              # Count boolean metrics (passed and failed)
              BOOLEAN_COUNT=$(jq -r ".suites[$j].evaluations[$k].metrics[] | 
                select(.[1].Boolean != null) | .[0]" "$OUTPUT_FILE" | wc -l | tr -d ' ')
              
              # Count failed boolean metrics
              FAILURES=$(jq -r ".suites[$j].evaluations[$k].metrics[] | 
                select(
                  .[1].Boolean == false or .[1].Boolean == \"false\" or .[1].Boolean == 0 or .[1].Boolean == \"0\"
                ) | .[0]" "$OUTPUT_FILE" | wc -l | tr -d ' ')
              
              # Count passed boolean metrics
              PASSES=$((BOOLEAN_COUNT - FAILURES))
              
              # Count non-boolean metrics
              NON_BOOLEAN=$((METRIC_COUNT - BOOLEAN_COUNT))
              
              # Update global counters
              FAILED_METRICS=$((FAILED_METRICS + FAILURES))
              PASSED_METRICS=$((PASSED_METRICS + PASSES))
              OTHER_METRICS=$((OTHER_METRICS + NON_BOOLEAN))
              
              if [ "$FAILURES" -gt 0 ] || [ "$ERROR_COUNT" -gt 0 ]; then
                echo "  ❌ $EVAL_NAME:" >> "$ANALYSIS_FILE"
                
                if [ "$FAILURES" -gt 0 ]; then
                  echo "    - $FAILURES metric failures detected" >> "$ANALYSIS_FILE"
                  # Print the specific failing metrics
                  FAILING_METRICS=$(jq -r ".suites[$j].evaluations[$k].metrics[] | 
                    select(
                    .[1].Boolean == false or .[1].Boolean == \"false\" or .[1].Boolean == 0 or .[1].Boolean == \"0\"
                  ) | .[0]" "$OUTPUT_FILE")
                  echo "    Failed metrics:" >> "$ANALYSIS_FILE"
                  echo "$FAILING_METRICS" | sed 's/^/      - /' >> "$ANALYSIS_FILE"
                fi
                
                if [ "$ERROR_COUNT" -gt 0 ]; then
                  echo "    - $ERROR_COUNT errors detected" >> "$ANALYSIS_FILE"
                  # Print the errors
                  jq -r ".suites[$j].evaluations[$k].errors[] | \"      [\(.level)] \(.message)\"" "$OUTPUT_FILE" >> "$ANALYSIS_FILE"
                fi
              else
                # This line is no longer needed since we count passes/fails/others individually
                echo "  ✅ $EVAL_NAME: All metrics passed, no errors" >> "$ANALYSIS_FILE"
              fi
            done
            echo "" >> "$ANALYSIS_FILE"
          done
          
          # Print summary
          echo "Summary:" >> "$ANALYSIS_FILE"
          echo "-------" >> "$ANALYSIS_FILE"
          echo "Total Evaluations: $TOTAL_EVALS" >> "$ANALYSIS_FILE"
          echo "Total Metrics: $TOTAL_METRICS" >> "$ANALYSIS_FILE"
          echo "Passed Metrics: $PASSED_METRICS" >> "$ANALYSIS_FILE"
          echo "Failed Metrics: $FAILED_METRICS" >> "$ANALYSIS_FILE"
          echo "Other Metrics: $OTHER_METRICS" >> "$ANALYSIS_FILE"
          echo "Total Errors: $TOTAL_ERRORS" >> "$ANALYSIS_FILE"
          
          # Verification of metrics counting
          COUNTED_METRICS=$((PASSED_METRICS + FAILED_METRICS + OTHER_METRICS))
          if [ "$COUNTED_METRICS" -ne "$TOTAL_METRICS" ]; then
            echo "⚠️ Metrics counting discrepancy: $COUNTED_METRICS counted vs $TOTAL_METRICS total" >> "$ANALYSIS_FILE"
          fi
          
          # Determine success/failure
          if [ "$FAILED_METRICS" -gt 0 ] || [ "$TOTAL_ERRORS" -gt 0 ]; then
            if [ "$FAILED_METRICS" -gt 0 ]; then
              echo "❌ Benchmark has $FAILED_METRICS failed metrics" >> "$ANALYSIS_FILE"
            fi
            if [ "$TOTAL_ERRORS" -gt 0 ]; then
              echo "❌ Benchmark has $TOTAL_ERRORS errors" >> "$ANALYSIS_FILE"
            fi
            echo "❌ Tests failed for $provider/$model" | tee -a "$SUMMARY_FILE"
            cat "$ANALYSIS_FILE" >> "$SUMMARY_FILE"
            OVERALL_SUCCESS=false
          else
            echo "✅ All metrics passed successfully, no errors" >> "$ANALYSIS_FILE"
            echo "✅ All tests passed for $provider/$model" | tee -a "$SUMMARY_FILE"
            cat "$ANALYSIS_FILE" >> "$SUMMARY_FILE"
          fi
        else
          echo "❌ Invalid JSON in benchmark output" | tee -a "$SUMMARY_FILE"
          OVERALL_SUCCESS=false
        fi
      fi
    else
      echo "❌ Benchmark output file not found" | tee -a "$SUMMARY_FILE"
      OVERALL_SUCCESS=false
    fi
  else
    echo "❌ Benchmark failed to run" | tee -a "$SUMMARY_FILE"
    OVERALL_SUCCESS=false
  fi
  
  echo "" >> "$SUMMARY_FILE"
  echo ""
done

echo "=========================================================="
echo "Benchmark run completed"
echo "Results saved to: $OUTPUT_DIR"
echo "Summary file: $SUMMARY_FILE"

# Output final status
if [ "$OVERALL_SUCCESS" = false ]; then
  echo "❌ Some benchmarks failed. Check the summary for details."
  exit 1
else
  echo "✅ All benchmarks completed successfully."
  exit 0
fi