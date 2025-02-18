#!/bin/bash

# NOTE: MacOS ships with Bash 3.2 by default, which does NOT support declare -A for associative arrays.
# This script uses standard Bash arrays to remain compatible.

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# Initialize error log array
ERROR_LOG=()

#---------------------------------------------------------------------------#
# EXTENSIONS
#---------------------------------------------------------------------------#
# We'll define each extension in an array of prompts. Then we define an array
# of extension names, so we can iterate over them.
#---------------------------------------------------------------------------#
EXTENSIONS=(developer computercontroller google_drive memory)

developer_prompts=(
  "List the contents of the current directory."
  "Create a new file called test.txt with the content 'Hello, World!'"
  "Read the contents of test.txt"
)

computercontroller_prompts=(
    "What are the headlines on hackernews? Organize the list into categories."
    "Make a ding sound"
)

google_drive_prompts=(
  "List the files in my Google Drive."
  "Search for documents containing 'meeting notes'"
)

memory_prompts=(
  "Save this fact: The capital of France is Paris."
  "What is the capital of France?"
)


#---------------------------------------------------------------------------#
# LOGGING FUNCTION
#---------------------------------------------------------------------------#
log_error() {
  local provider=$1
  local model=$2
  local extension=$3
  local error=$4
  ERROR_LOG+=("${RED}[ERROR]${NC} Provider: $provider, Model: $model, Extension: $extension\n$error\n")
}

#---------------------------------------------------------------------------#
# MAIN TEST FUNCTION
#---------------------------------------------------------------------------#
run_test() {
  local provider=$1
  local model=$2
  local extension=$3
  local prompt=$4
  local timeout_seconds=30

  echo -e "${YELLOW}Testing:${NC} $provider/$model with $extension"
  echo -e "${YELLOW}Prompt:${NC} $prompt"

  local temp_file
  temp_file="$(mktemp)"
  echo "$prompt" > "$temp_file"

  # Run goose with timeout
  timeout $timeout_seconds goose run \
    --with-builtin "$extension" \
    -t "$(cat "$temp_file")" 2>&1 | tee test_output.log

  # Check for errors
  if [ ${PIPESTATUS[0]} -ne 0 ]; then
    log_error "$provider" "$model" "$extension" "$(cat test_output.log)"
    echo -e "${RED}✗ Test failed${NC}"
  else
    echo -e "${GREEN}✓ Test passed${NC}"
  fi

  rm -f "$temp_file" test_output.log
}

#---------------------------------------------------------------------------#
# TESTING EXTENSION (ITERATING OVER PROMPTS)
#---------------------------------------------------------------------------#
test_extension() {
  local provider=$1
  local model=$2
  local extension=$3

  echo -e "\n${BOLD}Testing extension: $extension${NC}"

  # We'll build the array name dynamically, e.g. developer_prompts, memory_prompts, etc.
  # Then we retrieve that array's contents via indirect expansion.
  local arr_name="${extension}_prompts[@]"
  local prompts=("${!arr_name}")

  for prompt in "${prompts[@]}"; do
    run_test "$provider" "$model" "$extension" "$prompt"
    sleep 2  # brief pause
  done
}

#---------------------------------------------------------------------------#
# USAGE FUNCTION
#---------------------------------------------------------------------------#
usage() {
  echo "Usage: $0 [-p provider -m model[,model2,model3]...]..."
  echo "  -p provider : Provider to use"
  echo "  -m models   : Comma-separated list of models to use with the provider"
  echo "  -h         : Show this help message"
  echo ""
  echo "Examples:"
  echo "  $0                                    # Uses default: databricks/goose"
  echo "  $0 -p anthropic -m claude             # Single provider/model"
  echo "  $0 -p anthropic -m claude,claude2     # One provider, multiple models"
  echo "  $0 -p anthropic -m claude -p databricks -m goose  # Multiple providers"
  echo "  $0 -p anthropic -m claude,claude2 -p databricks -m goose,goose2  # Multiple of both"
  exit 1
}

#---------------------------------------------------------------------------#
# MAIN WORKFLOW
#---------------------------------------------------------------------------#
main() {
  # Arrays to store provider/model combinations
  declare -a provider_model_pairs=()
  local current_provider=""

  # Parse command line arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h)
        usage
        ;;
      -p)
        shift
        if [[ -z "$1" ]]; then
          echo "Error: -p requires a provider name"
          usage
        fi
        current_provider="$1"
        shift
        ;;
      -m)
        if [[ -z "$current_provider" ]]; then
          echo "Error: -m must follow a -p option"
          usage
        fi
        shift
        if [[ -z "$1" ]]; then
          echo "Error: -m requires at least one model name"
          usage
        fi
        # Split comma-separated models and create provider:model pairs
        IFS=',' read -ra models <<< "$1"
        for model in "${models[@]}"; do
          provider_model_pairs+=("$current_provider:$model")
        done
        shift
        ;;
      *)
        echo "Error: Unknown option $1"
        usage
        ;;
    esac
  done

  # If no providers/models specified, use defaults
  if [ ${#provider_model_pairs[@]} -eq 0 ]; then
    provider_model_pairs=("databricks:goose")
  fi

  echo -e "${BOLD}Starting Goose CLI Integration Tests${NC}"

  # Iterate through provider/model pairs
  for pair in "${provider_model_pairs[@]}"; do
    # Split the pair into provider and model
    IFS=':' read -r provider model <<< "$pair"
    
    echo -e "\n${BOLD}Testing provider: $provider${NC}"
    echo -e "${BOLD}Testing model: $model${NC}"

    # Now test each extension for this provider/model pair
    for extension in "${EXTENSIONS[@]}"; do
      test_extension "$provider" "$model" "$extension"
    done
  done

  # Print summary
  if [ ${#ERROR_LOG[@]} -eq 0 ]; then
    echo -e "\n${GREEN}All tests completed successfully!${NC}"
  else
    echo -e "\n${RED}Test Summary - Errors Found:${NC}"
    echo -e "================================"
    printf '%b\n' "${ERROR_LOG[@]}"
    exit 1
  fi
}

# Call main with all arguments
main "$@"