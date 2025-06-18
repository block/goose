use crate::bench_config::{BenchEval, BenchModel, BenchRunConfig};
use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{EvaluationSuite, ExtensionRequirements};
use crate::reporting::EvaluationResult;
use crate::utilities::await_process_exits;
use anyhow::{bail, Context, Result};
use mcp_core::Tool;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing;

// Configuration struct to reduce function parameter count
#[derive(Clone)]
struct RowProcessingConfig<'a> {
    dataset_config: &'a crate::bench_config::BenchDataset,
    selector: String,
    request_delay_ms: Option<u64>,
    agent_creation_mutex: Arc<Mutex<()>>,
    agent_timeout_seconds: Option<u64>,
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

        // Use high precision timestamp + index to ensure uniqueness across parallel executions
        let session_id = format!("{}-{}-row{}", config.selector, now_stamp, idx);

        let extensions = parse_extensions_stateless(&row, &config.dataset_config.tools_column)?;

        // Serialize agent creation to prevent database table conflicts
        let mut agent = {
            let _lock = config.agent_creation_mutex.lock().await;
            agent_generator(
                ExtensionRequirements::default(),
                session_id,
                true,
                Some(extensions),
            )
            .await
        };

        if let Some(system_prompt) =
            get_dataset_system_prompt_stateless(&row, &config.dataset_config.system_prompt_column)
        {
            agent.session.override_system_prompt(system_prompt).await;
        }

        let prompt = row
            .get(&config.dataset_config.prompt_column)
            .and_then(|v| v.as_str())
            .context("Missing prompt field")?;

        // Add delay before LLM call to respect rate limits
        if let Some(delay) = config.request_delay_ms {
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        // Use timeout to prevent hanging on user input requests
        let timeout_duration = Duration::from_secs(config.agent_timeout_seconds.unwrap_or(30));
        let (result, timed_out) =
            match tokio::time::timeout(timeout_duration, agent.prompt(prompt.to_string())).await {
                Ok(result) => (result, false),
                Err(_) => {
                    tracing::warn!(
                        "Agent prompt timed out after {}s for row {}, collecting partial results",
                        timeout_duration.as_secs(),
                        idx
                    );
                    // Instead of failing, collect the agent's message history up to the timeout
                    (Ok(agent.session.message_history()), true)
                }
            };

        Ok::<(Result<Vec<goose::message::Message>>, bool), anyhow::Error>((result, timed_out))
    }
    .await
    .unwrap_or_else(|e: anyhow::Error| {
        // If there's an error in the async block, treat it as a regular error
        (Err(e), false)
    });

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

    pub async fn run<F, Fut>(&mut self, agent_generator: F) -> Result<()>
    where
        F: Fn(ExtensionRequirements, String, bool, Option<HashMap<String, Vec<Tool>>>) -> Fut
            + Clone,
        Fut: Future<Output = BenchAgent> + Send,
    {
        let bench_eval = self
            .config
            .evals
            .first()
            .context("No evaluations specified in configuration")?;

        if bench_eval.dataset.is_some() {
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
        tracing::info!("Set evaluation directory for {}", bench_eval.selector);

        if let Some(eval) = EvaluationSuite::from(&bench_eval.selector) {
            let now_stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("Failed to get current timestamp")?
                .as_nanos();

            let session_id = format!("{}-{}", bench_eval.selector.clone(), now_stamp);
            let mut agent =
                agent_generator(eval.required_extensions(), session_id, false, None).await;
            tracing::info!("Agent created for {}", eval.name());

            let mut result = EvaluationResult::new(eval.name().to_string());

            match eval.run(&mut agent, &mut work_dir).await {
                Ok(metrics) => {
                    tracing::info!("Evaluation run successful with {} metrics", metrics.len());
                    for (name, metric) in metrics {
                        result.add_metric(name, metric);
                    }
                }
                Err(e) => {
                    tracing::error!("Evaluation run failed: {}", e);
                }
            }

            // Add any errors that occurred
            let errors = agent.get_errors().await;
            tracing::info!("Agent reported {} errors", errors.len());
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

            tracing::info!(
                "Wrote evaluation results to {}",
                eval_results_file.display()
            );

            self.config.save("config.cfg".to_string());
            work_dir.save();

            // handle running post-process cmd if configured
            if let Some(cmd) = &bench_eval.post_process_cmd {
                tracing::info!("Running post-process command: {:?}", cmd);

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

            tracing::info!("Evaluation completed successfully");
        } else {
            tracing::error!("No evaluation found for selector: {}", bench_eval.selector);
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
        let bench_eval = self
            .config
            .evals
            .first()
            .context("No evaluations specified")?;
        let dataset_config = bench_eval
            .dataset
            .as_ref()
            .context("No dataset specified")?;

        if dataset_config.prompt_column.is_empty() {
            bail!("Prompt column cannot be empty");
        }

        let dataset: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&dataset_config.path)?)?;
        let mut rows = dataset
            .as_array()
            .context("Dataset must be an array")?
            .clone();

        // Apply debug_size limit if specified
        if let Some(dataset_cfg) = &self.config.dataset_config {
            if let Some(debug_size) = dataset_cfg.debug_size {
                if debug_size > 0 {
                    let limit = debug_size as usize;
                    rows.truncate(limit);
                    tracing::info!(
                        "Limited dataset to {} rows due to debug_size setting",
                        limit
                    );
                }
            }
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
        tracing::info!("Set evaluation directory for {}", bench_eval.selector);

        // Process all rows with maximum parallelism using chunked approach
        let max_concurrent = self
            .config
            .dataset_config
            .as_ref()
            .map(|dc| dc.max_concurrent)
            .unwrap_or(10);

        tracing::info!(
            "Processing {} rows with max concurrency {}",
            rows.len(),
            max_concurrent
        );

        // Process rows in parallel with rate limiting
        let bench_eval = self
            .config
            .evals
            .first()
            .context("No evaluations specified")?;
        let selector = bench_eval.selector.clone();

        // Calculate delay between requests based on requests_per_second
        let request_delay_ms = self
            .config
            .dataset_config
            .as_ref()
            .and_then(|dc| dc.requests_per_second)
            .map(|rps| (1000.0 / rps) as u64);

        // Get chunk delay (pause between processing chunks)
        let chunk_delay_ms = self
            .config
            .dataset_config
            .as_ref()
            .and_then(|dc| dc.chunk_delay_ms)
            .unwrap_or(0);

        // Get agent timeout
        let agent_timeout_seconds = self
            .config
            .dataset_config
            .as_ref()
            .and_then(|dc| dc.agent_timeout_seconds);

        // Create mutex to serialize agent creation and prevent database conflicts
        let agent_creation_mutex = Arc::new(Mutex::new(()));

        // Process in chunks to control overall throughput
        let mut all_results = Vec::new();
        let chunk_size = max_concurrent;

        tracing::info!(
            "Processing {} rows in chunks of {} with rate limiting and serialized agent creation",
            rows.len(),
            chunk_size
        );
        if let Some(delay) = request_delay_ms {
            tracing::info!("Request delay: {}ms", delay);
        }
        if chunk_delay_ms > 0 {
            tracing::info!("Chunk delay: {}ms", chunk_delay_ms);
        }

        for (chunk_num, chunk) in rows.chunks(chunk_size).enumerate() {
            tracing::debug!("Processing chunk {} with {} rows", chunk_num, chunk.len());

            let futures: Vec<_> = chunk
                .iter()
                .enumerate()
                .map(|(chunk_idx, row)| {
                    let global_idx = chunk_num * chunk_size + chunk_idx;
                    let mutex_clone = Arc::clone(&agent_creation_mutex);
                    process_single_dataset_row_stateless(
                        global_idx,
                        row.clone(),
                        agent_generator.clone(),
                        RowProcessingConfig {
                            dataset_config,
                            selector: selector.clone(),
                            request_delay_ms,
                            agent_creation_mutex: mutex_clone,
                            agent_timeout_seconds,
                        },
                    )
                })
                .collect();

            let chunk_results = futures::future::join_all(futures).await;
            all_results.extend(chunk_results);

            // Pause between chunks if configured
            if chunk_delay_ms > 0 && chunk_num < rows.len() / chunk_size {
                tracing::debug!("Waiting {}ms before next chunk", chunk_delay_ms);
                tokio::time::sleep(Duration::from_millis(chunk_delay_ms)).await;
            }
        }

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
        tracing::info!("Wrote dataset evaluation results to {}", results_path);
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
