use crate::bench_config::{BenchEval, BenchModel, BenchRunConfig};
use crate::eval_suites::EvaluationSuite;
use crate::reporting::{BenchmarkResults, SuiteResult};
use crate::runners::eval_runner::EvalRunner;
use crate::utilities::{await_process_exits, parallel_bench_cmd, union_hashmaps};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::process::Child;
use std::thread;

#[derive(Clone)]
pub struct ModelRunner {
    config: BenchRunConfig,
}

impl ModelRunner {
    pub fn from(config: String) -> anyhow::Result<ModelRunner> {
        let config = BenchRunConfig::from_string(config)?;
        Ok(ModelRunner { config })
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let model = self.config.models.first().unwrap();
        let suites = self.collect_evals_for_run();

        let mut handles = vec![];

        for i in 0..self.config.repeat.unwrap_or(1) {
            let mut self_copy = self.clone();
            let model_clone = model.clone();
            let suites_clone = suites.clone();
            // create thread to handle launching parallel processes to run model's evals in parallel
            let handle = thread::spawn(move || {
                self_copy.run_benchmark(&model_clone, suites_clone, i.to_string())
            });
            handles.push(handle);
        }
        await_process_exits(&mut Vec::new(), handles);

        let mut all_runs_results: Vec<BenchmarkResults> = Vec::new();
        for i in 0..self.config.repeat.unwrap_or(1) {
            let run_results =
                self.collect_run_results(model.clone(), suites.clone(), i.to_string())?;
            all_runs_results.push(run_results);
        }
        // write summary file

        Ok(())
    }

    fn run_benchmark(
        &mut self,
        model: &BenchModel,
        suites: HashMap<String, Vec<BenchEval>>,
        run_id: String,
    ) -> anyhow::Result<()> {
        let mut results_handles = HashMap::<String, Vec<Child>>::new();

        let mut envs = self.toolshim_envs();
        envs.push((model.clone().name, model.clone().provider));

        for (suite, evals) in suites.iter() {
            results_handles.insert((*suite).clone(), Vec::new());

            // launch single suite's evals in parallel
            for eval_selector in evals {
                self.config.run_id = Some(run_id.clone());
                self.config.evals = vec![(*eval_selector).clone()];
                let cfg = self.config.to_string()?;
                let handle = parallel_bench_cmd("exec-eval".to_string(), cfg, envs.clone());
                results_handles.get_mut(suite).unwrap().push(handle);
            }
        }

        // await all suite's evals
        for (_, child_procs) in results_handles.iter_mut() {
            await_process_exits(child_procs, Vec::new());
        }

        Ok(())
    }

    fn collect_run_results(
        &self,
        model: BenchModel,
        suites: HashMap<String, Vec<BenchEval>>,
        run_id: String,
    ) -> anyhow::Result<BenchmarkResults> {
        let mut results = BenchmarkResults::new(model.provider.clone());

        let mut summary_path: Option<PathBuf> = None;

        for (suite, evals) in suites.iter() {
            let mut suite_result = SuiteResult::new(suite.clone());
            for eval_selector in evals {
                let mut eval_path =
                    EvalRunner::path_for_eval(&model, eval_selector, run_id.clone());
                eval_path.push(self.config.eval_result_filename.clone());
                let eval_result = serde_json::from_str(&read_to_string(&eval_path)?)?;
                suite_result.add_evaluation(eval_result);

                // use current eval to determine where the summary should be written
                if summary_path.is_none() {
                    let mut result = PathBuf::new();
                    let mut iter = eval_path.components();
                    if let Some(first) = iter.next() {
                        result.push(first);
                        if let Some(second) = iter.next() {
                            result.push(second);
                        }
                    }
                    summary_path = Some(result);
                }
            }
            results.add_suite(suite_result);
        }

        let mut run_summary = PathBuf::new();
        run_summary.push(summary_path.clone().unwrap());
        run_summary.push(&self.config.run_summary_filename);

        let output_str = serde_json::to_string_pretty(&results)?;
        std::fs::write(run_summary, &output_str)?;

        Ok(results)
    }
    fn collect_evals_for_run(&self) -> HashMap<String, Vec<BenchEval>> {
        // convert suites map {suite_name => [eval_selector_str] to map suite_name => [BenchEval]
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
        union_hashmaps(suites)
    }

    fn toolshim_envs(&self) -> Vec<(String, String)> {
        // read tool-shim preference from config, set respective env vars accordingly
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
}
