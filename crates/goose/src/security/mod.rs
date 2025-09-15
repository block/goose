pub mod patterns;
pub mod scanner;
pub mod security_inspector;

use crate::conversation::message::{Message, ToolRequest};
use crate::permission::permission_judge::PermissionCheckResult;
use anyhow::Result;
use scanner::PromptInjectionScanner;
use std::collections::{hash_map::DefaultHasher, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

/// Simple security manager for the POC
/// Focuses on tool call analysis with conversation context
pub struct SecurityManager {
    scanner: Option<PromptInjectionScanner>,
    flagged_findings: Arc<Mutex<HashSet<String>>>,
}

#[derive(Debug, Clone)]
pub struct SecurityResult {
    pub is_malicious: bool,
    pub confidence: f32,
    pub explanation: String,
    pub should_ask_user: bool,
    pub finding_id: String,
    pub tool_request_id: String,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            scanner: None, // Initialize lazily
            flagged_findings: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Check if security is enabled - reads fresh from config each time
    pub fn is_enabled(&self) -> bool {
        Self::should_enable_security()
    }



    /// Check if security should be enabled based on config
    fn should_enable_security() -> bool {
        // Check config file for security settings
        use crate::config::Config;
        let config = Config::global();

        // Try to read the nested security config first
        if let Ok(security_config) = config.get_param::<serde_json::Value>("security") {
            if let Some(enabled) = security_config.get("enabled").and_then(|v| v.as_bool()) {
                return enabled;
            }
        }

        // Fall back to dot notation (security.enabled)
        let result = config
            .get_param::<bool>("security.enabled")
            .unwrap_or(false);

        tracing::debug!(
            security_config = ?config.get_param::<serde_json::Value>("security"),
            enabled = result,
            "Security configuration check completed"
        );

        result
    }

    /// New method for tool inspection framework - works directly with tool requests
    pub async fn analyze_tool_requests(
        &mut self,
        tool_requests: &[ToolRequest],
        messages: &[Message],
    ) -> Result<Vec<SecurityResult>> {
        // Check if security is enabled and get scanner if needed
        if !self.is_enabled() {
            tracing::debug!("🔓 Security scanning disabled - returning empty results");
            return Ok(vec![]);
        }

        // Initialize scanner if needed
        if self.scanner.is_none() {
            tracing::info!("Security scanner initialized and enabled");
            self.scanner = Some(PromptInjectionScanner::new());
        }

        let Some(scanner) = &self.scanner else {
            return Ok(vec![]);
        };

        let mut results = Vec::new();

        tracing::info!(
            "🔍 Starting security analysis - {} tool requests, {} messages",
            tool_requests.len(),
            messages.len()
        );

        // Only analyze CURRENT tool requests, not historical ones from conversation
        // This prevents re-flagging the same malicious content from previous messages
        for (i, tool_request) in tool_requests.iter().enumerate() {
            if let Ok(tool_call) = &tool_request.tool_call {
                tracing::info!(
                    tool_name = %tool_call.name,
                    tool_index = i,
                    tool_request_id = %tool_request.id,
                    tool_args = ?tool_call.arguments,
                    "🔍 Starting security analysis for current tool call"
                );

                // Analyze only the current tool call content, not the entire conversation history
                // This prevents re-analyzing and re-flagging historical malicious content
                let analysis_result = scanner
                    .analyze_tool_call_with_context(tool_call, &[]) // Pass empty messages to avoid historical analysis
                    .await?;

                // Get threshold from config - only flag things above threshold
                let config_threshold = scanner.get_threshold_from_config();

                if analysis_result.is_malicious && analysis_result.confidence > config_threshold {
                    // Generate a unique finding ID based on normalized tool call content
                    // This ensures the same malicious content always gets the same finding ID
                    // regardless of JSON formatting or tool request ID variations
                    let normalized_content = format!(
                        "{}:{}",
                        tool_call.name,
                        serde_json::to_string(&tool_call.arguments).unwrap_or_default()
                    );
                    let mut hasher = DefaultHasher::new();
                    normalized_content.hash(&mut hasher);
                    let content_hash = hasher.finish();
                    let finding_id = format!("SEC-{:016x}", content_hash);

                    // Check if we've already flagged this exact finding before
                    let mut flagged_set = self.flagged_findings.lock().unwrap();
                    if flagged_set.contains(&finding_id) {
                        tracing::debug!(
                            tool_name = %tool_call.name,
                            tool_request_id = %tool_request.id,
                            finding_id = %finding_id,
                            "🔄 Skipping already flagged security finding - preventing re-flagging"
                        );
                        continue;
                    }

                    // Mark this finding as flagged
                    flagged_set.insert(finding_id.clone());
                    drop(flagged_set); // Release the lock

                    tracing::warn!(
                        tool_name = %tool_call.name,
                        tool_request_id = %tool_request.id,
                        confidence = analysis_result.confidence,
                        explanation = %analysis_result.explanation,
                        finding_id = %finding_id,
                        threshold = config_threshold,
                        "🔒 Current tool call flagged as malicious after security analysis (above threshold)"
                    );

                    results.push(SecurityResult {
                        is_malicious: analysis_result.is_malicious,
                        confidence: analysis_result.confidence,
                        explanation: analysis_result.explanation,
                        should_ask_user: true, // Always ask user for threats above threshold
                        finding_id,
                        tool_request_id: tool_request.id.clone(),
                    });
                } else if analysis_result.is_malicious {
                    tracing::warn!(
                        tool_name = %tool_call.name,
                        tool_request_id = %tool_request.id,
                        confidence = analysis_result.confidence,
                        explanation = %analysis_result.explanation,
                        threshold = config_threshold,
                        "🔒 Security finding below threshold - logged but not blocking execution"
                    );
                } else {
                    tracing::debug!(
                        tool_name = %tool_call.name,
                        tool_request_id = %tool_request.id,
                        confidence = analysis_result.confidence,
                        explanation = %analysis_result.explanation,
                        "✅ Current tool call passed security analysis"
                    );
                }
            }
        }

        tracing::info!(
            "🔍 Security analysis complete - found {} security issues in current tool requests",
            results.len()
        );
        Ok(results)
    }

    /// Main security check function - called from reply_internal
    /// Uses the proper two-step security analysis process
    /// Scans ALL tools (approved + needs_approval) for security threats
    pub async fn filter_malicious_tool_calls(
        &mut self,
        messages: &[Message],
        permission_check_result: &PermissionCheckResult,
        _system_prompt: Option<&str>,
    ) -> Result<Vec<SecurityResult>> {
        // Extract tool requests from permission result and delegate to new method
        let tool_requests: Vec<_> = permission_check_result
            .approved
            .iter()
            .chain(permission_check_result.needs_approval.iter())
            .cloned()
            .collect();

        self.analyze_tool_requests(&tool_requests, messages).await
    }

    /// Check if models need to be downloaded and return appropriate user message
    pub async fn check_model_download_status(&self) -> Option<String> {
        // Phase 1: No ML models needed, pattern matching is instant
        None
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}
