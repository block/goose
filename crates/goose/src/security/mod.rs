pub mod classification_client;
pub mod patterns;
pub mod scanner;
pub mod security_inspector;

use crate::config::Config;
use crate::conversation::message::{Message, ToolRequest};
use crate::permission::permission_judge::PermissionCheckResult;
use anyhow::Result;
use scanner::PromptInjectionScanner;
use std::sync::OnceLock;
use uuid::Uuid;

pub struct SecurityManager {
    scanner: OnceLock<PromptInjectionScanner>,
}

#[derive(Debug, Clone)]
pub struct SecurityResult {
    pub is_malicious: bool,
    pub confidence: f32,
    pub explanation: String,
    pub should_ask_user: bool,
    pub finding_id: String,
    pub tool_request_id: String,
    pub detection_type: Option<String>,
    pub command_confidence: Option<f32>,
    pub prompt_confidence: Option<f32>,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            scanner: OnceLock::new(),
        }
    }

    pub fn is_prompt_injection_detection_enabled(&self) -> bool {
        let config = Config::global();

        config
            .get_param::<bool>("SECURITY_PROMPT_ENABLED")
            .unwrap_or(false)
    }

    fn is_ml_scanning_enabled(&self) -> bool {
        let config = Config::global();

        config
            .get_param::<bool>("SECURITY_PROMPT_CLASSIFIER_ENABLED")
            .unwrap_or(false)
    }

    pub async fn analyze_tool_requests(
        &self,
        tool_requests: &[ToolRequest],
        messages: &[Message],
    ) -> Result<Vec<SecurityResult>> {
        if !self.is_prompt_injection_detection_enabled() {
            tracing::debug!(
                counter.goose.prompt_injection_scanner_disabled = 1,
                "Security scanning disabled"
            );
            return Ok(vec![]);
        }

        let scanner = self.scanner.get_or_init(|| {
            let ml_enabled = self.is_ml_scanning_enabled();

            let scanner = if ml_enabled {
                match PromptInjectionScanner::with_ml_detection() {
                    Ok(s) => {
                        tracing::info!(
                            counter.goose.prompt_injection_scanner_enabled = 1,
                            "🔓 Security scanner initialized with ML-based detection"
                        );
                        s
                    }
                    Err(e) => {
                        let error_chain = format!("{:#}", e);
                        tracing::warn!(
                            "⚠️ ML scanning requested but failed to initialize. Falling back to pattern-only scanning.\n\nError details:\n{}",
                            error_chain
                        );
                        PromptInjectionScanner::new()
                    }
                }
            } else {
                tracing::info!(
                    counter.goose.prompt_injection_scanner_enabled = 1,
                    "🔓 Security scanner initialized with pattern-based detection only"
                );
                PromptInjectionScanner::new()
            };

            scanner
        });

        let mut results = Vec::new();

        tracing::info!(
            "🔍 Starting security analysis - {} tool requests, {} messages",
            tool_requests.len(),
            messages.len()
        );

        for tool_request in tool_requests.iter() {
            if let Ok(tool_call) = &tool_request.tool_call {
                let analysis_result = scanner
                    .analyze_tool_call_with_context(tool_call, messages)
                    .await?;

                let config_threshold = scanner.get_threshold_from_config();
                let sanitized_explanation = analysis_result.explanation.replace('\n', " | ");

                if analysis_result.is_malicious {
                    let above_threshold = analysis_result.confidence > config_threshold;
                    let finding_id = format!("SEC-{}", Uuid::new_v4().simple());

                    tracing::warn!(
                        counter.goose.prompt_injection_finding = 1,
                        above_threshold = above_threshold,
                        tool_name = %tool_call.name,
                        tool_request_id = %tool_request.id,
                        confidence = analysis_result.confidence,
                        explanation = %sanitized_explanation,
                        finding_id = %finding_id,
                        threshold = config_threshold,
                        "{}",
                        if above_threshold {
                            "Current tool call flagged as malicious after security analysis (above threshold)"
                        } else {
                            "Security finding below threshold - logged but not blocking execution"
                        }
                    );
                    if above_threshold {
                        results.push(SecurityResult {
                            is_malicious: analysis_result.is_malicious,
                            confidence: analysis_result.confidence,
                            explanation: analysis_result.explanation,
                            should_ask_user: true, // Always ask user for threats above threshold
                            finding_id,
                            tool_request_id: tool_request.id.clone(),
                            detection_type: analysis_result
                                .detection_type
                                .map(|dt| format!("{:?}", dt)),
                            command_confidence: analysis_result.command_confidence,
                            prompt_confidence: analysis_result.prompt_confidence,
                        });
                    }
                } else {
                    tracing::info!(
                        tool_name = %tool_call.name,
                        tool_request_id = %tool_request.id,
                        confidence = analysis_result.confidence,
                        explanation = %sanitized_explanation,
                        "✅ Current tool call passed security analysis"
                    );
                }
            }
        }

        tracing::info!(
            counter.goose.prompt_injection_analysis_performed = 1,
            security_issues_found = results.len(),
            "Security analysis complete"
        );
        Ok(results)
    }

    pub async fn filter_malicious_tool_calls(
        &self,
        messages: &[Message],
        permission_check_result: &PermissionCheckResult,
        _system_prompt: Option<&str>,
    ) -> Result<Vec<SecurityResult>> {
        let tool_requests: Vec<_> = permission_check_result
            .approved
            .iter()
            .chain(permission_check_result.needs_approval.iter())
            .cloned()
            .collect();

        self.analyze_tool_requests(&tool_requests, messages).await
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}
