use crate::model_training::job_manager::{TrainerExecutor, TrainingJob};
use crate::model_training::trainer::{TrainingConfig, FinetuneMethod, PreferenceMethod, RLMethod, RewardMethod, Quantization, TrainingResult, TrainingProgress, ModelVersion};
use crate::training_data::schema::{TrainingExample, PreferenceExample};
use anyhow::{Context, Result};
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use uuid::Uuid;

// Embed the training script so it's always available
const TRAINING_SCRIPT: &str = include_str!("train_lora.py");

#[derive(Debug, Clone)]
pub struct AxolotlRuntime {
    pub python: String,             // e.g., "python" or full path
    pub venv_path: Option<PathBuf>, // optional venv path
    pub output_root: PathBuf,       // where to store runs/checkpoints
}

impl Default for AxolotlRuntime {
    fn default() -> Self {
        Self::from_env()
    }
}

impl AxolotlRuntime {
    pub fn from_env() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let default_root = home.join(".config/goose/training");
        let default_venv = home.join(".config/goose/axolotl-venv");
        
        // Check for explicit env vars first, then auto-detect venv
        let (python, venv_path) = if let Ok(py) = std::env::var("AXOLOTL_PYTHON") {
            let venv = std::env::var("AXOLOTL_VENV").ok().map(PathBuf::from);
            (py, venv)
        } else if default_venv.join("bin/python").exists() {
            // Auto-detect the venv we created
            let py = default_venv.join("bin/python").display().to_string();
            (py, Some(default_venv))
        } else {
            // Fallback to system python
            ("python".into(), None)
        };
        
        let output_root = std::env::var("TRAINING_OUTPUT_DIR").map(PathBuf::from).unwrap_or(default_root);
        Self { python, venv_path, output_root }
    }
}

#[derive(Serialize)]
struct DatasetRef {
    path: String,
    #[serde(rename = "type")]
    kind: String, // "chat" or "preference"
}

#[derive(Serialize)]
struct LoRACfg {
    r: usize,
    alpha: usize,
    target_modules: Vec<String>,
    bias: String,
    dropout: f32,
}

#[derive(Serialize)]
struct TrainCfg {
    epochs: usize,
    per_device_train_batch_size: usize,
    learning_rate: f64,
    warmup_steps: usize,
    max_seq_length: usize,
    gradient_accumulation_steps: usize,
    weight_decay: f64,
    bf16: bool,
    fp16: bool,
}

#[derive(Serialize)]
struct AxolotlConfig {
    base_model: String,
    chat_template: Option<String>,
    datasets: Vec<DatasetRef>,
    output_dir: String,
    save_steps: usize,
    eval_steps: usize,
    logging_steps: usize,
    // method flags
    finetune_method: String,
    preference_method: Option<String>,
    rl_method: Option<String>,
    reward_method: Option<String>,
    quantization: Option<String>,
    // LoRA
    lora: Option<LoRACfg>,
    // training
    training: TrainCfg,
    // HuggingFace token for gated models
    #[serde(skip_serializing_if = "Option::is_none")]
    hf_token: Option<String>,
    // Force CPU training (for memory-constrained devices)
    #[serde(skip_serializing_if = "Option::is_none")]
    use_cpu: Option<bool>,
}

fn map_chat_template(model_path: &Path) -> Option<String> {
    // Heuristic: qwen model path contains "qwen"
    let s = model_path.display().to_string().to_lowercase();
    if s.contains("qwen") {
        Some("qwen".into())
    } else if s.contains("llama") {
        Some("llama-3".into())
    } else {
        None
    }
}

fn to_axolotl_yaml(cfg: &TrainingConfig, base_model_path: &Path, datasets: Vec<DatasetRef>, run_dir: &Path) -> Result<String> {
    let lora = match cfg.finetune_method {
        FinetuneMethod::LoRA | FinetuneMethod::QLoRA => Some(LoRACfg {
            r: cfg.lora_config.rank,
            alpha: cfg.lora_config.alpha as usize,
            target_modules: cfg.lora_config.target_modules.clone(),
            bias: cfg.lora_config.bias.clone(),
            dropout: cfg.lora_config.dropout as f32,
        }),
        FinetuneMethod::FullFineTune => None,
    };

    let finetune_method = match cfg.finetune_method { FinetuneMethod::FullFineTune => "full", FinetuneMethod::LoRA => "lora", FinetuneMethod::QLoRA => "qlora" }.to_string();
    let preference_method = match cfg.preference_method { PreferenceMethod::None => None, PreferenceMethod::DPO => Some("dpo"), PreferenceMethod::IPO => Some("ipo"), PreferenceMethod::KTO => Some("kto"), PreferenceMethod::ORPO => Some("orpo") }.map(|s| s.to_string());
    let rl_method = match cfg.rl_method { RLMethod::None => None, RLMethod::GRPO => Some("grpo") }.map(|s| s.to_string());
    let reward_method = match cfg.reward_method { RewardMethod::None => None, RewardMethod::RM => Some("rm"), RewardMethod::PRM => Some("prm") }.map(|s| s.to_string());
    let quantization = match cfg.quantization { Quantization::None => None, Quantization::GPTQ => Some("gptq"), Quantization::QAT => Some("qat") }.map(|s| s.to_string());

    let train = TrainCfg {
        epochs: cfg.num_epochs,
        per_device_train_batch_size: cfg.batch_size,
        learning_rate: cfg.learning_rate,
        warmup_steps: cfg.warmup_steps,
        max_seq_length: cfg.max_seq_length,
        gradient_accumulation_steps: cfg.gradient_accumulation_steps,
        weight_decay: cfg.weight_decay,
        bf16: cfg.mixed_precision,
        fp16: cfg.mixed_precision,
    };

    // Extract HF token and CPU flag from metadata
    let hf_token = cfg.metadata.get("hf_token").cloned();
    let use_cpu = cfg.metadata.get("use_cpu").and_then(|v| {
        if v == "true" { Some(true) } else { None }
    });

    let ax = AxolotlConfig {
        base_model: base_model_path.display().to_string(),
        chat_template: map_chat_template(base_model_path),
        datasets,
        output_dir: run_dir.display().to_string(),
        save_steps: cfg.save_steps,
        eval_steps: cfg.eval_steps,
        logging_steps: cfg.logging_steps,
        finetune_method,
        preference_method,
        rl_method,
        reward_method,
        quantization,
        lora,
        training: train,
        hf_token,
        use_cpu,
    };

    let yaml = serde_yaml::to_string(&ax)?;
    Ok(yaml)
}

async fn write_yaml(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() { tokio::fs::create_dir_all(parent).await?; }
    tokio::fs::write(path, content).await?;
    Ok(())
}

fn parse_progress_line(line: &str) -> Option<(usize, f32)> {
    // Try to parse patterns like: "step 123 loss 1.234" or "Step: 12 | loss=0.56"
    static STEP_LOSS_1: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| Regex::new(r"step\s+(?P<step>\d+).*loss\s+(?P<loss>\d+\.?\d*)").unwrap());
    static STEP_LOSS_2: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| Regex::new(r"Step\s*[:=]\s*(?P<step>\d+).*loss\s*[:=]\s*(?P<loss>\d+\.?\d*)").unwrap());
    if let Some(caps) = STEP_LOSS_1.captures(line).or_else(|| STEP_LOSS_2.captures(line)) {
        let step: usize = caps.name("step")?.as_str().parse().ok()?;
        let loss: f32 = caps.name("loss")?.as_str().parse().ok()?;
        return Some((step, loss));
    }
    None
}

pub struct AxolotlTrainerExecutor {
    runtime: AxolotlRuntime,
}

impl AxolotlTrainerExecutor {
    pub fn new(runtime: AxolotlRuntime) -> Self { Self { runtime } }

    async fn prepare_run_dirs(&self, job: &TrainingJob) -> Result<PathBuf> {
        let run_dir = self.runtime.output_root.join(format!("job-{}", job.id));
        tokio::fs::create_dir_all(&run_dir).await?;
        Ok(run_dir)
    }

    async fn export_datasets(&self, run_dir: &Path, training_examples: &[TrainingExample], cfg: &TrainingConfig) -> Result<(Vec<DatasetRef>, Option<PathBuf>)> {
        let mut datasets = Vec::new();
        // SFT dataset
        let sft_path = run_dir.join("sft.jsonl");
        self.export_sft_jsonl(training_examples, &sft_path).await?;
        datasets.push(DatasetRef { path: sft_path.display().to_string(), kind: "chat".into() });

        // Preference dataset for DPO (best-effort): read ~/.config/goose/training/preferences.jsonl
        let mut dpo_path: Option<PathBuf> = None;
        if matches!(cfg.preference_method, PreferenceMethod::DPO) {
            let prefs_src = self.runtime.output_root.join("preferences.jsonl");
            if prefs_src.exists() {
                let dst = run_dir.join("dpo.jsonl");
                tokio::fs::copy(&prefs_src, &dst).await.ok();
                datasets.push(DatasetRef { path: dst.display().to_string(), kind: "preference".into() });
                dpo_path = Some(dst);
            }
        }
        Ok((datasets, dpo_path))
    }

    async fn export_sft_jsonl(&self, examples: &[TrainingExample], path: &Path) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        
        let mut file = tokio::fs::File::create(path).await?;
        
        for example in examples {
            // Convert TrainingExample to Axolotl chat format
            let mut messages = Vec::new();
            for msg in &example.messages {
                messages.push(serde_json::json!({
                    "role": match msg.role {
                        rmcp::model::Role::User => "user",
                        rmcp::model::Role::Assistant => "assistant",
                    },
                    "content": msg.as_concat_text(),
                }));
            }
            
            let line = serde_json::json!({
                "messages": messages,
            });
            
            file.write_all(line.to_string().as_bytes()).await?;
            file.write_all(b"\n").await?;
        }
        
        Ok(())
    }

    async fn generate_config(&self, job: &TrainingJob, run_dir: &Path, datasets: Vec<DatasetRef>) -> Result<PathBuf> {
        let yaml = to_axolotl_yaml(&job.config, &job.base_model_path, datasets, run_dir)?;
        let cfg_path = run_dir.join("config.yml");
        write_yaml(&cfg_path, &yaml).await?;
        Ok(cfg_path)
    }

    fn build_command_with_script(&self, script_path: &Path, cfg_path: &Path, run_dir: &Path) -> Command {
        let mut cmd = Command::new(&self.runtime.python);
        
        // Run the training script directly (accelerate is used internally if needed)
        cmd.arg(script_path)
            .arg("-c").arg(cfg_path);
        
        // Working directory is run_dir for artifacts
        cmd.current_dir(run_dir);

        // Adjust PATH if venv provided
        if let Some(venv) = &self.runtime.venv_path {
            use std::env;
            let envs: HashMap<String, String> = env::vars().collect();
            let bin_dir = if cfg!(target_os = "windows") { venv.join("Scripts") } else { venv.join("bin") };
            let mut new_path = bin_dir.display().to_string();
            if let Some(old) = envs.get("PATH") { new_path.push(if cfg!(target_os = "windows") { ';' } else { ':' }); new_path.push_str(old); }
            cmd.env("PATH", new_path);
        }
        cmd
    }

    async fn find_adapter(&self, out_dir: &Path) -> Option<PathBuf> {
        // Common Axolotl/PEFT output: adapter_model.safetensors under output_dir
        let candidate = out_dir.join("adapter_model.safetensors");
        if candidate.exists() { return Some(candidate); }
        // Fallback: search recursively for adapter_model.safetensors (best-effort)
        if let Ok(mut rd) = tokio::fs::read_dir(out_dir).await {
            while let Ok(Some(entry)) = rd.next_entry().await {
                let p = entry.path();
                if p.is_dir() {
                    let inner = p.join("adapter_model.safetensors");
                    if inner.exists() { return Some(inner); }
                }
            }
        }
        None
    }
}

#[async_trait::async_trait]
impl TrainerExecutor for AxolotlTrainerExecutor {
    async fn execute_job(
        &self,
        job: &TrainingJob,
        training_examples: Vec<TrainingExample>,
        progress_sender: mpsc::UnboundedSender<TrainingProgress>,
    ) -> Result<TrainingResult> {
        // Verify runtime is available before starting
        self.check_runtime().await?;

        let run_dir = self.prepare_run_dirs(job).await?;
        
        // Write embedded training script to job directory
        let script_path = run_dir.join("train_lora.py");
        tokio::fs::write(&script_path, TRAINING_SCRIPT).await
            .context("Failed to write training script")?;
        info!("Training script written to: {}", script_path.display());
        
        let (datasets, _maybe_dpo) = self.export_datasets(&run_dir, &training_examples, &job.config).await?;
        let cfg_path = self.generate_config(job, &run_dir, datasets).await?;

        info!("Launching training with config: {}", cfg_path.display());
        let mut cmd = self.build_command_with_script(&script_path, &cfg_path, &run_dir);
        cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
        let mut child = cmd.spawn().context("failed to spawn training process")?;

        // Stream logs
        if let Some(stdout) = child.stdout.take() {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Some((step, loss)) = parse_progress_line(&line) {
                    let _ = progress_sender.send(TrainingProgress {
                        job_id: job.id,
                        epoch: 0,
                        step,
                        total_steps: 0,
                        loss,
                        learning_rate: job.config.learning_rate,
                        throughput: 0.0,
                        eta_seconds: None,
                        timestamp: chrono::Utc::now(),
                    });
                }
                info!(target: "axolotl", "{}", line);
            }
        }
        if let Some(stderr) = child.stderr.take() {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                warn!(target: "axolotl", "{}", line);
            }
        }
        let status = child.wait().await.context("failed to wait for axolotl")?;
        if !status.success() {
            return Err(anyhow::anyhow!("axolotl exited with status {}", status));
        }

        // Discover adapter
        let adapter_path = self.find_adapter(&run_dir).await;

        let model_version = ModelVersion {
            id: Uuid::new_v4(),
            name: format!("trained-{}", job.id),
            path: adapter_path.unwrap_or_else(|| run_dir.clone()),
            created_at: chrono::Utc::now(),
        };

        // NOTE: Registering the version is done by the higher-level trainer in current design.
        Ok(TrainingResult {
            job_id: job.id,
            model_version,
            final_loss: 0.0,
            best_eval_loss: None,
            training_time_seconds: 0,
            total_steps: 0,
            convergence_achieved: true,
            evaluation_metrics: None,
        })
    }
}

impl AxolotlTrainerExecutor {
    async fn check_runtime(&self) -> Result<()> {
        // Check for required libraries: accelerate, transformers, peft, torch, yaml
        let py = &self.runtime.python;
        
        info!("Checking training runtime with python: {}", py);
        
        // Check all required libraries at once
        let mut check = Command::new(py);
        check.arg("-c").arg("import accelerate, transformers, peft, torch, yaml; print('ok')");
        
        let output = check.output().await
            .map_err(|e| anyhow::anyhow!("Failed to run python: {}", e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            error!("Runtime check failed. stdout: {}, stderr: {}", stdout, stderr);
            return Err(anyhow::anyhow!(
                "Required libraries not available. Install with: pip install accelerate transformers peft torch pyyaml\nError: {}",
                stderr
            )); 
        }

        info!("Training runtime check passed: all required libraries available");
        Ok(())
    }
}
