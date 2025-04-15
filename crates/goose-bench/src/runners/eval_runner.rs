use crate::bench_config::{BenchEval, BenchModel, BenchRunConfig};
use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{EvaluationSuite, ExtensionRequirements};
use crate::reporting::EvaluationResult;
use crate::utilities::await_process_exits;
use std::env;
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct EvalRunner {
    config: BenchRunConfig,
}

impl EvalRunner {
    pub fn from(config: String) -> anyhow::Result<EvalRunner> {
        let config = BenchRunConfig::from_string(config)?;
        Ok(EvalRunner { config })
    }

    fn create_work_dir(&self, config: &BenchRunConfig) -> anyhow::Result<BenchmarkWorkDir> {
        let goose_model = config.models.first().unwrap();
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
    pub async fn run<F, Fut>(&mut self, agent_generator: F) -> anyhow::Result<()>
    where
        F: Fn(ExtensionRequirements, String) -> Fut,
        Fut: Future<Output = BenchAgent> + Send,
    {
        let mut work_dir = self.create_work_dir(&self.config)?;
        let bench_eval = self.config.evals.first().unwrap();

        let run_id = &self
            .config
            .run_id
            .clone()
            .unwrap_or_else(|| "run-0".to_string());
        let run_id = format!("run-{}", run_id.clone());

        // create entire dir subtree for eval and cd into dir for running eval
        work_dir.set_eval(&bench_eval.selector, run_id);

        if let Some(eval) = EvaluationSuite::from(&bench_eval.selector) {
            let now_stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
            let session_id = format!("{}-{}", bench_eval.selector.clone(), now_stamp);
            let mut agent = agent_generator(eval.required_extensions(), session_id).await;

            let mut result = EvaluationResult::new(eval.name().to_string());

            if let Ok(metrics) = eval.run(&mut agent, &mut work_dir).await {
                for (name, metric) in metrics {
                    result.add_metric(name, metric);
                }

                // Add any errors that occurred
                for error in agent.get_errors().await {
                    result.add_error(error);
                }
            }

            let eval_results = serde_json::to_string_pretty(&result)?;

            let eval_results_file = env::current_dir()?.join(&self.config.eval_result_filename);
            fs::write(&eval_results_file, &eval_results)?;
            self.config.save("config.cfg".to_string());
            work_dir.save();

            // handle running post-process cmd if configured
            if let Some(cmd) = &bench_eval.post_process_cmd {
                let handle = Command::new(cmd).arg(&eval_results_file).spawn()?;
                await_process_exits(&mut [handle], Vec::new());
            }

            // copy session file into eval-dir
            let here = env::current_dir()?.canonicalize()?;
            BenchmarkWorkDir::deep_copy(agent.session_file().as_path(), here.as_path(), false)?;
        }

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
