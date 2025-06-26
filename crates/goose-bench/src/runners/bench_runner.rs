use crate::bench_config::{BenchModel, BenchRunConfig};
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::EvaluationSuite;
use crate::runners::model_runner::ModelRunner;
use crate::utilities::{await_process_exits, parallel_bench_cmd};
use anyhow::Context;
use std::path::PathBuf;
use std::fs;
use std::env;

#[derive(Clone)]
pub struct BenchRunner {
    config: BenchRunConfig,
}

impl BenchRunner {
    pub fn new(config_path: PathBuf) -> anyhow::Result<BenchRunner> {
        let config = BenchRunConfig::from(config_path.clone())?;

        let resolved_output_dir = match &config.output_dir {
            Some(path) => {
                if !path.is_absolute() {
                    anyhow::bail!(
                         "Config Error in '{}': 'output_dir' must be an absolute path, but found relative path: {}",
                         config_path.display(),
                         path.display()
                     );
                }
                path.clone()
            }
            None => std::env::current_dir().context(
                "Failed to get current working directory to use as default output directory",
            )?,
        };

        BenchmarkWorkDir::init_experiment(resolved_output_dir)?;

        config.save("config.cfg".to_string());
        Ok(BenchRunner { config })
    }

    pub fn from(config: String) -> anyhow::Result<BenchRunner> {
        let config = BenchRunConfig::from_string(config)?;
        Ok(BenchRunner { config })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let (parallel_models, serial_models): (Vec<BenchModel>, Vec<BenchModel>) = self
            .config
            .models
            .clone()
            .into_iter()
            .partition(|model| model.parallel_safe);


        let mut parallel_handles = Vec::new();
        for (idx, model) in parallel_models.into_iter().enumerate() {
            self.config.models = vec![model];
            // Write config to temp file
            let config_path = env::current_dir()
                .context("Failed to get current directory")?
                .join(format!("temp_model_config_{}.json", idx));
            fs::write(&config_path, self.config.to_string()?)
                .context("Failed to write temporary config file")?;
            parallel_handles.push(parallel_bench_cmd("eval-model".to_string(), config_path.to_string_lossy().to_string(), Vec::new()));
        }

        for (idx, model) in serial_models.into_iter().enumerate() {
            self.config.models = vec![model];
            // Write config to temp file
            let config_path = env::current_dir()
                .context("Failed to get current directory")?
                .join(format!("temp_model_config_serial_{}.json", idx));
            fs::write(&config_path, self.config.to_string()?)
                .context("Failed to write temporary config file")?;
            ModelRunner::new(config_path.clone())?.run().await?;
        }

        await_process_exits(&mut parallel_handles, Vec::new());
        Ok(())
    }

    pub fn list_selectors(_config: Option<PathBuf>) -> anyhow::Result<()> {
        let selector_eval_counts = EvaluationSuite::available_selectors();
        let mut keys: Vec<_> = selector_eval_counts.keys().collect();
        keys.sort();
        let max_key_len = keys.iter().map(|k| k.len()).max().unwrap_or(0);
        println!(
            "selector {} => Eval Count",
            " ".repeat(max_key_len - "selector".len())
        );
        println!("{}", "-".repeat(max_key_len + 6));
        for selector in keys {
            println!(
                "{} {} => {}",
                selector,
                " ".repeat(max_key_len - selector.len()),
                selector_eval_counts.get(selector).unwrap()
            );
        }
        Ok(())
    }
}
