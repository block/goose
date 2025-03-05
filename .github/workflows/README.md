# Goose Benchmark Workflow

This directory contains the GitHub Actions workflow for running Goose benchmarks across different provider-model pairs.

## Workflow: `run-benchmarks.yml`

This workflow runs the Goose benchmark suites for specified provider-model pairs, and reports the results.

### Inputs

The workflow accepts the following inputs:

- **provider_models**: Comma-separated list of provider:model pairs (e.g., `openai:gpt-4o,anthropic:claude-3-5-sonnet`)
  - Each pair should be formatted as `provider:model`
  - Multiple pairs are separated by commas
  - Example: `openai:gpt-4o,anthropic:claude-3-5-sonnet` specifies two pairs:
    1. Provider: openai, Model: gpt-4o
    2. Provider: anthropic, Model: claude-3-5-sonnet
- **suites**: Comma-separated list of benchmark suites to run (e.g., `core,small_models`)
- **debug_mode**: Boolean flag to use debug build instead of release build (default: false)

### How It Works

1. The workflow builds Goose from source (debug or release mode based on input)
2. For each provider-model pair:
   - Sets environment variables for that provider/model
   - Runs the specified benchmark suites
   - Saves the results as JSON
   - Analyzes the results for failures
3. Generates a summary report
4. Uploads all results as artifacts

### Result Analysis

The workflow uses the `scripts/run-benchmarks.sh` script to run benchmarks and analyze results. This script:

- Sets environment variables for each provider-model pair
- Runs the benchmarks with the specified suites
- Analyzes the results for failures
- Generates a comprehensive report

### Artifacts

After the workflow completes, the following artifacts are available:

- **benchmark-results**: Contains JSON files with raw benchmark results and analysis text files
- A summary report is also added to the GitHub Actions run summary

### Example Usage

To run the benchmark workflow:

1. Go to the Actions tab in the GitHub repository
2. Select the "Run Goose Benchmarks" workflow
3. Click "Run workflow"
4. Fill in the inputs:
   - provider_models: `openai:gpt-4o,anthropic:claude-3-5-sonnet`
   - suites: `core,small_models`
   - debug_mode: `false` (or `true` to use debug build)
5. Click "Run workflow"