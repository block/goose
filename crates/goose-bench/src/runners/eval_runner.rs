use crate::bench_config::{BenchEval, BenchModel, BenchRunConfig};
use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{EvaluationSuite, ExtensionRequirements};
use crate::reporting::EvaluationResult;
use crate::utilities::await_process_exits;
use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use mcp_core::Tool;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Clean up all mock_mcp_server processes
fn cleanup_mock_mcp_processes() {
    let _ = Command::new("pkill")
        .arg("-f")
        .arg("mock_mcp_server")
        .output();
    tracing::debug!("Cleaned up all mock_mcp_server processes");
}

// Configuration struct to reduce function parameter count
#[derive(Clone)]
struct RowProcessingConfig<'a> {
    dataset_config: &'a crate::bench_config::BenchDataset,
    selector: String,
}

// Stateless functions for parallel processing
async fn process_single_dataset_row_stateless<F, Fut>(
    idx: usize,
    row: serde_json::Value,
    agent_generator: F,
    config: RowProcessingConfig<'_>,
) -> serde_json::Value
where
    F: Fn(ExtensionRequirements, String, bool, Option<HashMap<String, Vec<Tool>>>) -> Fut,
    Fut: Future<Output = BenchAgent> + Send,
{
    let start_time = std::time::Instant::now();
    let (result, timed_out) = async {
        let now_stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("Failed to get current timestamp")?
            .as_nanos();

        let session_id = format!("{}-{}-row{}", config.selector, now_stamp, idx);

        // Parse extensions for this specific row
        let extensions = parse_extensions_stateless(&row, &config.dataset_config.tools_column)
            .unwrap_or_default();

        // Create a fresh agent for each row
        tracing::debug!(row_idx = idx, "Creating fresh agent for row");
        
        let mut agent = agent_generator(
            ExtensionRequirements::default(),
            session_id,
            true,
            Some(extensions),
        )
        .await;

        if let Some(system_prompt) =
            get_dataset_system_prompt_stateless(&row, &config.dataset_config.system_prompt_column)
        {
            agent.session.override_system_prompt(system_prompt).await;
        }

        let prompts = match row.get(&config.dataset_config.prompt_column) {
            Some(value) => {
                if let Some(array) = value.as_array() {
                    // Handle array case - new format
                    let prompt_strs: Result<Vec<&str>> = array
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .ok_or_else(|| anyhow::anyhow!("Prompt array must contain strings"))
                        })
                        .collect();
                    prompt_strs?.into_iter().map(|s| s.to_string()).collect()
                } else if let Some(single_str) = value.as_str() {
                    // Handle single string case - backward compatibility
                    vec![
                        single_str.to_string(),
                        "Please immediately do whatever you think is most sensible.".to_string(),
                    ]
                } else {
                    bail!("Prompt field must be either a string or an array of strings");
                }
            }
            None => bail!("Missing prompt field"),
        };

        if prompts.is_empty() {
            bail!("Prompt array must contain at least one string");
        }

        // Row-level rate limiting is now handled by the global rate limiter

        // Add timeout to prevent hanging on LLM clarifying questions
        let prompt_timeout = tokio::time::Duration::from_secs(900);
        let result = match tokio::time::timeout(prompt_timeout, async {
            agent.prompt_multi_turn(prompts).await
        })
        .await
        {
            Ok(result) => {
                // Log conversation results
                result
            }
            Err(_timeout) => Err(anyhow::anyhow!(
                "Prompt timeout - likely LLM asking for clarification"
            )),
        };

        // Clean up agent and its extension processes
        if let Err(e) = agent.shutdown().await {
            tracing::warn!(row_idx = idx, error = %e, "Failed to shutdown agent cleanly");
        } else {
            tracing::debug!(row_idx = idx, "Agent cleaned up successfully");
        }

        Ok::<(Result<Vec<goose::message::Message>>, bool), anyhow::Error>((result, false))
    }
    .await
    .unwrap_or_else(|e: anyhow::Error| (Err(e), false));

    let duration_ms = start_time.elapsed().as_millis() as u64;
    create_dataset_result_row_stateless(row, result, idx, timed_out, duration_ms)
}

fn parse_extensions_stateless(
    row: &serde_json::Value,
    tools_column: &Option<String>,
) -> Result<HashMap<String, Vec<Tool>>> {
    let mut extensions = HashMap::new();
    if let Some(tools_col) = tools_column {
        if let Some(tools_str) = row.get(tools_col).and_then(|v| v.as_str()) {
            extensions = serde_json::from_str(tools_str)?;
        }
    }
    Ok(extensions)
}

fn get_dataset_system_prompt_stateless(
    row: &serde_json::Value,
    system_prompt_column: &Option<String>,
) -> Option<String> {
    system_prompt_column
        .as_ref()
        .and_then(|col| row.get(col))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn create_dataset_result_row_stateless(
    mut row: serde_json::Value,
    result: Result<Vec<goose::message::Message>>,
    idx: usize,
    timed_out: bool,
    duration_ms: u64,
) -> serde_json::Value {
    let obj = row.as_object_mut().unwrap();

    match result {
        Ok(messages) => {
            obj.insert(
                "output".to_string(),
                serde_json::to_value(messages).unwrap(),
            );
            obj.insert("error".to_string(), serde_json::Value::Null);
            obj.insert("success".to_string(), serde_json::Value::Bool(!timed_out));
            // False if timed out
        }
        Err(e) => {
            obj.insert("output".to_string(), serde_json::Value::Null);
            obj.insert(
                "error".to_string(),
                serde_json::Value::String(e.to_string()),
            );
            obj.insert("success".to_string(), serde_json::Value::Bool(false));
        }
    }

    obj.insert(
        "row_index".to_string(),
        serde_json::Value::Number(serde_json::Number::from(idx)),
    );
    obj.insert("timed_out".to_string(), serde_json::Value::Bool(timed_out));
    obj.insert(
        "duration_ms".to_string(),
        serde_json::Value::Number(serde_json::Number::from(duration_ms)),
    );

    if timed_out {
        obj.insert(
            "completion_status".to_string(),
            serde_json::Value::String("partial".to_string()),
        );
    } else {
        obj.insert(
            "completion_status".to_string(),
            serde_json::Value::String("complete".to_string()),
        );
    }

    row
}

#[derive(Clone)]
pub struct EvalRunner {
    config: BenchRunConfig,
}

impl EvalRunner {
    pub fn new(config_path: PathBuf) -> Result<EvalRunner> {
        tracing::info!("Loading config from: {}", config_path.display());
        let config = BenchRunConfig::from(config_path.clone())
            .context("Failed to parse evaluation configuration from file")?;
        tracing::info!("Config loaded successfully");
        Ok(EvalRunner { config })
    }
    
    pub fn from(config: String) -> Result<EvalRunner> {
        let config = BenchRunConfig::from_string(config)
            .context("Failed to parse evaluation configuration")?;
        Ok(EvalRunner { config })
    }

    fn create_work_dir(&self, config: &BenchRunConfig) -> Result<BenchmarkWorkDir> {
        let goose_model = config
            .models
            .first()
            .context("No model specified in configuration")?;
        let model_name = goose_model.name.clone();
        let provider_name = goose_model.provider.clone();

        // construct work-dir name to have a shim component only if shim configured to be used
        let work_dir_name_shim = {
            let mut shim_name = "".to_string();
            if let Some(shim_opt) = &goose_model.tool_shim {
                if shim_opt.use_tool_shim {
                    let shim_model = if let Some(shim_model) = &shim_opt.tool_shim_model {
                        shim_model.clone()
                    } else {
                        "default".to_string()
                    };
                    shim_name = format!("-{}-shim-model", shim_model);
                }
            }
            shim_name
        };

        let include_dir = config.include_dirs.clone();
        let work_dir_name = format!("{}-{}{}", provider_name, model_name, work_dir_name_shim);
        let work_dir = BenchmarkWorkDir::new(work_dir_name, include_dir);
        Ok(work_dir)
    }

    /// Helper method to get the dataset config from the global dataset
    fn get_dataset_config(&self) -> Option<&crate::bench_config::BenchDatasetConfig> {
        self.config.dataset.as_ref().and_then(|d| d.config.as_ref())
    }

    pub async fn run<F, Fut>(&mut self, agent_generator: F) -> Result<()>
    where
        F: Fn(ExtensionRequirements, String, bool, Option<HashMap<String, Vec<Tool>>>) -> Fut
            + Clone,
        Fut: Future<Output = BenchAgent> + Send,
    {
        tracing::info!("Starting eval runner");
        // Find the first active evaluation
        let bench_eval = self
            .config
            .evals
            .iter()
            .find(|eval| eval.active)
            .context("No active evaluations found in configuration")?;
        
        println!("Running evaluation: {}", bench_eval.selector);
        
        if self.config.dataset.is_some() {
            return self.run_dataset(agent_generator).await;
        }

        let mut work_dir = self
            .create_work_dir(&self.config)
            .context("Failed to create evaluation work directory")?;

        let run_id = &self
            .config
            .run_id
            .clone()
            .unwrap_or_else(|| "run-0".to_string());
        let run_id = format!("run-{}", run_id.clone());

        // create entire dir subtree for eval and cd into dir for running eval
        work_dir.set_eval(&bench_eval.selector, run_id);

        if let Some(eval) = EvaluationSuite::from(&bench_eval.selector) {
            let now_stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("Failed to get current timestamp")?
                .as_nanos();

            let session_id = format!("{}-{}", bench_eval.selector.clone(), now_stamp);
            let mut agent =
                agent_generator(eval.required_extensions(), session_id, false, None).await;

            let mut result = EvaluationResult::new(eval.name().to_string());

            if let Ok(metrics) = eval.run(&mut agent, &mut work_dir).await {
                for (name, metric) in metrics {
                    result.add_metric(name, metric);
                }
            }

            // Add any errors that occurred
            let errors = agent.get_errors().await;
            for error in errors {
                result.add_error(error);
            }

            // Write results to file
            let eval_results = serde_json::to_string_pretty(&result)
                .context("Failed to serialize evaluation results to JSON")?;

            let eval_results_file = env::current_dir()
                .context("Failed to get current directory")?
                .join(&self.config.eval_result_filename);

            fs::write(&eval_results_file, &eval_results).with_context(|| {
                format!(
                    "Failed to write evaluation results to {}",
                    eval_results_file.display()
                )
            })?;

            self.config.save("config.cfg".to_string());
            work_dir.save();

            // handle running post-process cmd if configured
            if let Some(cmd) = &bench_eval.post_process_cmd {
                let handle = Command::new(cmd)
                    .arg(&eval_results_file)
                    .spawn()
                    .with_context(|| {
                        format!("Failed to execute post-process command: {:?}", cmd)
                    })?;

                await_process_exits(&mut [handle], Vec::new());
            }

            // copy session file into eval-dir
            let here = env::current_dir()
                .context("Failed to get current directory")?
                .canonicalize()
                .context("Failed to canonicalize current directory path")?;

            BenchmarkWorkDir::deep_copy(agent.session_file().as_path(), here.as_path(), false)
                .context("Failed to copy session file to evaluation directory")?;
        } else {
            bail!("No evaluation found for selector: {}", bench_eval.selector);
        }

        Ok(())
    }
    pub async fn run_dataset<F, Fut>(&mut self, agent_generator: F) -> Result<()>
    where
        F: Fn(ExtensionRequirements, String, bool, Option<HashMap<String, Vec<Tool>>>) -> Fut
            + Clone,
        Fut: Future<Output = BenchAgent> + Send,
    {
        // Find the first active evaluation
        let bench_eval = self
            .config
            .evals
            .iter()
            .find(|eval| eval.active)
            .context("No active evaluations found")?;
        
        println!("Running dataset evaluation: {}", bench_eval.selector);
        let dataset_config = self
            .config
            .dataset
            .as_ref()
            .context("No dataset specified")?;

        if dataset_config.prompt_column.is_empty() {
            bail!("Prompt column cannot be empty");
        }

        tracing::info!(path = %dataset_config.path.display(), "Loading dataset file");

        let file_content = std::fs::read_to_string(&dataset_config.path)?;
        tracing::info!(
            file_size_mb = file_content.len() as f64 / 1024.0 / 1024.0,
            "Dataset file loaded"
        );

        let dataset: serde_json::Value = serde_json::from_str(&file_content)?;
        tracing::info!("Dataset JSON parsed");

        let mut rows = dataset
            .as_array()
            .context("Dataset must be an array")?
            .clone();

        // Apply debug size limit BEFORE scanning tools to avoid processing huge datasets
        if let Some(dataset_cfg) = self.get_dataset_config() {
            if let Some(debug_size) = dataset_cfg.debug_size {
                if debug_size > 0 {
                    let limit = debug_size as usize;
                    rows.truncate(limit);
                    tracing::info!(
                        original_size = dataset.as_array().unwrap().len(),
                        debug_size = limit,
                        "Applied debug size limit to dataset"
                    );
                }
            }
        }

        tracing::info!(final_row_count = rows.len(), "Dataset loading completed");

        let mut work_dir = self
            .create_work_dir(&self.config)
            .context("Failed to create evaluation work directory")?;

        let run_id = &self
            .config
            .run_id
            .clone()
            .unwrap_or_else(|| "run-0".to_string());
        let run_id = format!("run-{}", run_id.clone());

        work_dir.set_eval(&bench_eval.selector, run_id);

        // No need to scan for tools in advance - each agent will handle its own extensions

        // Process all rows with maximum parallelism using chunked approach
        let max_concurrent = self
            .get_dataset_config()
            .map(|dc| dc.max_concurrent)
            .unwrap_or(10);

        // Process rows in parallel with rate limiting
        let bench_eval = self
            .config
            .evals
            .first()
            .context("No evaluations specified")?;
        let selector = bench_eval.selector.clone();

        // Initialize global rate limiter with configured or default settings
        if let Some(dataset_cfg) = self.get_dataset_config() {
            if let Some(rate_limit_rps) = dataset_cfg.rate_limit_rps {
                // Use configured rate limit
                let burst_capacity = ((rate_limit_rps * 2.0).ceil() as u32).max(10);
                crate::rate_limiter::initialize_global_rate_limiter(
                    rate_limit_rps,
                    Some(burst_capacity),
                );
                tracing::info!(rate_limit_rps = rate_limit_rps, "Rate limiter enabled with configured rate");
            } else {
                // Use conservative default rate limit to prevent API overload
                let default_rps = 10.0;
                crate::rate_limiter::initialize_global_rate_limiter(default_rps, Some(20));
                tracing::warn!(
                    default_rps = default_rps,
                    "No rate_limit_rps configured - using conservative default to prevent API overload"
                );
            }
        } else {
            // Use conservative default rate limit to prevent API overload
            let default_rps = 10.0;
            crate::rate_limiter::initialize_global_rate_limiter(default_rps, Some(20));
            tracing::warn!(
                default_rps = default_rps,
                "No dataset config found - using conservative default rate limit to prevent API overload"
            );
        }

        tracing::info!(max_concurrent = max_concurrent, "Processing with fresh agents per row");

        let mut all_results = Vec::new();
        let chunk_size = max_concurrent;

        // Create progress bar
        let progress = ProgressBar::new(rows.len() as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{bar:40.cyan/blue} {pos}/{len} rows ({percent}%) [{elapsed_precise}]")
                .unwrap()
                .progress_chars("##-"),
        );
        progress.set_message("Processing dataset");

        tracing::info!(total_rows = rows.len(), "Starting row processing");

        for (chunk_num, chunk) in rows.chunks(chunk_size).enumerate() {
            tracing::debug!(
                chunk_num = chunk_num,
                chunk_size = chunk.len(),
                "Processing chunk"
            );

            let futures: Vec<_> = chunk
                .iter()
                .enumerate()
                .map(|(chunk_idx, row)| {
                    let global_idx = chunk_num * chunk_size + chunk_idx;
                    process_single_dataset_row_stateless(
                        global_idx,
                        row.clone(),
                        agent_generator.clone(),
                        RowProcessingConfig {
                            dataset_config,
                            selector: selector.clone(),
                        },
                    )
                })
                .collect();

            let chunk_results = futures::future::join_all(futures).await;
            all_results.extend(chunk_results);
            
            // Log rate limiter stats to check saturation
            if let Some(stats) = crate::rate_limiter::global_rate_limiter_stats().await {
                tracing::info!(
                    available_tokens = stats.available_tokens,
                    waiting_requests = stats.waiting_requests,
                    capacity = stats.capacity,
                    "Rate limiter stats after chunk completion"
                );
            }

            tracing::info!(
                chunk_num = chunk_num,
                rows_processed = chunk.len(),
                "Chunk processing completed"
            );

            // Update progress bar by the number of rows completed in this chunk
            progress.inc(chunk.len() as u64);

            // Clean up any remaining mock_mcp_server processes after chunk completion
            cleanup_mock_mcp_processes();

            // Chunk delays are now handled by the global rate limiter
        }

        tracing::info!(
            total_rows_processed = rows.len(),
            "All row processing completed"
        );

        // Finish progress bar
        progress.finish_with_message("Dataset processing complete");

        // All agents have been cleaned up individually
        // Final cleanup of any remaining mock_mcp_server processes
        cleanup_mock_mcp_processes();

        self.write_dataset_results(&bench_eval.selector, vec![all_results])
            .await
    }

    async fn write_dataset_results(
        &self,
        selector: &str,
        results: Vec<Vec<serde_json::Value>>,
    ) -> Result<()> {
        let results_path = format!("{}_results.json", selector);
        std::fs::write(&results_path, serde_json::to_string_pretty(&results)?)?;
        Ok(())
    }

    pub fn path_for_eval(model: &BenchModel, eval: &BenchEval, run_id: String) -> PathBuf {
        let provider = model.provider.clone();
        let model = model.name.clone();
        let eval_path = &eval.selector.replace(":", std::path::MAIN_SEPARATOR_STR);
        let eval_results_location = format!(
            "{}-{}/run-{}{}{}",
            &provider,
            model,
            run_id,
            std::path::MAIN_SEPARATOR_STR,
            eval_path
        );
        PathBuf::from(eval_results_location.clone())
    }
}
