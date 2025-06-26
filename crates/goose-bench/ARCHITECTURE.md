# Goose Benchmark Architecture

This document provides an overview of the architecture and design patterns used in the Goose benchmark system.

## Overview

The Goose benchmark system is designed to evaluate the performance and capabilities of AI agents across different scenarios. The system is organized into "evaluation suites" that test specific aspects of agent functionality.

## Directory Structure

```
crates/goose-bench/
├── src/
│   ├── assets/                  # Input files used by benchmark tests
│   ├── eval_suites/            # Contains all evaluation suites
│   │   ├── core/               # Core functionality evaluations
│   │   ├── vibes/              # User experience evaluations
│   │   ├── evaluation.rs       # Defines the Evaluation trait
│   │   ├── factory.rs          # Registry for evaluations
│   │   ├── metrics.rs          # Metrics collection utilities
│   │   ├── mod.rs              # Module definitions
│   │   └── utils.rs            # Shared utilities
│   ├── runners/                # Test runners for evaluations
│   ├── bench_config.rs         # Configuration for benchmarks
│   ├── bench_session.rs        # Session management for tests
│   ├── bench_work_dir.rs       # Working directory management
│   └── lib.rs                  # Library entry point
└── Cargo.toml                  # Package dependencies
```

## Key Components

### Evaluation Trait

The `Evaluation` trait (defined in `evaluation.rs`) is the core interface that all benchmark evaluations must implement:

```rust
#[async_trait]
pub trait Evaluation: Send + Sync {
    async fn run(
        &self,
        agent: &mut BenchAgent,
        run_loc: &mut BenchmarkWorkDir,
    ) -> Result<Vec<(String, EvalMetricValue)>>;

    fn name(&self) -> &str;

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements::default()
    }
}
```

Each evaluation must implement:
- `run`: The main function that executes the test and returns metrics
- `name`: A unique identifier for the evaluation
- `required_extensions`: Specifies what extensions the evaluation needs

### Evaluation Suites

Evaluations are organized into suites, each focusing on a different aspect of agent capabilities:

1. **Core Suite**: Tests core functionality like tool use, memory, and developer capabilities
2. **Vibes Suite**: Tests user experience aspects like summarization, research, and creative tasks

### Registration System

Evaluations are registered using the `register_evaluation!` macro, which adds them to a global registry maintained by the `EvaluationSuite` struct. This allows evaluations to be discovered and executed by name.

### Metrics Collection

The `metrics.rs` file provides utilities for collecting common metrics:

- `collect_baseline_metrics`: Captures execution time, token usage, and tool call counts
- `used_tool`: Checks if a specific tool was used
- `metrics_hashmap_to_vec`: Converts metrics from a HashMap to a Vec

## Creating a New Evaluation

To create a new evaluation:

1. Create a new Rust file in the appropriate suite directory
2. Define a struct that will implement the Evaluation trait
3. Implement the required methods (run, name, required_extensions)
4. Register the evaluation using the `register_evaluation!` macro
5. Update the suite's mod.rs file to include your new evaluation

Example template:

```rust
use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{collect_baseline_metrics, metrics_hashmap_to_vec, write_response_to_file, EvalMetricValue, Evaluation, ExtensionRequirements};
use crate::register_evaluation;
use async_trait::async_trait;

pub struct MyNewEvaluation {}

impl MyNewEvaluation {
    pub fn new() -> Self {
        MyNewEvaluation {}
    }
    
    // Custom validation methods here
}

#[async_trait]
impl Evaluation for MyNewEvaluation {
    async fn run(
        &self,
        agent: &mut BenchAgent,
        run_loc: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvalMetricValue)>> {
        println!("MyNewEvaluation - run");

        // Collect baseline metrics with your prompt
        let (response, perf_metrics) = collect_baseline_metrics(
            agent,
            "Your prompt here".to_string()
        ).await;

        // Write response to file
        let response_text = write_response_to_file(&response, run_loc, "output_file.txt")?;

        // Prepare metrics
        let mut metrics = metrics_hashmap_to_vec(perf_metrics);
        
        // Add custom metrics
        metrics.push(("custom_metric".to_string(), EvalMetricValue::Boolean(true)));

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "my_new_evaluation"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["developer".to_string()],
            external: Vec::new(),
            remote: Vec::new(),
        }
    }
}

register_evaluation!(MyNewEvaluation);
```

## Running Evaluations

Evaluations are executed by the benchmark runner system, which:

1. Sets up the environment
2. Creates an agent instance
3. Runs the evaluation
4. Collects and reports metrics

The system supports running specific evaluations by name or running all evaluations in a suite.