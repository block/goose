---
sidebar_position: 7
---
# Benchmarking with Goose

The Goose benchmarking system allows you to evaluate goose performance on complex tasks with one or more system configurations. This guide covers how to use the `goose bench` command to run benchmarks and analyze results.

## Configuration File

The benchmark configuration is specified in a JSON file with the following structure:

```json
{
  "models": [
    {
      "provider": "databricks",
      "name": "goose",
      "parallel_safe": true,
      "tool_shim": {
        "use_tool_shim": false,
        "tool_shim_model": null
      }
    }
  ],
  "evals": [
    {
      "selector": "core",
      "post_process_cmd": null,
      "parallel_safe": true
    }
  ],
  "include_dirs": [],
  "repeat": 2,
  "run_id": null,
  "eval_result_filename": "eval-results.json",
  "run_summary_filename": "run-results-summary.json",
  "env_file": null
}
```

### Configuration Options

#### Models Section
Each model entry in the `models` array specifies:

- `provider`: The model provider (e.g., "databricks")
- `name`: Model identifier
- `parallel_safe`: Whether the model can be run in parallel
- `tool_shim`: Optional configuration for tool shimming
  - `use_tool_shim`: Enable/disable tool shimming
  - `tool_shim_model`: Optional model to use for tool shimming

#### Evals Section
Each evaluation entry in the `evals` array specifies:

- `selector`: The evaluation suite to run (e.g., "core")
- `post_process_cmd`: Optional path to a post-processing script
- `parallel_safe`: Whether the evaluation can run in parallel

#### General Options

- `include_dirs`: Additional directories to include in the evaluation
- `repeat`: Number of times to repeat each evaluation
- `run_id`: Optional identifier for the benchmark run
- `eval_result_filename`: Name of the evaluation results file
- `run_summary_filename`: Name of the summary results file
- `env_file`: Optional path to an environment file

## Running Benchmarks

### Quick Start

1. The benchmarking system includes several evaluation suites. Run the following to see a listing of every valid selector:
```bash
$> goose bench selectors
```

2. Create a basic configuration file:

```bash
$> goose bench init-config -n bench-config.json
$> cat bench-config.json
{
  "models": [
    {
      "provider": "databricks",
      "name": "goose",
      "parallel_safe": true
    }
  ],
  "evals": [
    {
      "selector": "core",
      "parallel_safe": true
    }
  ],
  "repeat": 1
}
...etc.
```

2. Run the benchmark:

```bash
goose bench run -c bench-config.json
```

### Customizing Evaluations

You can customize runs in several ways:

1. Using Post-Processing Commands after evaluation:
```json
{
  "evals": [
    {
      "selector": "core",
      "post_process_cmd": "/path/to/process-script.sh",
      "parallel_safe": true
    }
  ]
}
```

2. Including Additional Data:
```json
{
  "include_dirs": [
    "/path/to/custom/eval/data"
  ]
}
```

3. Setting Environment Variables:
```json
{
  "env_file": "/path/to/env-file"
}
```

## Output and Results

The benchmark generates two main output files:

1. `eval-results.json`: Contains detailed results from each evaluation, including:
   - Individual test case results
   - Model responses
   - Scoring metrics
   - Error logs

2. `run-results-summary.json`: A collection of all eval results across all suites.

### Debug Mode

For detailed logging, you can enable debug mode:

```bash
RUST_LOG=debug goose bench bench-config.json
```

## Advanced Usage

### Tool Shimming

Tool shimming allows you to use a non-tool-capable models by feeding results of model-A into a tool-capable model-B, for model-B to format into a valid MCP-compliant tool-call:
