pub mod patterns;
pub mod scanner;
pub mod inspector;

use crate::conversation::message::{Message, ToolRequest};
use crate::permission::permission_judge::PermissionCheckResult;
use anyhow::Result;
use scanner::PromptInjectionScanner;

/// Simple security manager for the POC
/// Focuses on tool call analysis with conversation context
pub struct SecurityManager {
    scanner: Option<PromptInjectionScanner>,
}

#[derive(Debug, Clone)]
pub struct SecurityResult {
    pub is_malicious: bool,
    pub confidence: f32,
    pub explanation: String,
    pub should_ask_user: bool,
    pub finding_id: String,
}

impl SecurityManager {
    pub fn new() -> Self {
        // Initialize scanner based on config
        let should_enable = Self::should_enable_security();

        let scanner = match should_enable {
            true => {
                tracing::info!("Security scanner initialized and enabled");
                Some(PromptInjectionScanner::new())
            }
            false => {
                tracing::debug!("Security scanning disabled via configuration");
                None
            }
        };

        Self { scanner }
    }

    /// Check if security should be enabled based on config
    fn should_enable_security() -> bool {
        // Check config file for security settings
        use crate::config::Config;
        let config = Config::global();

        // Try to get security.enabled from config
        let result = config
            .get_param::<serde_json::Value>("security")
            .ok()
            .and_then(|security_config| security_config.get("enabled")?.as_bool())
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
        &self,
        tool_requests: &[ToolRequest],
        messages: &[Message],
    ) -> Result<Vec<SecurityResult>> {
        let Some(scanner) = &self.scanner else {
            // Security disabled, return empty results
            tracing::debug!("🔓 Security scanning disabled - returning empty results");
            return Ok(vec![]);
        };

        let mut results = Vec::new();

        tracing::info!("🔍 Starting security analysis - {} tool requests, {} messages", 
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
                    .analyze_tool_call_with_context(tool_call, &[])  // Pass empty messages to avoid historical analysis
                    .await?;

                // Get threshold from config - only flag things above threshold
                let config_threshold = scanner.get_threshold_from_config();
                
                if analysis_result.is_malicious && analysis_result.confidence > config_threshold {
                    // Generate a deterministic finding ID based on tool request ID to avoid duplicates
                    let finding_id = format!("SEC-{}", tool_request.id);

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
        &self,
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

    /// Scan recipe components for security threats
    /// This should be called when loading/applying recipes
    pub async fn scan_recipe_components(
        &self,
        recipe: &crate::recipe::Recipe,
    ) -> Result<Vec<SecurityResult>> {
        let Some(scanner) = &self.scanner else {
            // Security disabled, return empty results
            return Ok(vec![]);
        };

        let mut results = Vec::new();

        // Scan recipe prompt (becomes initial user message)
        if let Some(prompt) = &recipe.prompt {
            if !prompt.trim().is_empty() {
                tracing::info!("🔍 Scanning recipe prompt for injection attacks");

                let prompt_result = scanner.scan_with_prompt_injection_model(prompt).await?;

                if prompt_result.is_malicious {
                    let finding_id = format!(
                        "RCP-{}",
                        &uuid::Uuid::new_v4().simple().to_string().to_uppercase()[..8]
                    );

                    tracing::warn!(
                        confidence = prompt_result.confidence,
                        explanation = %prompt_result.explanation,
                        finding_id = %finding_id,
                        "🔒 Recipe prompt contains malicious content"
                    );

                    let config_threshold = scanner.get_threshold_from_config();

                    results.push(SecurityResult {
                        is_malicious: prompt_result.is_malicious,
                        confidence: prompt_result.confidence,
                        explanation: format!(
                            "Recipe prompt injection: {}",
                            prompt_result.explanation
                        ),
                        should_ask_user: prompt_result.confidence > config_threshold,
                        finding_id,
                    });
                }
            }
        }

        // Scan recipe context (additional context data)
        if let Some(context_items) = &recipe.context {
            for (i, context_item) in context_items.iter().enumerate() {
                if !context_item.trim().is_empty() {
                    tracing::info!(
                        "🔍 Scanning recipe context item {} for injection attacks",
                        i
                    );

                    let context_result = scanner
                        .scan_with_prompt_injection_model(context_item)
                        .await?;

                    if context_result.is_malicious {
                        let finding_id = format!(
                            "RCC-{}",
                            &uuid::Uuid::new_v4().simple().to_string().to_uppercase()[..8]
                        );

                        tracing::warn!(
                            context_index = i,
                            confidence = context_result.confidence,
                            explanation = %context_result.explanation,
                            finding_id = %finding_id,
                            "🔒 Recipe context contains malicious content"
                        );

                        let config_threshold = scanner.get_threshold_from_config();

                        results.push(SecurityResult {
                            is_malicious: context_result.is_malicious,
                            confidence: context_result.confidence,
                            explanation: format!(
                                "Recipe context[{}] injection: {}",
                                i, context_result.explanation
                            ),
                            should_ask_user: context_result.confidence > config_threshold,
                            finding_id,
                        });
                    }
                }
            }
        }

        Ok(results)
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}
