use crate::conversation::message::Message;
use anyhow::{anyhow, Result};
use mcp_core::tool::ToolCall;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::security::model_downloader::{get_global_downloader, ModelInfo};

// ML inference backends
use ort::{session::builder::GraphOptimizationLevel, session::Session};
use tokenizers::Tokenizer;

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub is_malicious: bool,
    pub confidence: f32,
    pub explanation: String,
}

/// Trait for different ML inference backends
#[async_trait::async_trait]
pub trait PromptInjectionModel: Send + Sync {
    async fn predict(&self, text: &str) -> Result<(f32, String)>;
    fn model_name(&self) -> &str;
}

/// ONNX Runtime implementation
pub struct OnnxPromptInjectionModel {
    session: Arc<std::sync::Mutex<Session>>,
    tokenizer: Arc<Tokenizer>,
    model_name: String,
}

impl OnnxPromptInjectionModel {
    pub async fn new(
        model_path: PathBuf,
        tokenizer_path: PathBuf,
        model_name: String,
    ) -> Result<Self> {
        tracing::info!("🔒 Starting ONNX model initialization...");

        // Initialize ONNX Runtime session
        tracing::info!("🔒 Creating ONNX session from: {:?}", model_path);
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .commit_from_file(&model_path)?;

        tracing::info!("🔒 ONNX session created successfully");

        // Load tokenizer
        tracing::info!("🔒 Loading tokenizer from: {:?}", tokenizer_path);
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

        tracing::info!("🔒 Tokenizer loaded successfully");

        Ok(Self {
            session: Arc::new(std::sync::Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
            model_name,
        })
    }
}

#[async_trait::async_trait]
impl PromptInjectionModel for OnnxPromptInjectionModel {
    async fn predict(&self, text: &str) -> Result<(f32, String)> {
        tracing::info!("🔒 ONNX predict() called with text length: {}", text.len());
        tracing::info!(
            "🔒 ONNX predict() received text: '{}'",
            text.chars().take(200).collect::<String>()
        );

        // Tokenize the input text
        tracing::debug!("🔒 Tokenizing input text...");
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow!("Tokenization failed: {}", e))?;

        let input_ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();

        tracing::debug!(
            "🔒 Tokenization complete. Sequence length: {}",
            input_ids.len()
        );

        // Convert to the format expected by ONNX (batch_size=1)
        let input_ids: Vec<i64> = input_ids.iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = attention_mask.iter().map(|&mask| mask as i64).collect();

        let seq_len = input_ids.len();

        tracing::debug!("🔒 Creating ONNX tensors...");
        // Create ONNX tensors
        let input_ids_tensor =
            ort::value::Tensor::from_array(([1, seq_len], input_ids.into_boxed_slice()))?;
        let attention_mask_tensor =
            ort::value::Tensor::from_array(([1, seq_len], attention_mask.into_boxed_slice()))?;

        tracing::debug!("🔒 Running ONNX inference...");
        // Run inference and extract the logits immediately
        let (logit_0, logit_1) = {
            let mut session = self
                .session
                .lock()
                .map_err(|e| anyhow!("Failed to lock session: {}", e))?;
            tracing::debug!("🔒 Session locked, running inference...");
            let outputs = session.run(ort::inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor
            ])?;

            tracing::debug!("🔒 Inference complete, extracting logits...");
            // Extract logits from output immediately while we have the lock
            let logits = outputs["logits"].try_extract_tensor::<f32>()?;
            let logits_slice = logits.1;

            // Extract the values we need
            let logit_0 = logits_slice[0]; // Non-injection class
            let logit_1 = logits_slice[1]; // Injection class

            tracing::debug!("🔒 Logits extracted: [{:.3}, {:.3}]", logit_0, logit_1);

            (logit_0, logit_1)
        };

        // Apply softmax to get probabilities
        let exp_0 = logit_0.exp();
        let exp_1 = logit_1.exp();
        let sum_exp = exp_0 + exp_1;

        let prob_injection = exp_1 / sum_exp;

        let explanation = format!(
            "ONNX model '{}': Injection probability = {:.3} (logits: [{:.3}, {:.3}])",
            self.model_name, prob_injection, logit_0, logit_1
        );

        tracing::info!(
            "🔒 ONNX prediction complete: confidence={:.3}, explanation={}",
            prob_injection,
            explanation
        );

        tracing::debug!(
            model = %self.model_name,
            text_length = text.len(),
            seq_length = seq_len,
            logit_0 = logit_0,
            logit_1 = logit_1,
            prob_injection = prob_injection,
            "ONNX inference completed"
        );

        Ok((prob_injection, explanation))
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

type ModelCache = Arc<tokio::sync::RwLock<Option<Arc<dyn PromptInjectionModel>>>>;

/// Global model cache with reload capability
static MODEL_CACHE: OnceCell<ModelCache> = OnceCell::const_new();

/// Initialize the global model
async fn initialize_model() -> Result<Option<Arc<dyn PromptInjectionModel>>> {
    tracing::info!("🔒 Attempting to initialize ONNX security model...");

    // Try to load the ONNX model
    match get_global_downloader().await {
        Ok(downloader) => {
            let model_info = PromptInjectionScanner::get_model_info_from_config();
            match downloader.ensure_model_available(&model_info).await {
                Ok((model_path, tokenizer_path)) => {
                    tracing::info!("🔒 Loading ONNX model from: {:?}", model_path);
                    match OnnxPromptInjectionModel::new(
                        model_path,
                        tokenizer_path,
                        model_info.hf_model_name.clone(),
                    )
                    .await
                    {
                        Ok(model) => {
                            tracing::info!("🔒 ✅ ONNX security model loaded successfully");
                            return Ok(Some(Arc::new(model)));
                        }
                        Err(e) => {
                            tracing::warn!("🔒 Failed to initialize ONNX model: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("🔒 Failed to ensure model available: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::warn!("🔒 Failed to get model downloader: {}", e);
        }
    }

    tracing::info!("🔒 ONNX model not available, will use pattern-based scanning");
    Ok(None)
}

/// Get or initialize the global model
async fn get_model() -> Option<Arc<dyn PromptInjectionModel>> {
    let cache = MODEL_CACHE
        .get_or_init(|| async { Arc::new(tokio::sync::RwLock::new(None)) })
        .await;

    // Check if model is already loaded in memory
    let read_guard = cache.read().await;
    if let Some(model) = read_guard.as_ref() {
        tracing::debug!("🔒 Model found in memory cache, using cached instance");
        return Some(model.clone());
    }
    drop(read_guard);

    // Model not loaded in memory, try to initialize from disk
    tracing::info!("🔒 Model not loaded in memory, loading from disk cache...");
    let mut write_guard = cache.write().await;

    // Double-check in case another task loaded it while we were waiting
    if let Some(model) = write_guard.as_ref() {
        tracing::debug!("🔒 Model was loaded by another task while waiting, using that instance");
        return Some(model.clone());
    }

    // Load the model from disk
    match initialize_model().await {
        Ok(Some(model)) => {
            tracing::info!("🔒 ✅ Model successfully loaded into memory cache");
            *write_guard = Some(model.clone());
            Some(model)
        }
        Ok(None) => {
            tracing::info!("🔒 No model available, using pattern-based fallback");
            None
        }
        Err(e) => {
            tracing::warn!("🔒 Failed to initialize model: {}", e);
            None
        }
    }
}

/// Check if model is available without triggering download/initialization
pub async fn get_model_if_available() -> Option<Arc<dyn PromptInjectionModel>> {
    let cache = MODEL_CACHE
        .get_or_init(|| async { Arc::new(tokio::sync::RwLock::new(None)) })
        .await;

    // Only check if model is already loaded in memory - don't trigger loading
    let read_guard = cache.read().await;
    read_guard.as_ref().cloned()
}

/// Simple prompt injection scanner
/// Uses the existing model_downloader infrastructure
pub struct PromptInjectionScanner {
    enabled: bool,
}

impl PromptInjectionScanner {
    pub fn new() -> Self {
        println!("🔒 PromptInjectionScanner::new() called");

        // Check if models are available, trigger download if needed
        let scanner = Self {
            enabled: Self::check_and_prepare_models(),
        };

        println!("🔒 Scanner enabled: {}", scanner.enabled);

        scanner
    }

    /// Check if models are available and trigger download if needed
    fn check_and_prepare_models() -> bool {
        // Check if models are already cached
        let model_info = Self::get_model_info_from_config();

        // Check if model files exist in cache
        if let Some(cache_dir) = dirs::cache_dir() {
            let security_models_dir = cache_dir.join("goose").join("security_models");
            let model_path = security_models_dir.join(&model_info.onnx_filename);
            let tokenizer_path = security_models_dir.join(&model_info.tokenizer_filename);

            if model_path.exists() && tokenizer_path.exists() {
                tracing::info!(
                    "🔒 Security model files found on disk - loading model into memory now..."
                );

                // Load model into memory immediately at startup
                tokio::spawn(async move {
                    tracing::info!("🔒 Pre-loading security model at startup...");
                    if let Some(_model) = get_model().await {
                        tracing::info!(
                            "🔒 ✅ Security model pre-loaded successfully - ready for scanning"
                        );
                    } else {
                        tracing::warn!("🔒 Failed to pre-load security model");
                    }
                });

                return true;
            }
        }

        // Models not cached - we need to download them
        tracing::info!("🔒 Security model files not found on disk");
        tracing::info!(
            "🔒 Models will be downloaded on first security scan (this may cause a delay)"
        );

        // For now, return true to enable security scanning
        // The models will be downloaded lazily on first scan
        // TODO: Consider blocking startup to download models synchronously
        true
    }

    /// Ensure models are available using the existing model_downloader
    async fn ensure_models_available() {
        tracing::info!("🔒 Ensuring security models are available...");

        match get_global_downloader().await {
            Ok(downloader) => {
                let model_info = Self::get_model_info_from_config();
                match downloader.ensure_model_available(&model_info).await {
                    Ok((model_path, tokenizer_path)) => {
                        tracing::info!(
                            "🔒 ✅ Security models ready: model={:?}, tokenizer={:?}",
                            model_path,
                            tokenizer_path
                        );
                    }
                    Err(e) => {
                        tracing::warn!("🔒 Failed to ensure models available: {}", e);
                        tracing::info!("🔒 Continuing with pattern-based security scanning");
                    }
                }
            }
            Err(e) => {
                tracing::warn!("🔒 Failed to get model downloader: {}", e);
            }
        }
    }

    /// Get model information from config file
    pub fn get_model_info_from_config() -> ModelInfo {
        use crate::config::Config;
        let config = Config::global();

        // Try to get model from config
        let security_config = config.get_param::<serde_json::Value>("security").ok();

        let model_name = security_config
            .as_ref()
            .and_then(|security| security.get("models")?.as_array()?.first())
            .and_then(|model| model.get("model")?.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                tracing::warn!(
                    "🔒 No security model configured, security scanning will be disabled"
                );
                // Return a placeholder that won't work, forcing pattern-only mode
                "no-model-configured".to_string()
            });

        tracing::info!("🔒 Using security model from config: {}", model_name);

        // Create ModelInfo from config
        ModelInfo::from_config_model(&model_name)
    }

    /// Two-step security analysis: scan tool call first, then analyze context if suspicious
    pub async fn analyze_tool_call_with_context(
        &self,
        tool_call: &ToolCall,
        messages: &[Message],
    ) -> Result<ScanResult> {
        // Step 1: Scan the tool call itself for suspicious patterns
        let tool_call_result = self.scan_tool_call_only(tool_call).await?;

        if !tool_call_result.is_malicious {
            // Tool call looks safe, no need for context analysis
            tracing::debug!(
                tool_name = %tool_call.name,
                confidence = tool_call_result.confidence,
                "✅ Tool call passed initial security scan"
            );
            return Ok(tool_call_result);
        }

        // Step 2: Tool call looks suspicious, analyze conversation context
        tracing::info!(
            tool_name = %tool_call.name,
            confidence = tool_call_result.confidence,
            "🔍 Tool call flagged as suspicious, analyzing conversation context"
        );

        let user_messages_result = self.scan_user_messages_only(messages).await?;

        // Decision logic: combine both results
        let final_result =
            self.make_final_security_decision(&tool_call_result, &user_messages_result, tool_call);

        tracing::info!(
            tool_name = %tool_call.name,
            tool_confidence = tool_call_result.confidence,
            user_confidence = user_messages_result.confidence,
            final_malicious = final_result.is_malicious,
            final_confidence = final_result.confidence,
            "🔒 Two-step security analysis complete"
        );

        Ok(final_result)
    }

    /// Step 1: Scan only the tool call for suspicious patterns
    async fn scan_tool_call_only(&self, tool_call: &ToolCall) -> Result<ScanResult> {
        // Debug: Log the raw tool call arguments first
        tracing::info!("🔒 Raw tool call arguments: {:?}", tool_call.arguments);

        // Create text representation of the tool call for analysis
        let arguments_json =
            serde_json::to_string_pretty(&tool_call.arguments).unwrap_or_else(|e| {
                tracing::warn!("🔒 Failed to serialize tool arguments: {}", e);
                format!("{{\"error\": \"Failed to serialize arguments: {}\"}}", e)
            });

        let tool_text = format!("Tool: {}\nArguments: {}", tool_call.name, arguments_json);

        tracing::info!(
            "🔒 Complete tool text being analyzed (length: {}): '{}'",
            tool_text.len(),
            tool_text
        );

        self.scan_with_prompt_injection_model(&tool_text).await
    }

    /// Step 2: Scan only the user messages (conversation history) for prompt injection
    async fn scan_user_messages_only(&self, messages: &[Message]) -> Result<ScanResult> {
        // Extract only user messages from recent conversation history
        let user_messages: Vec<String> = messages
            .iter()
            .rev()
            .take(5) // Take last 5 messages for context
            .rev()
            .filter_map(|msg| {
                // Only analyze user messages, not assistant responses
                if matches!(msg.role, rmcp::model::Role::User) {
                    msg.content.first()?.as_text().map(|text| text.to_string())
                } else {
                    None
                }
            })
            .collect();

        if user_messages.is_empty() {
            return Ok(ScanResult {
                is_malicious: false,
                confidence: 0.0,
                explanation: "No user messages found in conversation history".to_string(),
            });
        }

        let user_context = user_messages.join("\n\n");
        self.scan_with_prompt_injection_model(&user_context).await
    }

    /// Make final security decision based on both tool call and user message analysis
    fn make_final_security_decision(
        &self,
        tool_call_result: &ScanResult,
        user_messages_result: &ScanResult,
        _tool_call: &ToolCall,
    ) -> ScanResult {
        // Simple decision logic using config threshold:
        // 1. If user messages contain prompt injection, tool call is likely malicious
        // 2. Otherwise, use the tool call confidence as-is
        // 3. Let the config threshold determine if user should be asked

        let (confidence, explanation) = if user_messages_result.is_malicious {
            // User messages contain prompt injection - tool call is likely malicious
            let combined_confidence =
                (tool_call_result.confidence + user_messages_result.confidence) / 2.0;
            let explanation = "Tool appears to be the result of a prompt injection attack.".to_string();
            (combined_confidence, explanation)
        } else {
            // Use tool call confidence as-is, let config threshold decide
            let explanation = if tool_call_result.confidence > 0.0 {
                format!("Tool flagged with confidence: {:.2}", tool_call_result.confidence)
            } else {
                "Tool appears safe".to_string()
            };
            (tool_call_result.confidence, explanation)
        };

        // Get threshold from config to determine if malicious
        let config_threshold = self.get_threshold_from_config();
        let is_malicious = confidence > config_threshold;

        ScanResult {
            is_malicious,
            confidence,
            explanation,
        }
    }

    /// Legacy method for backward compatibility - now delegates to two-step analysis
    pub async fn scan_tool_call(&self, tool_call: &ToolCall) -> Result<ScanResult> {
        // For backward compatibility, just scan the tool call without context
        self.scan_tool_call_only(tool_call).await
    }

    /// Legacy method for backward compatibility - now delegates to user message scanning
    pub async fn analyze_conversation_context(
        &self,
        messages: &[Message],
        _tool_call: &ToolCall, // Ignored in new implementation
    ) -> Result<ScanResult> {
        // For backward compatibility, just scan user messages
        self.scan_user_messages_only(messages).await
    }

    /// Model-agnostic prompt injection scanning
    async fn scan_with_prompt_injection_model(&self, text: &str) -> Result<ScanResult> {
        tracing::info!(
            "🔒 Starting scan_with_prompt_injection_model for text (length: {}): '{}'",
            text.len(),
            text.chars().take(100).collect::<String>()
        );

        // Always run pattern-based scanning first
        let pattern_result = self.scan_with_patterns(text).await?;

        // Try to get the ML model for additional scanning
        tracing::info!("🔒 Attempting to get ML model...");
        if let Some(model) = get_model().await {
            tracing::info!("🔒 ML model retrieved successfully, calling predict...");
            tracing::info!(
                "🔒 About to call model.predict() with text length: {}",
                text.len()
            );
            match model.predict(text).await {
                Ok((ml_confidence, ml_explanation)) => {
                    tracing::info!("🔒 ML model predict returned successfully");
                    // Get threshold from config
                    let threshold = self.get_threshold_from_config();
                    let ml_is_malicious = ml_confidence > threshold;

                    tracing::info!(
                        "🔒 ML model prediction: confidence={:.3}, threshold={:.3}, malicious={}",
                        ml_confidence,
                        threshold,
                        ml_is_malicious
                    );

                    // Combine ML and pattern results
                    let combined_result = self.combine_scan_results(
                        &pattern_result,
                        ml_confidence,
                        &ml_explanation,
                        ml_is_malicious,
                    );

                    tracing::info!(
                        "🔒 Combined scan result: ML confidence={:.3}, Pattern confidence={:.3}, Final confidence={:.3}, Final malicious={}",
                        ml_confidence, pattern_result.confidence, combined_result.confidence, combined_result.is_malicious
                    );

                    return Ok(combined_result);
                }
                Err(e) => {
                    tracing::warn!("🔒 ML model prediction failed: {}", e);
                    // Fall through to pattern-only result
                }
            }
        } else {
            tracing::info!("🔒 No ML model available, using pattern-based scanning only");
        }

        tracing::info!("🔒 Using pattern-based scan result only");
        Ok(pattern_result)
    }

    /// Combine ML model and pattern matching results
    fn combine_scan_results(
        &self,
        pattern_result: &ScanResult,
        ml_confidence: f32,
        _ml_explanation: &str,
        ml_is_malicious: bool,
    ) -> ScanResult {
        // Take the higher confidence score
        let final_confidence = pattern_result.confidence.max(ml_confidence);

        // Mark as malicious if either method detects it
        let final_is_malicious = pattern_result.is_malicious || ml_is_malicious;

        // Simplified explanation - just show what detected the threat
        let combined_explanation = if pattern_result.is_malicious && ml_is_malicious {
            "Detected by both pattern analysis and ML model".to_string()
        } else if pattern_result.is_malicious {
            format!(
                "Detected by pattern analysis: {}",
                pattern_result
                    .explanation
                    .replace("Pattern-based detection: ", "")
            )
        } else if ml_is_malicious {
            "Detected by machine learning model".to_string()
        } else {
            "No threats detected".to_string()
        };

        ScanResult {
            is_malicious: final_is_malicious,
            confidence: final_confidence,
            explanation: combined_explanation,
        }
    }

    /// Get threshold from config
    pub fn get_threshold_from_config(&self) -> f32 {
        use crate::config::Config;
        let config = Config::global();

        // Get security config and extract threshold
        if let Ok(security_value) = config.get_param::<serde_json::Value>("security") {
            if let Some(models_array) = security_value.get("models").and_then(|m| m.as_array()) {
                if let Some(first_model) = models_array.first() {
                    if let Some(threshold) = first_model.get("threshold").and_then(|t| t.as_f64()) {
                        return threshold as f32;
                    }
                }
            }
        }

        0.7 // Default threshold
    }

    /// Fallback pattern-based scanning
    async fn scan_with_patterns(&self, text: &str) -> Result<ScanResult> {
        let text_lower = text.to_lowercase();

        // Command injection patterns - detect potentially dangerous commands
        let dangerous_patterns = [
            // File system operations
            "rm -rf /",
            "rm -rf /*",
            "rm -rf ~",
            "rm -rf $home",
            "rmdir /",
            "del /s /q",
            "format c:",
            // System manipulation
            "shutdown",
            "reboot",
            "halt",
            "poweroff",
            "kill -9",
            "killall",
            // Network/data exfiltration
            "curl http",
            "wget http",
            "nc -l",
            "netcat",
            "ssh ",
            "scp ",
            "rsync",
            // Process manipulation
            "sudo ",
            "su -",
            "chmod 777",
            "chown root",
            // Command chaining that could hide malicious intent
            "; rm ",
            "&& rm ",
            "| rm ",
            "; curl ",
            "&& curl ",
            "| curl ",
            "; wget ",
            "&& wget ",
            // Suspicious file operations
            "rm -f /",
            "rm -rf .",
            "rm -rf ..",
            "> /dev/",
            "dd if=",
            "mkfs",
            // Potential data theft
            "cat /etc/passwd",
            "cat /etc/shadow",
            "/etc/hosts",
            "~/.ssh/",
            "id_rsa",
            // Obfuscation attempts
            "base64 -d",
            "echo | sh",
            "eval ",
            "exec ",
        ];

        let mut detected_patterns = Vec::new();
        let mut max_risk_score: f32 = 0.0;

        for pattern in &dangerous_patterns {
            if text_lower.contains(pattern) {
                detected_patterns.push(pattern.to_string());

                // Assign risk scores based on severity
                let risk_score = match *pattern {
                    // Critical - system destruction
                    "rm -rf /" | "rm -rf /*" | "format c:" | "mkfs" => 0.95,
                    "rm -rf ~" | "rm -rf $home" => 0.90,

                    // High - system control
                    "shutdown" | "reboot" | "halt" | "poweroff" => 0.85,
                    "sudo " | "su -" | "chmod 777" | "chown root" => 0.80,

                    // Medium-High - network/data access
                    "curl http" | "wget http" | "ssh " | "scp " => 0.75,
                    "cat /etc/passwd" | "cat /etc/shadow" | "~/.ssh/" => 0.85,

                    // Medium - suspicious operations
                    "; rm " | "&& rm " | "| rm " => 0.70,
                    "kill -9" | "killall" => 0.65,

                    // Lower - potentially legitimate but suspicious
                    "base64 -d" | "eval " | "exec " => 0.60,

                    _ => 0.50,
                };

                max_risk_score = max_risk_score.max(risk_score);
            }
        }

        if !detected_patterns.is_empty() {
            // Use config threshold for pattern-based detection too
            let config_threshold = self.get_threshold_from_config();
            let is_malicious = max_risk_score > config_threshold;
            let explanation = format!(
                "Pattern-based detection: Found {} suspicious command pattern(s): [{}]. Risk score: {:.2}",
                detected_patterns.len(),
                detected_patterns.join(", "),
                max_risk_score
            );

            tracing::info!(
                "🔒 Pattern-based scan detected {} suspicious patterns with max risk score: {:.2}",
                detected_patterns.len(),
                max_risk_score
            );

            Ok(ScanResult {
                is_malicious,
                confidence: max_risk_score,
                explanation,
            })
        } else {
            Ok(ScanResult {
                is_malicious: false,
                confidence: 0.0,
                explanation: "Pattern-based scan: No suspicious command patterns detected"
                    .to_string(),
            })
        }
    }

    /// Assess inherent risk of specific tools
    fn assess_tool_risk(&self, tool_name: &str) -> f32 {
        // Higher risk tools that could be used maliciously
        match tool_name {
            name if name.contains("shell") || name.contains("exec") => 0.8,
            name if name.contains("file") && name.contains("write") => 0.6,
            name if name.contains("network") || name.contains("http") => 0.5,
            name if name.contains("read") => 0.3,
            _ => 0.1,
        }
    }
}

impl Default for PromptInjectionScanner {
    fn default() -> Self {
        Self::new()
    }
}
