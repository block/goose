use std::sync::Arc;
use axum::{routing::{get, post}, Json, Router};
use goose::training_data::schema::TrainingExample;
use goose::training_data::storage::InMemoryTrainingDataStorage;
use goose::model_training::job_manager::{TrainingJobBuilder, JobPriority, TrainingJobManager, TrainerFactory, TrainerExecutor, TrainingDataFilter};
use goose::model_training::trainer::{TrainingConfig, TrainingProgress};
use http::StatusCode;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

use crate::state::AppState;

// ----- Simple in-memory training data storage (until a DB-backed storage is wired in server state) -----
static STORAGE: Lazy<InMemoryTrainingDataStorage> = Lazy::new(|| InMemoryTrainingDataStorage::new());

// ----- Minimal TrainerFactory/Executor stubs -----
struct AxTrainerFactory;

impl TrainerFactory for AxTrainerFactory {
    fn create_trainer(&self, _config: &TrainingConfig) -> anyhow::Result<Box<dyn TrainerExecutor>> {
        // For now, always return Axolotl executor; later we can branch on backend
        let runtime = goose::model_training::axolotl::AxolotlRuntime::default();
        Ok(Box::new(goose::model_training::axolotl::AxolotlTrainerExecutor::new(runtime)))
    }
}

static JOB_MANAGER: Lazy<TrainingJobManager> = Lazy::new(|| {
    let factory = Arc::new(AxTrainerFactory);
    TrainingJobManager::new(factory, 1)
});

// ----- Request/Response DTOs -----
#[derive(Debug, Deserialize)]
pub struct SubmitFeedbackRequest {
    pub conversation_id: String,
    pub session_id: Option<String>,
    pub messages: Vec<goose::conversation::message::Message>,
    pub rating: Option<u8>,
    pub correction: Option<String>,
    pub comments: Option<String>,
    pub domain_tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct StartTrainingRequest {
    pub base_model_path: String,
    pub priority: Option<String>,     // "low"|"normal"|"high"|"critical"
    pub min_quality_score: Option<f32>,
    pub domain_tags: Option<Vec<String>>,
    pub max_examples: Option<usize>,
    pub backend: Option<String>,      // "axolotl"|"rust_lora"
    pub config_overrides: Option<goose::model_training::trainer::TrainingConfig>,
}

#[derive(Debug, Serialize)]
pub struct StartTrainingResponse {
    pub job_id: String,
}

#[derive(Debug, Serialize)]
pub struct JobsListResponse {
    pub jobs: Vec<goose::model_training::job_manager::TrainingJob>,
}

#[derive(Debug, Serialize)]
pub struct ProgressResponse {
    pub updates: Vec<TrainingProgress>,
}

#[derive(Debug, Deserialize)]
pub struct ActivateAdapterRequest {
    pub lora_path: Option<String>,
    pub job_id: Option<String>,
}

// ----- Handlers -----
pub async fn submit_feedback(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Json(req): Json<SubmitFeedbackRequest>,
) -> Result<Json<String>, StatusCode> {
    // Convert feedback to a TrainingExample and store
    let mut example = TrainingExample::new(
        req.conversation_id.clone(),
        req.messages.clone(),
        "native_model".to_string(),
        "local".to_string(),
    );
    example.session_id = req.session_id;

    // apply optional fields
    if let Some(tags) = req.domain_tags {
        example.domain_tags = tags;
    }

    // update quality metrics heuristically from rating
    if let Some(r) = req.rating {
        let score = match r { 5 => 1.0, 4 => 0.8, 3 => 0.6, 2 => 0.4, 1 => 0.2, _ => 0.5 };
        example.quality_metrics.overall_score = score;
    }

    // add correction as metadata for now
    if let Some(correction) = req.correction {
        example.metadata
            .custom_fields
            .insert("correction".into(), serde_json::json!(correction));
    }
    if let Some(comments) = req.comments {
        example.metadata
            .custom_fields
            .insert("comments".into(), serde_json::json!(comments));
    }

    // Use AppState-managed storage
    state
        .training_state

        .storage
        .store_example(example)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json("ok".to_string()))
}

pub async fn list_jobs(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Result<Json<JobsListResponse>, StatusCode> {
    let jobs = JOB_MANAGER.list_jobs(None, None).await;
    Ok(Json(JobsListResponse { jobs }))
}

pub async fn start_training(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Json(req): Json<StartTrainingRequest>,
) -> Result<Json<StartTrainingResponse>, StatusCode> {
    // Build a job
    let priority = match req.priority.as_deref() {
        Some("low") => JobPriority::Low,
        Some("high") => JobPriority::High,
        Some("critical") => JobPriority::Critical,
        _ => JobPriority::Normal,
    };

    let mut filter = TrainingDataFilter::default();
    filter.min_quality_score = req.min_quality_score.or(Some(0.7));
    filter.domain_tags = req.domain_tags.clone();
    filter.max_examples = req.max_examples.or(Some(10_000));

    // Map Ollama tags to HuggingFace model IDs
    // Returns (model_id, requires_auth, recommended_cpu_fallback)
    fn map_ollama_tag_to_hf(tag: &str) -> (&str, bool, Option<&'static str>) {
        let t = tag.to_lowercase();
        
        // Qwen models - open, no auth required
        if t.contains("qwen2.5:7b") || t.contains("qwen2.5-7b") { 
            return ("Qwen/Qwen2.5-7B-Instruct", false, Some("Qwen/Qwen2.5-3B-Instruct")); 
        }
        if t.contains("qwen2.5:3b") || t.contains("qwen2.5-3b") { 
            return ("Qwen/Qwen2.5-3B-Instruct", false, Some("Qwen/Qwen2.5-1.5B-Instruct")); 
        }
        if t.contains("qwen2.5:1.5b") || t.contains("qwen2.5-1.5b") { 
            return ("Qwen/Qwen2.5-1.5B-Instruct", false, None); 
        }
        if t.contains("qwen2.5") {
            return ("Qwen/Qwen2.5-7B-Instruct", false, Some("Qwen/Qwen2.5-3B-Instruct"));
        }
        
        // Llama models - require HuggingFace auth
        if t.contains("llama3.2:3b") || t.contains("llama-3.2-3b") { 
            return ("meta-llama/Llama-3.2-3B-Instruct", true, Some("Qwen/Qwen2.5-3B-Instruct")); 
        }
        if t.contains("llama3.2:1b") || t.contains("llama-3.2-1b") { 
            return ("meta-llama/Llama-3.2-1B-Instruct", true, Some("Qwen/Qwen2.5-1.5B-Instruct")); 
        }
        if t.contains("llama3.1:8b") || t.contains("llama-3.1-8b") { 
            return ("meta-llama/Llama-3.1-8B-Instruct", true, Some("Qwen/Qwen2.5-7B-Instruct")); 
        }
        if t.contains("llama") {
            return ("meta-llama/Llama-3.2-3B-Instruct", true, Some("Qwen/Qwen2.5-3B-Instruct"));
        }
        
        // Mistral models - require auth
        if t.contains("mistral:7b") || t.contains("mistral-7b") { 
            return ("mistralai/Mistral-7B-Instruct-v0.3", true, Some("Qwen/Qwen2.5-7B-Instruct")); 
        }
        if t.contains("mistral") {
            return ("mistralai/Mistral-7B-Instruct-v0.3", true, Some("Qwen/Qwen2.5-7B-Instruct"));
        }
        
        // Default: try to use the tag as-is, assume no auth, no fallback
        (tag, false, None)
    }
    
    let (base_model_id, requires_auth, cpu_fallback) = map_ollama_tag_to_hf(&req.base_model_path);
    let base_model_id = base_model_id.to_string();
    
    // Check if HuggingFace token is available for gated models
    let hf_token = if requires_auth {
        goose::config::Config::global().get_param::<String>("HUGGINGFACE_TOKEN").ok()
    } else {
        None
    };
    
    // If model requires auth but no token is available, use CPU fallback
    let (final_model_id, use_cpu_training) = if requires_auth && hf_token.is_none() {
        if let Some(fallback) = cpu_fallback {
            warn!("Model {} requires HuggingFace authentication but no token found. Using fallback model: {}", base_model_id, fallback);
            (fallback.to_string(), false)
        } else {
            warn!("Model {} requires HuggingFace authentication but no token found. Training may fail.", base_model_id);
            (base_model_id, false)
        }
    } else {
        (base_model_id, false)
    };

    let mut builder = TrainingJobBuilder::new(
        "axolotl-training".to_string(),
        std::path::PathBuf::from(final_model_id.clone()),
    )
    .priority(priority)
    .training_data_filter(filter.clone());

    // Apply config overrides if provided
    let mut config = goose::model_training::trainer::TrainingConfig::default();
    if let Some(ovr) = req.config_overrides.clone() { config = ovr; }
    
    // Add HuggingFace token and CPU training flag if needed
    if let Some(token) = hf_token {
        // Store token in config metadata for training script
        config.metadata.insert("hf_token".to_string(), token);
    }
    if use_cpu_training {
        config.metadata.insert("use_cpu".to_string(), "true".to_string());
    }
    
    builder = builder.config(config);

    let job = builder.build();

    // Fetch training examples from storage and inject them into the job manager
    let all_examples = state
        .training_state
        .storage
        .get_examples_for_training(None, None, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Filter examples based on the training data filter
    let filtered_examples: Vec<TrainingExample> = all_examples
        .into_iter()
        .filter(|ex| {
            // Filter by quality score
            if let Some(min_score) = filter.min_quality_score {
                if ex.quality_metrics.overall_score < min_score {
                    return false;
                }
            }
            
            // Filter by domain tags
            if let Some(ref tags) = filter.domain_tags {
                if !tags.is_empty() && !ex.domain_tags.iter().any(|t| tags.contains(t)) {
                    return false;
                }
            }
            
            true
        })
        .take(filter.max_examples.unwrap_or(10_000))
        .collect();
    
    info!("Filtered {} training examples for job", filtered_examples.len());
    
    if filtered_examples.is_empty() {
        warn!("No training examples available for job");
        return Err(StatusCode::BAD_REQUEST);
    }

    let job_id = JOB_MANAGER
        .submit_job_with_data(job, filtered_examples)
        .await
        .map_err(|e| {
            warn!("Failed to submit job: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(StartTrainingResponse { job_id: job_id.to_string() }))
}

pub async fn job_progress(axum::extract::Path(job_id): axum::extract::Path<String>) -> Result<Json<ProgressResponse>, StatusCode> {
    let id = Uuid::parse_str(&job_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let updates = JOB_MANAGER.get_progress(id).await.unwrap_or_default();
    Ok(Json(ProgressResponse { updates }))
}

pub async fn activate_adapter(Json(req): Json<ActivateAdapterRequest>) -> Result<Json<String>, StatusCode> {
    // Resolve adapter path
    let adapter_path: Option<String> = if let Some(path) = req.lora_path.clone() {
        Some(path)
    } else if let Some(job_id_str) = req.job_id.clone() {
        match Uuid::parse_str(&job_id_str) {
            Ok(job_id) => {
                let runtime = goose::model_training::axolotl::AxolotlRuntime::default();
                let run_dir = runtime.output_root.join(format!("job-{}", job_id));
                // Try common filename first
                let candidate = run_dir.join("adapter_model.safetensors");
                if candidate.exists() {
                    Some(candidate.display().to_string())
                } else {
                    // Fallback: search immediate children for adapter
                    if let Ok(mut rd) = tokio::fs::read_dir(&run_dir).await {
                        let mut found: Option<String> = None;
                        while let Ok(Some(entry)) = rd.next_entry().await {
                            let p = entry.path().join("adapter_model.safetensors");
                            if p.exists() {
                                found = Some(p.display().to_string());
                                break;
                            }
                        }
                        found
                    } else {
                        None
                    }
                }
            }
            Err(_) => None,
        }
    } else {
        None
    };

    let adapter_path = adapter_path.ok_or(StatusCode::BAD_REQUEST)?;
    info!("Activating LoRA adapter at {}", adapter_path);

    // Persist adapter path so NativeModelProvider picks it up on next init
    let upsert = serde_json::json!({
        "key": "NATIVE_LORA_ADAPTER_PATH",
        "value": serde_json::Value::String(adapter_path),
        "is_secret": false,
    });
    let client = reqwest::Client::new();
    let res = client
        .post("http://localhost:3000/config/upsert")
        .json(&upsert)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !res.status().is_success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json("activated".to_string()))
}

pub async fn list_examples(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Result<Json<serde_json::Value>, StatusCode> {
    let examples = state
        .training_state
        .storage
        .get_examples_for_training(None, None, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let count = examples.len();
    let summary = serde_json::json!({
        "count": count,
        "examples": examples.iter().take(100).map(|ex| serde_json::json!({
            "id": ex.id,
            "conversation_id": ex.conversation_id,
            "created_at": ex.created_at,
            "quality_score": ex.quality_metrics.overall_score,
            "message_count": ex.messages.len(),
        })).collect::<Vec<_>>()
    });

    Ok(Json(summary))
}

#[derive(Debug, Deserialize)]
pub struct ImportJsonlRequest {
    pub url: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ImportJsonlResponse {
    pub imported: usize,
    pub errors: usize,
}

#[axum::debug_handler]
pub async fn import_jsonl(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    axum::Json(req): axum::Json<ImportJsonlRequest>,
) -> Result<axum::Json<ImportJsonlResponse>, StatusCode> {
    use goose::training_data::schema::TrainingExample;
    use goose::training_data::storage::TrainingDataStorage;

    // Canonicalize common Hugging Face UI URLs to raw file URLs
    let mut url = req.url.trim().to_string();
    if url.contains("huggingface.co/") && url.contains("/blob/") {
        url = url.replace("/blob/", "/resolve/");
    }

    let resp = reqwest::get(&url)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    if !resp.status().is_success() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let text = resp.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut imported = 0usize;
    let mut errors = 0usize;

    // Collect lines into owned Strings to avoid holding non-Send iterators across await points
    let lines_iter = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|s| s.to_string());
    let lines: Vec<String> = if let Some(limit) = req.limit {
        lines_iter.take(limit).collect()
    } else {
        lines_iter.collect()
    };

    // helpers to coerce various dataset shapes into Message[]
    let coerce_messages = |val: &serde_json::Value| -> Option<Vec<goose::conversation::message::Message>> {
        // 1) Direct messages array
        if let Some(messages_val) = val.get("messages").cloned() {
            if let Ok(v) = serde_json::from_value::<Vec<goose::conversation::message::Message>>(messages_val) {
                if !v.is_empty() { return Some(v); }
            }
        }
        // helper to build a Message from role/text
        let mk = |role: &str, text: &str| -> Option<goose::conversation::message::Message> {
            if text.trim().is_empty() { return None; }
            // normalize role: map unknowns (including system) to user
            let r = match role.to_lowercase().as_str() {
                "assistant" => "assistant",
                "user" => "user",
                _ => "user",
            };
            serde_json::from_value(serde_json::json!({"role": r, "content": text})).ok()
        };
        // 1b) messages array with arbitrary roles (normalize)
        if let Some(arr) = val.get("messages").and_then(|v| v.as_array()) {
            let mut out: Vec<goose::conversation::message::Message> = vec![];
            for el in arr {
                let role = el.get("role").and_then(|v| v.as_str()).unwrap_or("");
                let text = el.get("content").and_then(|v| v.as_str())
                    .or_else(|| el.get("text").and_then(|v| v.as_str()))
                    .unwrap_or("");
                if let Some(m) = mk(role, text) { out.push(m); }
            }
            if !out.is_empty() { return Some(out); }
        }
        // 2) conversations/turns/dialogue arrays
        for key in ["conversations", "conversation", "turns", "dialogue", "utterances", "history"] {
            if let Some(arr) = val.get(key).and_then(|v| v.as_array()) {
                let mut out: Vec<goose::conversation::message::Message> = vec![];
                for el in arr {
                    let role_raw = el.get("role").and_then(|v| v.as_str())
                        .or_else(|| el.get("from").and_then(|v| v.as_str()))
                        .or_else(|| el.get("speaker").and_then(|v| v.as_str()))
                        .or_else(|| el.get("author").and_then(|v| v.as_str()))
                        .or_else(|| el.get("name").and_then(|v| v.as_str()))
                        .unwrap_or("");
                    let text = el.get("content").and_then(|v| v.as_str())
                        .or_else(|| el.get("value").and_then(|v| v.as_str()))
                        .or_else(|| el.get("text").and_then(|v| v.as_str()))
                        .or_else(|| el.get("utterance").and_then(|v| v.as_str()))
                        .unwrap_or("");
                    let role_lc = role_raw.to_lowercase();
                    let role_norm: &str = match role_lc.as_str() {
                        "human" | "user" | "customer" => "user",
                        "assistant" | "gpt" | "agent" | "bot" => "assistant",
                        other if !other.is_empty() => other,
                        _ => "user",
                    };
                    if let Some(m) = mk(role_norm, text) { out.push(m); }
                }
                if !out.is_empty() { return Some(out); }
            }
        }
        // 3) prompt/instruction + output-like fields
        let user_text = val.get("instruction").and_then(|v| v.as_str())
            .or_else(|| val.get("prompt").and_then(|v| v.as_str()))
            .or_else(|| val.get("question").and_then(|v| v.as_str()))
            .or_else(|| val.get("query").and_then(|v| v.as_str()))
            .or_else(|| val.get("input").and_then(|v| v.as_str()))
            .or_else(|| val.get("user_input").and_then(|v| v.as_str()));
        let assistant_text = val.get("final_answer").and_then(|v| v.as_str())
            .or_else(|| val.get("output").and_then(|v| v.as_str()))
            .or_else(|| val.get("response").and_then(|v| v.as_str()))
            .or_else(|| val.get("answer").and_then(|v| v.as_str()))
            .or_else(|| val.get("completion").and_then(|v| v.as_str()))
            .or_else(|| val.get("target").and_then(|v| v.as_str()))
            .or_else(|| val.get("assistant").and_then(|v| v.as_str()));
        if let (Some(u), Some(a)) = (user_text, assistant_text) {
            let mut out = vec![];
            if let Some(m1) = mk("user", u) { out.push(m1); }
            if let Some(m2) = mk("assistant", a) { out.push(m2); }
            if !out.is_empty() { return Some(out); }
        }
        // 4) explicit user/assistant fields
        let u = val.get("user").and_then(|v| v.as_str());
        let a = val.get("assistant").and_then(|v| v.as_str()).or_else(|| val.get("agent").and_then(|v| v.as_str()));
        if let (Some(u), Some(a)) = (u, a) {
            let mut out = vec![];
            if let Some(m1) = mk("user", u) { out.push(m1); }
            if let Some(m2) = mk("assistant", a) { out.push(m2); }
            if !out.is_empty() { return Some(out); }
        }
        None
    };

    // Derive dataset tag from URL basename
    let dataset_tag = {
        let mut name = url.split('/').last().unwrap_or("dataset").to_string();
        if let Some(stripped) = name.strip_suffix(".jsonl") { name = stripped.to_string(); }
        format!("dataset:{}", name)
    };

    for (idx, line) in lines.into_iter().enumerate() {
        let val: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => { errors += 1; continue; }
        };

        let Some(messages) = coerce_messages(&val) else { errors += 1; continue; };

        let conversation_id = val
            .get("conversation_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("import#{}", idx));

        let provider = val
            .get("provider_used")
            .and_then(|v| v.as_str())
            .unwrap_or("native_model")
            .to_string();

        let model = val
            .get("model_used")
            .and_then(|v| v.as_str())
            .unwrap_or(
                &goose::providers::base::get_current_model().unwrap_or_else(|| "qwen2.5-7b".to_string())
            )
            .to_string();

        let mut example = TrainingExample::new(conversation_id, messages, provider, model);

        if let Some(tags) = val.get("domain_tags").and_then(|v| v.as_array()) {
            example.domain_tags = tags.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect();
        }
        if let Some(r) = val.get("rating").and_then(|v| v.as_u64()) {
            example.quality_metrics.overall_score = match r { 5 => 1.0, 4 => 0.8, 3 => 0.6, 2 => 0.4, 1 => 0.2, _ => 0.5 };
        }
        if let Some(c) = val.get("correction") { example.metadata.custom_fields.insert("correction".into(), c.clone()); }
        if let Some(c) = val.get("comments") { example.metadata.custom_fields.insert("comments".into(), c.clone()); }

        if let Err(_) = state.training_state.storage.store_example(example).await {
            errors += 1; continue;
        }
        imported += 1;
    }

    Ok(axum::Json(ImportJsonlResponse { imported, errors }))
}

#[derive(Debug, Serialize)]
pub struct CheckAxolotlResponse {
    pub installed: bool,
    pub accelerate_available: bool,
    pub axolotl_available: bool,
    pub python_path: String,
    pub error: Option<String>,
}

pub async fn check_axolotl() -> Result<Json<CheckAxolotlResponse>, StatusCode> {
    let runtime = goose::model_training::axolotl::AxolotlRuntime::default();
    let py = &runtime.python;
    
    // Check accelerate
    let mut accel_cmd = tokio::process::Command::new(py);
    accel_cmd.arg("-c").arg("import accelerate; print(accelerate.__version__)");
    let accel_result = accel_cmd.output().await;

    let accelerate_available = accel_result
        .map(|o| o.status.success())
        .unwrap_or(false);
    
    // Check transformers, peft, and torch (core training libraries)
    let mut tf_cmd = tokio::process::Command::new(py);
    tf_cmd.arg("-c").arg("import transformers, peft, torch; print('ok')");
    let tf_result = tf_cmd.output().await;
    let transformers_available = tf_result
        .map(|o| o.status.success())
        .unwrap_or(false);
    
    // Check PyYAML (needed for config parsing)
    let mut yaml_cmd = tokio::process::Command::new(py);
    yaml_cmd.arg("-c").arg("import yaml; print('ok')");
    let yaml_result = yaml_cmd.output().await;
    let yaml_available = yaml_result
        .map(|o| o.status.success())
        .unwrap_or(false);
    
    // We consider "installed" if accelerate + transformers + peft + torch + yaml are available
    // We no longer require Axolotl itself due to Python 3.14 compatibility issues
    let installed = accelerate_available && transformers_available && yaml_available;
    
    let error = if !installed {
        let mut msgs = vec![];
        if !accelerate_available {
            msgs.push("accelerate not available");
        }
        if !transformers_available {
            msgs.push("transformers/peft/torch not available");
        }
        if !yaml_available {
            msgs.push("PyYAML not available");
        }
        Some(msgs.join(", "))
    } else {
        None
    };
    
    Ok(Json(CheckAxolotlResponse {
        installed,
        accelerate_available,
        axolotl_available: transformers_available, // Use transformers as proxy
        python_path: py.clone(),
        error,
    }))
}

#[derive(Debug, Serialize)]
pub struct InstallAxolotlResponse {
    pub success: bool,
    pub message: String,
    pub log: Option<String>,
}

pub async fn install_axolotl() -> Result<Json<InstallAxolotlResponse>, StatusCode> {
    use tokio::io::AsyncWriteExt;
    
    info!("Starting Axolotl installation...");
    
    // Get the venv path
    let home = dirs::home_dir().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let venv_dir = home.join(".config/goose/axolotl-venv");
    let python_path = venv_dir.join("bin/python");
    
    // Check if already installed
    if python_path.exists() {
        let mut check_cmd = tokio::process::Command::new(&python_path);
        check_cmd.arg("-c").arg("import accelerate, transformers, peft, torch, yaml");
        if let Ok(output) = check_cmd.output().await {
            if output.status.success() {
                return Ok(Json(InstallAxolotlResponse {
                    success: true,
                    message: "Training libraries are already installed".to_string(),
                    log: None,
                }));
            }
        }
    }
    
    // Create venv
    info!("Creating virtual environment at {:?}", venv_dir);
    tokio::fs::create_dir_all(venv_dir.parent().unwrap()).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut venv_cmd = tokio::process::Command::new("python3");
    venv_cmd.arg("-m").arg("venv").arg(&venv_dir);
    let venv_output = venv_cmd.output().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if !venv_output.status.success() {
        let error = String::from_utf8_lossy(&venv_output.stderr);
        return Ok(Json(InstallAxolotlResponse {
            success: false,
            message: "Failed to create virtual environment".to_string(),
            log: Some(error.to_string()),
        }));
    }
    
    // Upgrade pip
    info!("Upgrading pip...");
    let mut pip_upgrade = tokio::process::Command::new(&python_path);
    pip_upgrade.arg("-m").arg("pip").arg("install").arg("--upgrade").arg("pip").arg("--quiet");
    let _ = pip_upgrade.output().await;
    
    // Check if uv is available (better for Python 3.14+)
    let uv_available = tokio::process::Command::new("uv")
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    
    if uv_available {
        info!("Using uv for installation (Python 3.14+ compatible)");
        
        // Recreate venv with uv
        let mut uv_venv = tokio::process::Command::new("uv");
        uv_venv.arg("venv").arg(&venv_dir).arg("--force");
        let _ = uv_venv.output().await;
        
        // Install core training packages with uv
        let mut uv_install = tokio::process::Command::new("uv");
        uv_install.arg("pip").arg("install")
            .arg("--python").arg(&python_path)
            .arg("accelerate").arg("torch").arg("transformers")
            .arg("datasets").arg("peft").arg("pyyaml");
        uv_install.stdout(std::process::Stdio::piped());
        uv_install.stderr(std::process::Stdio::piped());
        
        let install_output = uv_install.output().await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        let stdout = String::from_utf8_lossy(&install_output.stdout);
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        let combined_log = format!("uv installation:\n{}\n{}", stdout, stderr);
        
        if install_output.status.success() {
            info!("Training libraries installed successfully with uv");
            return Ok(Json(InstallAxolotlResponse {
                success: true,
                message: "Training libraries installed successfully! (accelerate, transformers, peft)".to_string(),
                log: Some(combined_log),
            }));
        } else {
            warn!("uv installation failed, falling back to pip");
        }
    }
    
    // Fallback to pip with multiple strategies
    info!("Installing with pip...");
    
    let strategies = vec![
        // Strategy 1: Install core packages only (skip axolotl due to Python 3.14 issues)
        vec!["install", "--no-cache-dir", "accelerate", "torch", "transformers", "datasets", "peft", "pyyaml"],
        // Strategy 2: With --no-deps
        vec!["install", "--no-cache-dir", "--no-deps", "accelerate", "transformers", "peft", "pyyaml"],
    ];
    
    let mut last_error = String::new();
    
    for (i, strategy) in strategies.iter().enumerate() {
        info!("Trying pip strategy {}/{}", i + 1, strategies.len());
        
        let mut install_cmd = tokio::process::Command::new(&python_path);
        install_cmd.arg("-m").arg("pip");
        for arg in strategy {
            install_cmd.arg(arg);
        }
        install_cmd.stdout(std::process::Stdio::piped());
        install_cmd.stderr(std::process::Stdio::piped());
        
        let install_output = install_cmd.output().await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        let stdout = String::from_utf8_lossy(&install_output.stdout);
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        let combined_log = format!("Strategy {}: {}\n{}\n{}", i + 1, strategy.join(" "), stdout, stderr);
        
        if install_output.status.success() {
            info!("Training libraries installed successfully with pip strategy {}", i + 1);
            return Ok(Json(InstallAxolotlResponse {
                success: true,
                message: format!("Training libraries installed successfully using pip strategy {}!", i + 1),
                log: Some(combined_log),
            }));
        }
        
        last_error = combined_log;
    }
    
    // All strategies failed
    warn!("All installation strategies failed");
    Ok(Json(InstallAxolotlResponse {
        success: false,
        message: "Installation failed. Python 3.14 has compatibility issues with pip. Try manually with: uv pip install accelerate transformers peft".to_string(),
        log: Some(last_error),
    }))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/training/feedback", post(submit_feedback))
        .route("/training/preference", post(submit_preference))
        .route("/training/start", post(start_training))
        .route("/training/jobs", get(list_jobs))
        .route("/training/progress/{job_id}", get(job_progress))
        .route("/training/activate", post(activate_adapter))
        .route("/training/examples", get(list_examples))
        .route("/training/check-axolotl", get(check_axolotl))
        .route("/training/install-axolotl", post(install_axolotl))
        .route("/training/import/jsonl", post(import_jsonl))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
pub struct SubmitPreferenceRequest {
    pub prompt: String,
    pub chosen: String,
    pub rejected: String,
}

pub async fn submit_preference(Json(req): Json<SubmitPreferenceRequest>) -> Result<Json<String>, StatusCode> {
    use tokio::io::AsyncWriteExt;
    
    // Append to preferences.jsonl under output_root so axolotl exporter can pick it up
    let runtime = goose::model_training::axolotl::AxolotlRuntime::default();
    let pref_path = runtime.output_root.join("preferences.jsonl");
    
    if let Some(parent) = pref_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    
    let line = serde_json::json!({
        "prompt": req.prompt,
        "chosen": req.chosen,
        "rejected": req.rejected,
    }).to_string() + "\n";
    
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&pref_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    file.write_all(line.as_bytes())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json("ok".into()))
}
