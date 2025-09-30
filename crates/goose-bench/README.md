# goose Benchmarking Framework

The `goose-bench` crate provides a framework for benchmarking and evaluating LLM models with the goose framework. This tool helps quantify model performance across various tasks and generate structured reports.

## Features

- Run benchmark suites across multiple LLM models
- Execute evaluations in parallel when supported
- Generate structured JSON and CSV reports
- Process evaluation results with custom scripts
- Calculate aggregate metrics across evaluations
- Support for tool-shim evaluation
- Generate leaderboards and comparative metrics

## Prerequisites

- **Python Environment**: The `generate-leaderboard` command executes Python scripts and requires a valid Python environment with necessary dependencies (pandas, etc.)
- **OpenAI API Key**: For evaluations using LLM-as-judge (like `blog_summary` and `restaurant_research`), you must have an `OPENAI_API_KEY` environment variable set, as the judge uses the OpenAI GPT-4o model

## Benchmark Workflow

Running benchmarks is a two-step process:

### Step 1: Run Benchmarks

First, run the benchmark evaluations with your configuration:

```bash
goose bench run --config /path/to/your-config.json
```

This will execute all evaluations for all models specified in your configuration and create a benchmark directory with results.

### Step 2: Generate Leaderboard

After the benchmarks complete, generate the leaderboard and aggregated metrics:

```bash
goose bench generate-leaderboard --benchmark-dir /path/to/benchmark-output-directory
```

The benchmark directory path will be shown in the output of the previous command, typically in the format `benchmark-YYYY-MM-DD-HH:MM:SS`.

**Note**: This command requires a valid Python environment as it executes Python scripts for data aggregation and leaderboard generation.

## Configuration

Benchmark configuration is provided through a JSON file. Here's a sample configuration file (leaderboard-config.json) that you can use as a template:

```json
{
  "models": [
    {
      "provider": "databricks",
      "name": "gpt-4-1-mini",
      "parallel_safe": true,
      "tool_shim": {
        "use_tool_shim": false,
        "tool_shim_model": null
      }
    },
    {
      "provider": "databricks",
      "name": "claude-sonnet-4",
      "parallel_safe": true,
      "tool_shim": null
    },
    {
      "provider": "databricks",
      "name": "gpt-4o",
      "parallel_safe": true,
      "tool_shim": null
    }
  ],
  "evals": [
    {
      "selector": "core:developer",
      "post_process_cmd": null,
      "parallel_safe": true
    },
    {
      "selector": "core:developer_search_replace",
      "post_process_cmd": null,
      "parallel_safe": true
    },
    {
      "selector": "vibes:blog_summary",
      "post_process_cmd": "/Users/ahau/Development/goose-1.0/goose/scripts/bench-postprocess-scripts/llm-judges/run_vibes_judge.sh",
      "parallel_safe": true
    },
    {
      "selector": "vibes:restaurant_research",
      "post_process_cmd": "/Users/ahau/Development/goose-1.0/goose/scripts/bench-postprocess-scripts/llm-judges/run_vibes_judge.sh",
      "parallel_safe": true
    }
  ],
  "include_dirs": [],
  "repeat": 3,
  "run_id": null,
  "output_dir": "/path/to/output/directory",
  "eval_result_filename": "eval-results.json",
  "run_summary_filename": "run-results-summary.json",
  "env_file": "/path/to/.goosebench.env"
}
```

## Configuration Options

### Models

- `provider`: The LLM provider (e.g., "databricks", "openai")
- `name`: The model name
- `parallel_safe`: Whether the model can be run in parallel
- `tool_shim`: Configuration for tool-shim support
  - `use_tool_shim`: Whether to use tool-shim
  - `tool_shim_model`: Optional custom model for tool-shim

### Evaluations

- `selector`: The evaluation selector in format `suite:evaluation`
- `post_process_cmd`: Optional path to a post-processing script
- `parallel_safe`: Whether the evaluation can be run in parallel

### Global Configuration

- `include_dirs`: Additional directories to include in the benchmark environment
- `repeat`: Number of times to repeat evaluations (for statistical significance)
- `run_id`: Optional identifier for the run (defaults to timestamp)
- `output_dir`: Directory to store benchmark results (must be absolute path)
- `eval_result_filename`: Filename for individual evaluation results
- `run_summary_filename`: Filename for run summary
- `env_file`: Optional path to environment variables file

## Environment Variables

You can provide environment variables through the `env_file` configuration option. This is useful for provider API keys and other sensitive information. Example `.goosebench.env` file:

```bash
OPENAI_API_KEY=your_openai_api_key_here
DATABRICKS_TOKEN=your_databricks_token_here
# Add other environment variables as needed
```

**Important**: For evaluations that use LLM-as-judge (like `blog_summary` and `restaurant_research`), you must set `OPENAI_API_KEY` as the judging system uses OpenAI's GPT-4o model.

## Post-Processing

You can specify post-processing commands for evaluations, which will be executed after each evaluation completes. The command receives the path to the evaluation results file as its first argument.

For example, the `run_vibes_judge.sh` script processes outputs from the `blog_summary` and `restaurant_research` evaluations, using LLM-based judging to assign scores.

## Output Structure

Results are organized in a directory structure that follows this pattern:

```
{benchmark_dir}/
├── config.cfg                           # Configuration used for the benchmark
├── {provider}-{model}/
│   ├── eval-results/
│   │   └── aggregate_metrics.csv        # Aggregated metrics for this model
│   └── run-{run_id}/
│       ├── {suite}/
│       │   └── {evaluation}/
│       │       ├── eval-results.json    # Individual evaluation results
│       │       ├── {eval_name}.jsonl    # Session logs
│       │       └── work_dir.json        # Info about evaluation working dir
│       └── run-results-summary.json     # Summary of all evaluations in this run
├── leaderboard.csv                      # Final leaderboard comparing all models
└── all_metrics.csv                      # Union of all metrics across all models
```

### Output Files Explained

#### Per-Model Files

- **`eval-results/aggregate_metrics.csv`**: Contains aggregated metrics for each evaluation, averaged across all runs. Includes metrics like `score_mean`, `total_tokens_mean`, `prompt_execution_time_seconds_mean`, etc.

#### Global Output Files

- **`leaderboard.csv`**: Final leaderboard ranking all models by their average performance across evaluations. Contains columns like:
  - `provider`, `model_name`: Model identification
  - `avg_score_mean`: Average score across all evaluations
  - `avg_prompt_execution_time_seconds_mean`: Average execution time
  - `avg_total_tool_calls_mean`: Average number of tool calls
  - `avg_total_tokens_mean`: Average token usage

- **`all_metrics.csv`**: Comprehensive dataset containing detailed metrics for every model-evaluation combination. This is a union of all individual model metrics, useful for detailed analysis and custom reporting.

Each model gets its own directory, containing run results and aggregated CSV files for analysis. The `generate-leaderboard` command processes all individual evaluation results and creates the comparative metrics files.

## Error Handling and Troubleshooting

**Important**: The current version of goose-bench does not have robust error handling for common issues that can occur during evaluation runs, such as:

- Rate limiting from inference providers
- Network timeouts or connection errors
- Provider API errors that cause early session termination
- Resource exhaustion or memory issues

### Checking for Failed Evaluations

After running benchmarks, you should inspect the generated metrics files to identify any evaluations that may have failed or terminated early:

1. **Check the `aggregate_metrics.csv` files** in each model's `eval-results/` directory for:
   - Missing evaluations (fewer rows than expected)
   - Unusually low scores or metrics
   - Zero or near-zero execution times
   - Missing or NaN values

2. **Look for `server_error_mean` column** in the aggregate metrics - values greater than 0 indicate server errors occurred during evaluation

3. **Review session logs** (`.jsonl` files) in individual evaluation directories for error messages like:
   - "Server error"
   - "Rate limit exceeded" 
   - "TEMPORARILY_UNAVAILABLE"
   - Unexpected session terminations

### Re-running Failed Evaluations

If you identify failed evaluations, you may need to:

1. **Adjust rate limiting**: Add delays between requests or reduce parallel execution
2. **Update environment variables**: Ensure API keys and tokens are valid
3. **Re-run specific model/evaluation combinations**: Create a new config with only the failed combinations
4. **Check provider status**: Verify the inference provider is operational

Example of creating a config to re-run failed evaluations:

```json
{
  "models": [
    {
      "provider": "databricks",
      "name": "claude-sonnet-4",
      "parallel_safe": false
    }
  ],
  "evals": [
    {
      "selector": "vibes:blog_summary",
      "post_process_cmd": "/path/to/scripts/bench-postprocess-scripts/llm-judges/run_vibes_judge.sh",
      "parallel_safe": false
    }
  ],
  "repeat": 1,
  "output_dir": "/path/to/retry-benchmark"
}
```

We recommend monitoring evaluation progress and checking for errors regularly, especially when running large benchmark suites across multiple models.

## Available Commands

### List Evaluations
```bash
goose bench selectors --config /path/to/config.json
```

### Generate Initial Config
```bash
goose bench init-config --name my-benchmark-config.json
```

### Run Benchmarks
```bash
goose bench run --config /path/to/config.json
```

### Generate Leaderboard
```bash
goose bench generate-leaderboard --benchmark-dir /path/to/benchmark-output
```
