use anyhow::{bail, ensure, Context, Result};
use std::path::PathBuf;
use tracing;

pub struct MetricAggregator;

impl MetricAggregator {
    /// Generate leaderboard and aggregated metrics CSV files from benchmark directory
    pub fn generate_csv_from_benchmark_dir(benchmark_dir: &PathBuf) -> Result<()> {
        use std::process::Command;

        // Step 1: Run prepare_aggregate_metrics.py to create aggregate_metrics.csv files
        let prepare_script_path = std::env::current_dir()
            .context("Failed to get current working directory")?
            .join("scripts")
            .join("bench-postprocess-scripts")
            .join("prepare_aggregate_metrics.py");

        ensure!(
            prepare_script_path.exists(),
            "Prepare script not found: {}",
            prepare_script_path.display()
        );

        tracing::info!(
            "Preparing aggregate metrics from benchmark directory: {}",
            benchmark_dir.display()
        );

        let output = Command::new(&prepare_script_path)
            .arg("--benchmark-dir")
            .arg(benchmark_dir)
            .output()
            .context("Failed to execute prepare_aggregate_metrics.py script")?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to prepare aggregate metrics: {}", error_message);
        }

        let success_message = String::from_utf8_lossy(&output.stdout);
        tracing::info!("{}", success_message);

        // Step 2: Run generate_leaderboard.py to create the final leaderboard
        let leaderboard_script_path = std::env::current_dir()
            .context("Failed to get current working directory")?
            .join("scripts")
            .join("bench-postprocess-scripts")
            .join("generate_leaderboard.py");

        ensure!(
            leaderboard_script_path.exists(),
            "Leaderboard script not found: {}",
            leaderboard_script_path.display()
        );

        tracing::info!(
            "Generating leaderboard from benchmark directory: {}",
            benchmark_dir.display()
        );

        let output = Command::new(&leaderboard_script_path)
            .arg("--benchmark-dir")
            .arg(benchmark_dir)
            .arg("--leaderboard-output")
            .arg("leaderboard.csv")
            .arg("--union-output")
            .arg("all_metrics.csv")
            .output()
            .context("Failed to execute generate_leaderboard.py script")?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to generate leaderboard: {}", error_message);
        }

        let success_message = String::from_utf8_lossy(&output.stdout);
        tracing::info!("{}", success_message);
        Ok(())
    }
}
