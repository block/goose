use crate::bench_work_dir::BenchmarkWorkDir;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::read_to_string;
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BenchToolShimOpt {
    pub use_tool_shim: bool,
    pub tool_shim_model: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BenchModel {
    pub provider: String,
    pub name: String,
    pub parallel_safe: bool,
    pub tool_shim: Option<BenchToolShimOpt>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BenchDatasetConfig {
    pub max_concurrent: usize,
    pub debug_size: Option<isize>,
    pub max_interactions: Option<usize>,
    /// Global rate limit for all LLM requests (requests per second)
    /// If not specified, no rate limiting is applied
    pub rate_limit_rps: Option<f64>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BenchDataset {
    pub path: PathBuf,
    pub prompt_column: String,
    pub system_prompt_column: Option<String>,
    pub tools_column: Option<String>,
    pub llm_output_column: String,
    pub output_dataset_file_name: String,
    #[serde(default)]
    pub config: Option<BenchDatasetConfig>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BenchEval {
    pub env: Vec<String>,
    pub selector: String,
    pub post_process_cmd: Option<PathBuf>,
    pub parallel_safe: bool,
    #[serde(default = "default_active")]
    pub active: bool,
}

fn default_active() -> bool {
    true
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BenchRunConfig {
    pub models: Vec<BenchModel>,
    pub evals: Vec<BenchEval>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub dataset: Option<BenchDataset>,
    pub include_dirs: Vec<PathBuf>,
    pub repeat: Option<usize>,
    pub run_id: Option<String>,
    pub output_dir: Option<PathBuf>,
    pub eval_result_filename: String,
    pub run_summary_filename: String,
    pub env_file: Option<PathBuf>,
}

impl Default for BenchRunConfig {
    fn default() -> Self {
        BenchRunConfig {
            models: vec![
                BenchModel {
                    provider: "databricks".to_string(),
                    name: "goose".to_string(),
                    parallel_safe: true,
                    tool_shim: Some(BenchToolShimOpt {
                        use_tool_shim: false,
                        tool_shim_model: None,
                    }),
                },
                BenchModel {
                    provider: "databricks".to_string(),
                    name: "goose-claude-3-5-sonnet".to_string(),
                    parallel_safe: true,
                    tool_shim: None,
                },
            ],
            evals: vec![BenchEval {
                env: vec![],
                selector: "core".into(),
                post_process_cmd: None,
                parallel_safe: true,
                active: true,
            }],
            env: vec![],
            dataset: None,
            include_dirs: vec![],
            repeat: Some(2),
            run_id: None,
            output_dir: None,
            eval_result_filename: "eval-results.json".to_string(),
            run_summary_filename: "run-results-summary.json".to_string(),
            env_file: None,
        }
    }
}
impl BenchRunConfig {
    pub fn from_string(cfg: String) -> anyhow::Result<Self> {
        let mut config: Self = serde_json::from_str(cfg.as_str())?;
        config.include_dirs = BenchmarkWorkDir::canonical_dirs(config.include_dirs);
        Self::canonicalize_eval_post_proc_cmd(&mut config);

        // Validate global environment variables
        Self::parse_env_vars(&config.env).context("Invalid global environment variables")?;

        // Validate eval-specific environment variables
        for (i, eval) in config.evals.iter().enumerate() {
            Self::parse_env_vars(&eval.env)
                .context(format!("Invalid environment variables in eval {}", i))?;
        }

        Ok(config)
    }

    /// Parse environment variable string in format "KEY=value" into (key, value) tuple
    pub fn parse_env_var(env_var: &str) -> anyhow::Result<(String, String)> {
        if let Some((key, value)) = env_var.split_once('=') {
            Ok((key.to_string(), value.to_string()))
        } else {
            anyhow::bail!(
                "Invalid environment variable format: '{}'. Expected format: 'KEY=value'",
                env_var
            )
        }
    }

    /// Parse a vector of environment variable strings into (key, value) tuples
    pub fn parse_env_vars(env_vars: &[String]) -> anyhow::Result<Vec<(String, String)>> {
        env_vars
            .iter()
            .map(|env_var| Self::parse_env_var(env_var))
            .collect()
    }

    fn canonicalize_eval_post_proc_cmd(config: &mut BenchRunConfig) {
        config.evals.iter_mut().for_each(|eval| {
            if let Some(post_process_cmd) = &eval.post_process_cmd {
                let canon = BenchmarkWorkDir::canonical_dirs(vec![post_process_cmd.clone()]);
                let full_path_cmd = canon[0].clone();
                if !full_path_cmd.exists() {
                    panic!("BenchConfigError: Eval post-process command not found. File {:?} does not exist", full_path_cmd);
                }
                eval.post_process_cmd = Some(full_path_cmd);
            }
        });
    }
    pub fn from(cfg: PathBuf) -> anyhow::Result<Self> {
        let content = read_to_string(&cfg)?;
        let config = Self::from_string(content)?;
        Ok(config)
    }

    pub fn to_string(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn save(&self, name: String) {
        let config = self.to_string().unwrap();
        fs::write(name, config).expect("Unable to write bench config file");
    }
}
