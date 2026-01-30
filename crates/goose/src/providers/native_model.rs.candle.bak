use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use crate::conversation::message::Message;
use crate::impl_provider_default;
use crate::model::ModelConfig;
use anyhow::Result;
// use async_stream::try_stream; // no longer used
use async_trait::async_trait;
use candle_core::{DType as CDType, Device, Tensor};
use candle_nn as nn;
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::qwen2::{Config as QwenConfig, ModelForCausalLM as QwenLM};
use futures::StreamExt;
use rmcp::model::Tool;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub const NATIVE_DEFAULT_MODEL: &str = "qwen2.5-7b";
pub const NATIVE_KNOWN_MODELS: &[&str] = &[
    "llama-3.2-1b-instruct",
    "llama-3.2-3b-instruct",
    "qwen2.5-3b-instruct",
    "qwen2.5-7b-instruct",
    "qwen2.5-7b",
];
pub const NATIVE_DOC_URL: &str = "https://github.com/huggingface/candle";

#[derive(Debug, Clone, Deserialize)]
struct ModelIndexJson {
    #[serde(default)]
    weight_map: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
struct NativeGenConfig {
    max_tokens: usize,
    temperature: f64,
    top_p: f64,
    top_k: usize,
    device: String, // "cpu" | "metal"
    dtype: String,  // "f16" | "bf16"
}

impl NativeGenConfig {
    fn from_env() -> Self {
        let cfg = crate::config::Config::global();
        Self {
            max_tokens: cfg.get_param("NATIVE_MAX_TOKENS").unwrap_or(256),
            temperature: cfg.get_param("NATIVE_TEMPERATURE").unwrap_or(0.7),
            top_p: cfg.get_param("NATIVE_TOP_P").unwrap_or(0.9),
            top_k: cfg.get_param("NATIVE_TOP_K").unwrap_or(40),
            device: cfg
                .get_param("NATIVE_DEVICE")
                .unwrap_or_else(|_| "cpu".to_string()),
            dtype: cfg
                .get_param("NATIVE_DTYPE")
                .unwrap_or_else(|_| "f16".to_string()),
        }
    }
}

/// Provider for locally hosted models (Phase A: load + validate Qwen files, wire streaming, clear errors)
#[derive(serde::Serialize)]
pub struct NativeModelProvider {
    model: ModelConfig,
    model_path: PathBuf,
    lora_adapter_path: Option<PathBuf>,
}

impl_provider_default!(NativeModelProvider);

impl NativeModelProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        // Get model path from environment
        let model_path = config
            .get_param("NATIVE_MODEL_PATH")
            .unwrap_or_else(|_| "/Users/spencermartin/Desktop/Distil/models".to_string());

        tracing::info!("Creating Native Model Provider with path: {}", model_path);

        // Check for adapter path in config (allows activation between sessions)
        let lora_adapter_path = config
            .get_param::<String>("NATIVE_LORA_ADAPTER_PATH")
            .ok()
            .map(PathBuf::from);

        Ok(Self {
            model,
            model_path: PathBuf::from(model_path),
            lora_adapter_path,
        })
    }

    fn qwen_chat_template(&self, system: &str, messages: &[Message]) -> String {
        // Qwen2 style chat template
        // <|im_start|>system\n...<|im_end|>\n<|im_start|>user\n...<|im_end|>\n...<|im_start|>assistant\n
        let mut out = String::new();
        if !system.trim().is_empty() {
            out.push_str("<|im_start|>system\n");
            out.push_str(system.trim());
            out.push_str("\n<|im_end|>\n");
        }
        for m in messages {
            match m.role {
                rmcp::model::Role::User => {
                    out.push_str("<|im_start|>user\n");
                    out.push_str(&m.as_concat_text());
                    out.push_str("\n<|im_end|>\n");
                }
                rmcp::model::Role::Assistant => {
                    out.push_str("<|im_start|>assistant\n");
                    out.push_str(&m.as_concat_text());
                    out.push_str("\n<|im_end|>\n");
                }
            }
        }
        out.push_str("<|im_start|>assistant\n");
        out
    }

    fn validate_qwen_files(&self) -> Result<(PathBuf, PathBuf, Vec<PathBuf>), ProviderError> {
        let base: &Path = &self.model_path;
        let tokenizer = base.join("tokenizer.json");
        let config_json = base.join("config.json");
        let index = base.join("model.safetensors.index.json");

        for p in [&tokenizer, &config_json, &index] {
            if !p.exists() {
                return Err(ProviderError::ExecutionError(format!(
                    "Missing required file: {}",
                    p.display()
                )));
            }
        }

        let index_str = fs::read_to_string(&index).map_err(|e| {
            ProviderError::ExecutionError(format!(
                "Failed reading index {}: {}",
                index.display(),
                e
            ))
        })?;
        let val: serde_json::Value = serde_json::from_str(&index_str).map_err(|e| {
            ProviderError::ExecutionError(format!("Invalid safetensors index JSON: {}", e))
        })?;

        let mut shards = Vec::new();
        if let Some(map) = val.get("weight_map").and_then(|v| v.as_object()) {
            use std::collections::BTreeSet;
            let mut set = BTreeSet::new();
            for v in map.values() {
                if let Some(s) = v.as_str() {
                    set.insert(s.to_string());
                }
            }
            for s in set {
                let p = base.join(s);
                if !p.exists() {
                    return Err(ProviderError::ExecutionError(format!(
                        "Shard listed in index not found: {}",
                        p.display()
                    )));
                }
                shards.push(p);
            }
        } else {
            return Err(ProviderError::ExecutionError(
                "safetensors index missing 'weight_map'".to_string(),
            ));
        }

        Ok((tokenizer, config_json, shards))
    }

    fn device_and_dtype(
        &self,
        gen_cfg: &NativeGenConfig,
    ) -> Result<(Device, CDType), ProviderError> {
        let device = match gen_cfg.device.as_str() {
            "metal" => Device::new_metal(0)
                .map_err(|e| ProviderError::ExecutionError(format!("Metal init failed: {e}")))?,
            _ => Device::Cpu,
        };
        let dtype = match gen_cfg.dtype.as_str() {
            "bf16" => CDType::BF16,
            "f16" => CDType::F16,
            _ => CDType::F16,
        };
        Ok((device, dtype))
    }

    fn load_tokenizer(&self, tokenizer_path: &Path) -> Result<Tokenizer, ProviderError> {
        Tokenizer::from_file(tokenizer_path).map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to load tokenizer.json: {e}"))
        })
    }

    fn estimate_usage(&self, prompt: &str, response_text: &str) -> Usage {
        Usage {
            input_tokens: Some((prompt.len() / 4) as i32),
            output_tokens: Some((response_text.len() / 4) as i32),
            total_tokens: None,
        }
    }
}

impl NativeModelProvider {
    // Apply top-k filtering to a 1D logits tensor by masking all but the top-k values
    fn apply_top_k(logits: &Tensor, k: usize, device: &Device) -> Result<Tensor, ProviderError> {
        if k == 0 {
            return Ok(logits.clone());
        }
        let v = logits
            .to_vec1::<f32>()
            .map_err(|e| ProviderError::ExecutionError(format!("Top-k: to_vec failed: {e}")))?;
        let vocab = v.len();
        if k >= vocab {
            return Ok(logits.clone());
        }
        // Find indices of top-k values (descending)
        let mut idx: Vec<usize> = (0..vocab).collect();
        idx.sort_unstable_by(|&a, &b| v[b].partial_cmp(&v[a]).unwrap_or(std::cmp::Ordering::Equal));
        // Mask non-top-k logits to a very negative value
        let mut keep = vec![false; vocab];
        for &i in idx.iter().take(k) {
            keep[i] = true;
        }
        let minf = -1e9f32;
        let mut masked = Vec::with_capacity(vocab);
        for i in 0..vocab {
            masked.push(if keep[i] { v[i] } else { minf });
        }
        Tensor::from_vec(masked, vocab, device)
            .map_err(|e| ProviderError::ExecutionError(format!("Top-k: tensor build failed: {e}")))
    }
}

#[async_trait]
impl Provider for NativeModelProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "native_model",
            "Native Model",
            "Locally hosted models using candle-core - no Ollama required",
            NATIVE_DEFAULT_MODEL,
            NATIVE_KNOWN_MODELS.to_vec(),
            NATIVE_DOC_URL,
            vec![
                ConfigKey::new(
                    "NATIVE_MODEL_PATH",
                    true,
                    false,
                    Some("/Users/spencermartin/Desktop/Distil/models"),
                ),
                ConfigKey::new("NATIVE_DEVICE", false, false, Some("cpu")),
                ConfigKey::new("NATIVE_DTYPE", false, false, Some("f16")),
                ConfigKey::new("NATIVE_MAX_TOKENS", false, false, Some("256")),
                ConfigKey::new("NATIVE_TEMPERATURE", false, false, Some("0.7")),
                ConfigKey::new("NATIVE_TOP_P", false, false, Some("0.9")),
                ConfigKey::new("NATIVE_TOP_K", false, false, Some("40")),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn complete_with_model(
        &self,
        _model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        tracing::info!(
            "Native Model Provider: request with {} messages",
            messages.len()
        );

        let gen_cfg = NativeGenConfig::from_env();
        // Validate files early and return clear error if missing
        let (_tokenizer_path, _config_path, shards) = self.validate_qwen_files()?;
        if shards.is_empty() {
            return Err(ProviderError::ExecutionError(
                "No safetensors shards discovered".to_string(),
            ));
        }

        // Format prompt using Qwen chat template
        let prompt = self.qwen_chat_template(system, messages);

        // PHASE A minimal: we confirm files + config are sound. Generation wiring will be added next phase.
        // For now, reply with a clear status message and echo last user content to enable in-situ UI testing.
        let mut response_text = String::new();
        response_text.push_str("[NativeModel] Qwen model detected and validated.\n");
        response_text.push_str(&format!(
            "• Device: {} | DType: {}\n",
            gen_cfg.device, gen_cfg.dtype
        ));
        response_text.push_str(&format!(
            "• Sampling: max_tokens={} temp={} top_p={} top_k={}\n",
            gen_cfg.max_tokens, gen_cfg.temperature, gen_cfg.top_p, gen_cfg.top_k
        ));
        response_text.push_str(&format!(
            "• Shards: {} found under {}\n",
            shards.len(),
            self.model_path.display()
        ));
        response_text
            .push_str("\nTemporary note: Candle generation will be enabled next step.\n\n");
        if let Some(last_user) = messages
            .iter()
            .rev()
            .find(|m| m.role == rmcp::model::Role::User)
        {
            response_text.push_str("Echo: ");
            response_text.push_str(&last_user.as_concat_text());
        } else {
            response_text.push_str("Ready.");
        }

        let response_message = Message::assistant().with_text(response_text.clone());
        let usage = self.estimate_usage(&prompt, &response_text);
        let provider_usage = ProviderUsage::new(self.model.model_name.clone(), usage);

        Ok((response_message, provider_usage))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        tracing::info!(
            "Native Model Provider STREAM: request with {} messages",
            messages.len()
        );

        // Clone data needed for the blocking task
        let model_path = self.model_path.clone();
        let model_name = self.model.model_name.clone();
        let system = system.to_string();
        let messages = messages.to_vec();

        let (tx, rx) = mpsc::unbounded_channel::<
            Result<(Option<Message>, Option<ProviderUsage>), ProviderError>,
        >();

        // Send an immediate message to test the stream
        let test_msg = Message::assistant().with_text("🔄 Loading model...");
        let test_usage = Usage {
            input_tokens: Some(0),
            output_tokens: Some(0),
            total_tokens: Some(0),
        };
        let test_provider_usage = ProviderUsage::new(model_name.clone(), test_usage);
        if let Err(e) = tx.send(Ok((Some(test_msg), Some(test_provider_usage)))) {
            tracing::error!("Failed to send test message: channel already closed!");
        } else {
            tracing::info!("Test message sent successfully to stream");
        }

        // Move ALL setup and generation into spawn_blocking to avoid blocking the async runtime
        let _task_handle = tokio::task::spawn_blocking(move || {
            tracing::info!("Native model generation task started - loading model...");

            // All the heavy lifting happens here in the blocking thread pool
                let gen_cfg = NativeGenConfig::from_env();

                // Build prompt - simplified for base model (not instruct-tuned)
                // Base models work better with completion-style prompts
                let mut prompt = String::new();
                
                // Add a simple conversational context
                prompt.push_str("User: ");
                
                // Get the last user message
                if let Some(last_user) = messages.iter().rev().find(|m| m.role == rmcp::model::Role::User) {
                    prompt.push_str(&last_user.as_concat_text());
                } else {
                    prompt.push_str("Hello");
                }
                
                prompt.push_str("\n\nAssistant:");

                // Validate files
                let base: &Path = &model_path;
                let tokenizer_path = base.join("tokenizer.json");
                let config_path = base.join("config.json");
                let index_path = base.join("model.safetensors.index.json");

                for p in [&tokenizer_path, &config_path, &index_path] {
                    if !p.exists() {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Missing required file: {}",
                            p.display()
                        ))));
                        return;
                    }
                }

                let index_str = match fs::read_to_string(&index_path) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Failed reading index: {e}"
                        ))));
                        return;
                    }
                };

                let val: serde_json::Value = match serde_json::from_str(&index_str) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Invalid index JSON: {e}"
                        ))));
                        return;
                    }
                };

                let mut shards = Vec::new();
                if let Some(map) = val.get("weight_map").and_then(|v| v.as_object()) {
                    use std::collections::BTreeSet;
                    let mut set = BTreeSet::new();
                    for v in map.values() {
                        if let Some(s) = v.as_str() {
                            set.insert(s.to_string());
                        }
                    }
                    for s in set {
                        shards.push(base.join(s));
                    }
                }

                // Load tokenizer
                let tokenizer = match Tokenizer::from_file(&tokenizer_path) {
                    Ok(t) => t,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Failed to load tokenizer: {e}"
                        ))));
                        return;
                    }
                };

                // Try Metal first, fall back to CPU if it fails
                let (device, dtype) = match Device::new_metal(0) {
                    Ok(metal_device) => {
                        tracing::info!("Using Metal GPU device");
                        // Use F16 for Metal for better performance
                        (metal_device, CDType::F16)
                    }
                    Err(e) => {
                        tracing::warn!("Metal initialization failed: {}, falling back to CPU", e);
                        (Device::Cpu, CDType::F32)
                    }
                };
                tracing::info!("Device: {:?}, DType: {:?}", device, dtype);

                // Load config
                let cfg_str = match fs::read_to_string(&config_path) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Failed to read config.json: {e}"
                        ))));
                        return;
                    }
                };

                let qwen_cfg: QwenConfig = match serde_json::from_str(&cfg_str) {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Failed to parse config: {e}"
                        ))));
                        return;
                    }
                };

                // Load model
                tracing::info!("Loading model weights...");
                let shard_refs: Vec<&Path> = shards.iter().map(|p| p.as_path()).collect();
                let vb = match unsafe {
                    nn::VarBuilder::from_mmaped_safetensors(&shard_refs, dtype, &device)
                } {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Failed to mmap safetensors: {e}"
                        ))));
                        return;
                    }
                };

                let mut model = match QwenLM::new(&qwen_cfg, vb) {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Failed to init model: {e}"
                        ))));
                        return;
                    }
                };

                tracing::info!("Model loaded, starting generation...");
                tracing::info!("Prompt: {:?}", &prompt[..prompt.len().min(200)]);

                // Encode prompt
                tracing::info!("Encoding prompt...");
                let enc = match tokenizer.encode(prompt.clone(), true) {
                    Ok(e) => e,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Tokenizer encode failed: {e}"
                        ))));
                        return;
                    }
                };

                let mut all_ids: Vec<u32> = enc.get_ids().to_vec();
                let prompt_len = all_ids.len();
                tracing::info!("Encoded prompt: {} tokens", prompt_len);

                let seed = 42u64;
                let mut logits_proc =
                    LogitsProcessor::new(seed, Some(gen_cfg.temperature), Some(gen_cfg.top_p));
                let eos_im_end = tokenizer.token_to_id("<|im_end|>");
                let eos_generic = tokenizer.token_to_id("</s>");
                let eos_endoftext = tokenizer.token_to_id("<|endoftext|>");
                let mut last_decoded = String::new();
                let mut tokens_emitted: usize = 0;
                let min_output_tokens: usize = std::env::var("NATIVE_MIN_OUTPUT_TOKENS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(5);
                let mut past_len = all_ids.len();

                tracing::info!("Running initial forward pass on {} tokens...", prompt_len);
                // Initial forward on full prompt to fill KV cache and obtain first logits
                tracing::info!("Creating input tensor from {} token IDs...", all_ids.len());
                let input0 =
                    match Tensor::new(all_ids.as_slice(), &device).and_then(|t| t.unsqueeze(0)) {
                        Ok(t) => t,
                        Err(e) => {
                            tracing::error!("Tensor build failed: {}", e);
                            let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                                "Tensor build failed: {e}"
                            ))));
                            return;
                        }
                    };
                tracing::info!("Input tensor created successfully, shape: {:?}", input0.shape());
                // Clear any previous cache and run full prompt once to fill internal caches
                tracing::info!("Clearing KV cache...");
                model.clear_kv_cache();
                tracing::info!("KV cache cleared, calling model.forward()...");
                let logits0 = match model.forward(&input0, 0) {
                    Ok(x) => {
                        tracing::info!("model.forward() succeeded! Logits shape: {:?}", x.shape());
                        x
                    },
                    Err(e) => {
                        tracing::error!("model.forward() failed: {}", e);
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Model forward failed: {e}"
                        ))));
                        return;
                    }
                };
                tracing::info!("Squeezing logits tensor...");
                let logits0 = match logits0.squeeze(0).and_then(|t| t.squeeze(0)) {
                    Ok(x) => x,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Squeeze failed: {e}"
                        ))));
                        return;
                    }
                };
                // Always sample on CPU f32 for numerical stability
                let logits0_cpu = match logits0
                    .to_device(&Device::Cpu)
                    .and_then(|t| t.to_dtype(candle_core::DType::F32))
                {
                    Ok(t) => t,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Move logits to CPU failed: {e}"
                        ))));
                        return;
                    }
                };
                // Mask EOS early to ensure visible output
                let mut logits0_vec = match logits0_cpu.to_vec1::<f32>() {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "logits to_vec failed: {e}"
                        ))));
                        return;
                    }
                };
                if tokens_emitted < min_output_tokens {
                    for &eos in [eos_im_end, eos_generic, eos_endoftext].iter().flatten() {
                        let idx = eos as usize;
                        if idx < logits0_vec.len() {
                            logits0_vec[idx] = f32::NEG_INFINITY;
                        }
                    }
                }
                let vocab0 = logits0_vec.len();
                let logits0_masked = match Tensor::from_vec(logits0_vec, vocab0, &Device::Cpu) {
                    Ok(t) => t,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "logits mask build failed: {e}"
                        ))));
                        return;
                    }
                };
                let filtered0 = if gen_cfg.top_k > 0 {
                    match Self::apply_top_k(&logits0_masked, gen_cfg.top_k, &Device::Cpu) {
                        Ok(f) => f,
                        Err(e) => {
                            let _ = tx.send(Err(e));
                            return;
                        }
                    }
                } else {
                    logits0_masked.clone()
                };
                let mut next_id = match logits_proc.sample(&filtered0) {
                    Ok(id) => id as u32,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Sampling failed: {e}"
                        ))));
                        return;
                    }
                };
                all_ids.push(next_id);
                // Emit initial delta from prompt boundary, skipping special tokens
                let decoded0 = match tokenizer.decode(&all_ids[prompt_len..], true) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                            "Decode failed: {e}"
                        ))));
                        return;
                    }
                };
                if !decoded0.trim().is_empty() {
                    last_decoded = decoded0.clone();
                    tokens_emitted += 1;
                    tracing::info!("Sending first token chunk: {:?}", decoded0);
                    let chunk = Message::assistant().with_text(decoded0.clone());
                    let usage = Usage {
                        input_tokens: None,
                        output_tokens: Some((decoded0.len() as i32) / 4),
                        total_tokens: None,
                    };
                    let usage = ProviderUsage::new(model_name.clone(), usage);
                    if let Err(e) = tx.send(Ok((Some(chunk), Some(usage)))) {
                        tracing::error!("Failed to send first chunk: channel closed");
                        return;
                    }
                } else {
                    tracing::warn!("First decoded token was empty or whitespace only");
                }

                // Now generate token-by-token using KV cache
                for _step in 1..=gen_cfg.max_tokens {
                    // Forward only the new token with proper start_pos using KV cache
                    let input_tok =
                        match Tensor::new(&[next_id], &device).and_then(|t| t.unsqueeze(0)) {
                            Ok(t) => t,
                            Err(e) => {
                                let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                                    "Tensor build failed: {e}"
                                ))));
                                break;
                            }
                        };
                    let logits = match model.forward(&input_tok, past_len) {
                        Ok(x) => x,
                        Err(e) => {
                            let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                                "Model forward failed: {e}"
                            ))));
                            break;
                        }
                    };
                    let logits = match logits.squeeze(0).and_then(|t| t.squeeze(0)) {
                        Ok(x) => x,
                        Err(e) => {
                            let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                                "Squeeze failed: {e}"
                            ))));
                            break;
                        }
                    };
                    let logits_cpu = match logits
                        .to_device(&Device::Cpu)
                        .and_then(|t| t.to_dtype(candle_core::DType::F32))
                    {
                        Ok(t) => t,
                        Err(e) => {
                            let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                                "Move logits to CPU failed: {e}"
                            ))));
                            break;
                        }
                    };
                    // Mask EOS early until we emit some visible content
                    let mut logits_vec = match logits_cpu.to_vec1::<f32>() {
                        Ok(v) => v,
                        Err(e) => {
                            let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                                "logits to_vec failed: {e}"
                            ))));
                            break;
                        }
                    };
                    if tokens_emitted < min_output_tokens {
                        for &eos in [eos_im_end, eos_generic, eos_endoftext].iter().flatten() {
                            let idx = eos as usize;
                            if idx < logits_vec.len() {
                                logits_vec[idx] = f32::NEG_INFINITY;
                            }
                        }
                    }
                    let vocab = logits_vec.len();
                    let logits_masked = match Tensor::from_vec(logits_vec, vocab, &Device::Cpu) {
                        Ok(t) => t,
                        Err(e) => {
                            let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                                "logits mask build failed: {e}"
                            ))));
                            break;
                        }
                    };
                    let filtered = if gen_cfg.top_k > 0 {
                        match Self::apply_top_k(&logits_masked, gen_cfg.top_k, &Device::Cpu) {
                            Ok(f) => f,
                            Err(e) => {
                                let _ = tx.send(Err(e));
                                break;
                            }
                        }
                    } else {
                        logits_masked.clone()
                    };
                    next_id = match logits_proc.sample(&filtered) {
                        Ok(id) => id as u32,
                        Err(e) => {
                            let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                                "Sampling failed: {e}"
                            ))));
                            break;
                        }
                    };
                    all_ids.push(next_id);
                    past_len += 1;

                    // Now decode the new generated segment (skip specials) and emit delta
                    let decoded = match tokenizer.decode(&all_ids[prompt_len..], true) {
                        Ok(s) => s,
                        Err(e) => {
                            let _ = tx.send(Err(ProviderError::ExecutionError(format!(
                                "Decode failed: {e}"
                            ))));
                            break;
                        }
                    };
                    let delta = if decoded.starts_with(&last_decoded) {
                        decoded[last_decoded.len()..].to_string()
                    } else {
                        decoded.clone()
                    };
                    if !delta.trim().is_empty() {
                        last_decoded = decoded;
                        tokens_emitted += 1;
                        let chunk = Message::assistant().with_text(delta.clone());
                        let usage = Usage {
                            input_tokens: None,
                            output_tokens: Some((delta.len() as i32) / 4),
                            total_tokens: None,
                        };
                        let usage = ProviderUsage::new(model_name.clone(), usage);
                        let _ = tx.send(Ok((Some(chunk), Some(usage))));
                    }

                    let is_eos = eos_im_end == Some(next_id)
                        || eos_generic == Some(next_id)
                        || eos_endoftext == Some(next_id);
                    if is_eos && tokens_emitted >= min_output_tokens {
                        tracing::info!(
                            "Generation completed: EOS token detected after {} tokens",
                            tokens_emitted
                        );
                        break;
                    }
                }
                tracing::info!(
                    "Generation task completed, emitted {} tokens",
                    tokens_emitted
                );
        });

        let stream = UnboundedReceiverStream::new(rx).map(|item| item);
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_provider_creation() {
        let model_config = ModelConfig::new("qwen2.5-7b").unwrap();
        let provider = NativeModelProvider::from_env(model_config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_metadata() {
        let metadata = NativeModelProvider::metadata();
        assert_eq!(metadata.name, "native_model");
        assert_eq!(metadata.display_name, "Native Model");
        assert!(metadata.description.contains("candle-core"));
    }
}

impl NativeModelProvider {
    /// Load a PEFT/Axolotl LoRA adapter and prepare to apply during forward
    pub fn load_adapter(&mut self, adapter_path: &str) -> Result<()> {
        let p = PathBuf::from(adapter_path);
        if !p.exists() {
            return Err(anyhow::anyhow!("LoRA adapter not found: {}", adapter_path));
        }
        self.lora_adapter_path = Some(p);
        tracing::info!("LoRA adapter configured for native model: {}", adapter_path);
        Ok(())
    }
}
