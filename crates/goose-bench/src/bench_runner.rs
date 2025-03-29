use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{EvaluationSuite, ExtensionRequirements};
use crate::reporting::EvaluationResult;
use crate::utilities::union_hashmaps;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::read_to_string;
use std::future::Future;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::{env, thread};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BenchModel {
    provider: String,
    name: String,
    parallel_safe: bool,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BenchEval {
    selector: String,
    post_process_cmd: Option<PathBuf>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BenchToolShimOpt {
    use_tool_shim: bool,
    tool_shim_model: Option<String>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BenchRunConfig {
    models: Vec<BenchModel>,
    evals: Vec<BenchEval>,
    include_dirs: Vec<PathBuf>,
    repeat: Option<usize>,
    tool_shim: Option<BenchToolShimOpt>,
    run_id: Option<String>,
}

impl Default for BenchRunConfig {
    fn default() -> Self {
        BenchRunConfig {
            models: vec![
                BenchModel {
                    provider: "databricks".to_string(),
                    name: "goose".to_string(),
                    parallel_safe: true,
                },
                BenchModel {
                    provider: "databricks".to_string(),
                    name: "goose-claude-3-5-sonnet".to_string(),
                    parallel_safe: true,
                },
            ],
            evals: vec![BenchEval {
                selector: "core".into(),
                post_process_cmd: None,
            }],
            include_dirs: vec![],
            repeat: Some(2),
            tool_shim: Some(BenchToolShimOpt {
                use_tool_shim: false,
                tool_shim_model: None,
            }),
            run_id: None,
        }
    }
}
impl BenchRunConfig {
    pub fn from_string(cfg: String) -> anyhow::Result<Self> {
        let mut config: Self = toml::from_str(cfg.as_str())?;
        config.include_dirs = BenchmarkWorkDir::canonical_dirs(config.include_dirs);
        Self::canonicalize_eval_post_proc_cmd(&mut config);
        Ok(config)
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
        let config = Self::from_string(read_to_string(cfg)?)?;
        Ok(config)
    }

    pub fn to_string(&self) -> anyhow::Result<String> {
        Ok(toml::to_string(self)?)
    }

    pub fn save(&self, name: String) {
        let config = toml::to_string(self).unwrap();
        fs::write(name, config).expect("Unable to write bench config file");
    }
}

#[derive(Clone)]
pub struct BenchRunner {
    config: BenchRunConfig,
}

impl BenchRunner {
    pub fn new(config: PathBuf) -> anyhow::Result<BenchRunner> {
        let config = BenchRunConfig::from(config)?;
        BenchmarkWorkDir::init_experiment();
        config.save("config.cfg".to_string());
        Ok(BenchRunner { config })
    }

    pub fn from(config: String) -> anyhow::Result<BenchRunner> {
        let config = BenchRunConfig::from_string(config)?;
        Ok(BenchRunner { config })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let (parallel_models, serial_models): &(Vec<BenchModel>, Vec<BenchModel>) = &self
            .config
            .models
            .clone()
            .into_iter()
            .partition(|model| model.parallel_safe);

        let mut parallel_models_handle = self.parallelize_models(parallel_models);
        self.await_process_exits(&mut parallel_models_handle);

        for model in serial_models {
            self.config.models = vec![model.clone()];
            self.run_eval_model()?;
        }

        // let mut results = BenchmarkResults::new(provider_name.clone());
        // let mut results = Vec::new(); // todo hashmap so collected by model
        // todo mass aggregation
        Ok(())
    }

    fn parallelize_models(&mut self, parallel_models: &Vec<BenchModel>) -> Vec<Child> {
        let mut models_handles = Vec::new();
        for model in parallel_models {
            self.config.models = vec![model.clone()];
            let bench_cmd = "eval-model".to_string();
            let cfg = self.config.to_string().unwrap();
            let model_handle = self.parallel_process(bench_cmd, cfg, Vec::new());
            models_handles.push(model_handle);
        }
        models_handles
    }

    pub fn run_eval_model(&self) -> anyhow::Result<()> {
        let model = self.config.models.first().unwrap();
        let repeat = self.config.repeat.unwrap_or(1);

        let mut handles = vec![];

        for i in 0..repeat {
            let mut self_copy = self.clone();
            let model_clone = model.clone();
            let handle =
                thread::spawn(move || self_copy.run_benchmark(&model_clone, i.to_string()));
            handles.push(handle);
        }

        for handle in handles {
            match handle.join() {
                Ok(_res) => {
                    // Handle the result
                    // e.g. self.handle_summary(&res)?;
                }
                Err(e) => {
                    // Handle thread panic
                    println!("Thread panicked: {:?}", e);
                }
            }
        }

        Ok(())
    }

    fn run_benchmark(&mut self, model: &BenchModel, run_id: String) -> anyhow::Result<()> {
        let suites = self
            .config
            .evals
            .iter()
            .map(|eval| {
                EvaluationSuite::select(vec![eval.clone().selector])
                    .iter()
                    .map(|(suite, evals)| {
                        let bench_evals = evals
                            .iter()
                            .map(|suite_eval| {
                                let mut updated_eval = eval.clone();
                                updated_eval.selector = (*suite_eval).to_string();
                                updated_eval
                            })
                            .collect::<Vec<_>>();
                        (suite.clone(), bench_evals)
                    })
                    .collect()
            })
            .collect();

        let mut results_handles = HashMap::<String, Vec<Child>>::new();

        let envs = [
            vec![(model.clone().name, model.clone().provider)],
            self.toolshim_envs(),
        ]
        .concat();

        for (suite, evals) in union_hashmaps(suites).iter() {
            results_handles.insert((*suite).clone(), Vec::new());

            for eval_selector in evals {
                self.config.run_id = Some(run_id.clone());
                self.config.evals = vec![(*eval_selector).clone()];
                let cfg = self.config.to_string()?;
                let bench_cmd = "exec-eval".to_string();
                let handle = self.parallel_process(bench_cmd, cfg, envs.clone());
                results_handles.get_mut(suite).unwrap().push(handle);
            }
        }

        for (_, child_procs) in results_handles.iter_mut() {
            self.await_process_exits(child_procs);
        }

        Ok(())
    }

    fn toolshim_envs(&self) -> Vec<(String, String)> {
        let mut shim_envs: Vec<(String, String)> = Vec::new();
        if let Some(shim_opt) = &self.config.tool_shim {
            if shim_opt.use_tool_shim {
                shim_envs.push(("GOOSE_TOOLSHIM".to_string(), "true".to_string()));
                if let Some(shim_model) = &shim_opt.tool_shim_model {
                    shim_envs.push((
                        "GOOSE_TOOLSHIM_OLLAMA_MODEL".to_string(),
                        shim_model.clone(),
                    ));
                }
            }
        }
        shim_envs
    }
    pub async fn run_eval<F, Fut>(&mut self, agent_generator: F) -> anyhow::Result<()>
    where
        F: Fn(ExtensionRequirements) -> Fut,
        Fut: Future<Output = Box<dyn BenchAgent>> + Send,
    {
        let goose_model = self.config.models.first().unwrap();
        let model_name = goose_model.name.clone();
        let provider_name = goose_model.provider.clone();

        let run_id = if let Some(run_id) = &self.config.run_id {
            format!("run-{}", run_id.clone())
        } else {
            "run-1".to_string()
        };
        let work_dir_name_shim = {
            let mut shim_name = "".to_string();
            if let Some(shim_opt) = &self.config.tool_shim {
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
        let work_dir_name = format!("{}-{}{}", provider_name, model_name, work_dir_name_shim);
        let mut work_dir = BenchmarkWorkDir::new(work_dir_name, Vec::new());
        let bench_eval = self.config.evals.first().unwrap();
        work_dir.set_eval(&bench_eval.selector, run_id);

        if let Some(eval) = EvaluationSuite::from(&bench_eval.selector) {
            let mut agent = agent_generator(eval.required_extensions()).await;

            let mut result = EvaluationResult::new(eval.name().to_string());

            if let Ok(metrics) = eval.run(&mut agent, &mut work_dir).await {
                for (name, metric) in metrics {
                    result.add_metric(name, metric);
                }

                // Add any errors that occurred
                for error in (*agent).get_errors().await {
                    result.add_error(error);
                }
            }

            let eval_results_file = env::current_dir()?.join("eval_result.json");
            fs::write(&eval_results_file, serde_json::to_string_pretty(&result)?)?;

            if let Some(cmd) = &bench_eval.post_process_cmd {
                let handle = Command::new(cmd).arg(eval_results_file).spawn()?;
                self.await_process_exits(&mut [handle]);
            }

            // todo get and write session file
        }

        Ok(())
    }

    fn await_process_exits(&self, child_processes: &mut [Child]) {
        for child in child_processes.iter_mut() {
            match child.wait() {
                Ok(status) => println!("Child exited with status: {}", status),
                Err(e) => println!("Error waiting for child: {}", e),
            }
        }
    }

    fn parallel_process(
        &self,
        bench_cmd: String,
        config: String,
        envs: Vec<(String, String)>,
    ) -> Child {
        let current_exe = env::current_exe().expect("Failed to get current executable path");

        let mut cmd = Command::new(current_exe);
        cmd.arg("bench").arg(bench_cmd).arg("--config").arg(config);

        for (key, value) in envs.into_iter() {
            cmd.env(key, value);
        }

        cmd.spawn().expect("Failed to spawn child process")
    }

    pub fn handle_summary(&self) -> anyhow::Result<()> {
        // Handle output based on format
        // let output_str = match format.as_str() {
        //     "json" => serde_json::to_string_pretty(&results)?,
        //     _ => results.to_string(), // Uses Display impl
        // };
        //
        // // Save to file if specified
        // if let Some(path) = &output {
        //     std::fs::write(current_dir.join(path), &output_str)?;
        //     println!("Results saved to: {}", path.display());
        // } else {
        //     // Print to console
        //     if summary {
        //         println!("{}", results.summary());
        //     } else {
        //         println!("{}", output_str);
        //     }
        // }

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
