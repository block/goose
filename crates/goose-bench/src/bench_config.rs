use crate::bench_work_dir::BenchmarkWorkDir;
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
pub struct BenchEval {
    pub selector: String,
    pub post_process_cmd: Option<PathBuf>,
    pub parallel_safe: bool,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BenchRunConfig {
    pub models: Vec<BenchModel>,
    pub evals: Vec<BenchEval>,
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
                    name: "goose-claude-4-sonnet".to_string(),
                    parallel_safe: true,
                    tool_shim: None,
                },
            ],
            evals: vec![BenchEval {
                selector: "core".into(),
                post_process_cmd: None,
                parallel_safe: true, // Default to true
            }],
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
        // update include_dirs to contain full-paths only
        config.include_dirs = BenchmarkWorkDir::canonical_dirs(config.include_dirs);
        Self::canonicalize_eval_post_proc_cmd(&mut config);
        Ok(config)
    }

    fn canonicalize_eval_post_proc_cmd(config: &mut BenchRunConfig) {
        // update eval post-process script paths to all be full-paths
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
        let config = Self::from_string(read_to_string(cfg)?)?;
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
