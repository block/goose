pub mod patterns;
pub mod scanner;

#[cfg(test)]
mod integration_tests;

use crate::conversation::message::Message;
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
        println!("🔒 SecurityManager::new() called - checking if security should be enabled");

        // Initialize scanner based on config
        let should_enable = Self::should_enable_security();
        println!("🔒 Security enabled check result: {}", should_enable);

        let scanner = match should_enable {
            true => {
                println!("🔒 Initializing security scanner");
                tracing::info!("🔒 Initializing security scanner");
                Some(PromptInjectionScanner::new())
            }
            false => {
                println!("🔓 Security scanning disabled");
                tracing::info!("🔓 Security scanning disabled");
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

        println!(
            "🔒 Config check - security config result: {:?}",
            config.get_param::<serde_json::Value>("security")
        );
        println!("🔒 Final security enabled result: {}", result);

        result
    }

    /// Main security check function - called from reply_internal
    /// Uses the proper two-step security analysis process
    /// Scans ALL tools (approved + needs_approval) for security threats
    /// Also scans user messages for dangerous content
    pub async fn filter_malicious_tool_calls(
        &self,
        messages: &[Message],
        permission_check_result: &PermissionCheckResult,
        _system_prompt: Option<&str>,
    ) -> Result<Vec<SecurityResult>> {
        let Some(scanner) = &self.scanner else {
            // Security disabled, return empty results
            tracing::debug!("🔓 Security scanning disabled - returning empty results");
            return Ok(vec![]);
        };

        let mut results = Vec::new();

        tracing::info!("🔍 Starting security analysis - {} approved tools, {} needs approval tools, {} messages", 
            permission_check_result.approved.len(),
            permission_check_result.needs_approval.len(),
            messages.len()
        );

        // Scan recent user messages for dangerous content
        // Look at the last few messages to catch malicious user input
        let recent_messages = messages.iter().rev().take(3).collect::<Vec<_>>();
        for (i, message) in recent_messages.iter().enumerate() {
            if message.role == rmcp::model::Role::User {
                let message_text = message.as_concat_text();
                if !message_text.trim().is_empty() {
                    tracing::info!("🔍 Scanning user message {} for dangerous content: '{}'", 
                        i, 
                        message_text.chars().take(100).collect::<String>()
                    );
                    
                    let message_result = scanner.scan_for_dangerous_patterns(&message_text).await?;
                    
                    if message_result.is_malicious {
                        let finding_id = format!("MSG-{}", &uuid::Uuid::new_v4().simple().to_string().to_uppercase()[..8]);
                        
                        tracing::warn!(
                            confidence = message_result.confidence,
                            explanation = %message_result.explanation,
                            finding_id = %finding_id,
                            message_preview = %message_text.chars().take(50).collect::<String>(),
                            "🔒 User message contains dangerous content"
                        );

                        let config_threshold = scanner.get_threshold_from_config();
                        
                        results.push(SecurityResult {
                            is_malicious: message_result.is_malicious,
                            confidence: message_result.confidence,
                            explanation: format!("Dangerous user input detected: {}", message_result.explanation),
                            should_ask_user: message_result.confidence > config_threshold,
                            finding_id,
                        });
                    } else {
                        tracing::debug!("✅ User message passed security analysis");
                    }
                }
            }
        }

        // Check ALL tools (approved + needs_approval) for potential security issues
        for (i, tool_request) in permission_check_result
            .approved
            .iter()
            .chain(permission_check_result.needs_approval.iter())
            .enumerate()
        {
            if let Ok(tool_call) = &tool_request.tool_call {
                tracing::info!(
                    tool_name = %tool_call.name,
                    tool_index = i,
                    tool_args = ?tool_call.arguments,
                    "🔍 Starting security analysis for tool call"
                );

                // Use the new two-step analysis method
                let analysis_result = scanner
                    .analyze_tool_call_with_context(tool_call, messages)
                    .await?;

                if analysis_result.is_malicious {
                    // Generate a unique finding ID for this security detection
                    let finding_id = format!("SEC-{}", &uuid::Uuid::new_v4().simple().to_string().to_uppercase()[..8]);
                    
                    tracing::warn!(
                        tool_name = %tool_call.name,
                        confidence = analysis_result.confidence,
                        explanation = %analysis_result.explanation,
                        finding_id = %finding_id,
                        "🔒 Tool call flagged as malicious after security analysis"
                    );

                    // Get threshold from config - if confidence > threshold, ask user
                    let config_threshold = scanner.get_threshold_from_config();
                    
                    results.push(SecurityResult {
                        is_malicious: analysis_result.is_malicious,
                        confidence: analysis_result.confidence,
                        explanation: analysis_result.explanation,
                        should_ask_user: analysis_result.confidence > config_threshold,
                        finding_id,
                    });
                } else {
                    tracing::debug!(
                        tool_name = %tool_call.name,
                        confidence = analysis_result.confidence,
                        explanation = %analysis_result.explanation,
                        "✅ Tool call passed security analysis"
                    );
                }
            }
        }

        tracing::info!("🔍 Security analysis complete - found {} security issues", results.len());
        Ok(results)
    }

    /// Check if models need to be downloaded and return appropriate user message
    pub async fn check_model_download_status(&self) -> Option<String> {
        // Phase 1: No ML models needed, pattern matching is instant
        None
    }

    /// Scan recipe components for security threats
    /// This should be called when loading/applying recipes
    pub async fn scan_recipe_components(&self, recipe: &crate::recipe::Recipe) -> Result<Vec<SecurityResult>> {
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
                    let finding_id = format!("RCP-{}", &uuid::Uuid::new_v4().simple().to_string().to_uppercase()[..8]);
                    
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
                        explanation: format!("Recipe prompt injection: {}", prompt_result.explanation),
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
                    tracing::info!("🔍 Scanning recipe context item {} for injection attacks", i);
                    
                    let context_result = scanner.scan_with_prompt_injection_model(context_item).await?;
                    
                    if context_result.is_malicious {
                        let finding_id = format!("RCC-{}", &uuid::Uuid::new_v4().simple().to_string().to_uppercase()[..8]);
                        
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
                            explanation: format!("Recipe context[{}] injection: {}", i, context_result.explanation),
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
