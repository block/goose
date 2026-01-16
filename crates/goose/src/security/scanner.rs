use crate::config::Config;
use crate::conversation::message::Message;
use crate::security::classification_client::ClassificationClient;
use crate::security::patterns::{PatternMatch, PatternMatcher};
use anyhow::Result;
use futures::stream::{self, StreamExt};
use rmcp::model::CallToolRequestParam;

const USER_SCAN_LIMIT: usize = 10;
const ML_SCAN_CONCURRENCY: usize = 3;

#[derive(Debug, Clone, Copy)]
enum ClassifierType {
    Command,
    Prompt,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub is_malicious: bool,
    pub confidence: f32,
    pub explanation: String,
    pub detection_type: Option<DetectionType>,
    pub command_confidence: Option<f32>,
    pub prompt_confidence: Option<f32>,
}

#[derive(Debug, Clone, Copy)]
pub enum DetectionType {
    CommandInjection,
    PromptInjection,
    PatternMatch,
}

#[derive(Clone)]
struct DetailedScanResult {
    confidence: f32,
    pattern_matches: Vec<PatternMatch>,
    ml_confidence: Option<f32>,
}

pub struct PromptInjectionScanner {
    pattern_matcher: PatternMatcher,
    command_classifier: Option<ClassificationClient>,
    prompt_classifier: Option<ClassificationClient>,
}

impl PromptInjectionScanner {
    pub fn new() -> Self {
        Self {
            pattern_matcher: PatternMatcher::new(),
            command_classifier: None,
            prompt_classifier: None,
        }
    }

    pub fn with_ml_detection() -> Result<Self> {
        let command_classifier = Self::create_classifier(ClassifierType::Command).ok();
        let prompt_classifier = Self::create_classifier(ClassifierType::Prompt).ok();

        if command_classifier.is_none() && prompt_classifier.is_none() {
            anyhow::bail!("ML detection enabled but no classifiers could be initialized");
        }

        Ok(Self {
            pattern_matcher: PatternMatcher::new(),
            command_classifier,
            prompt_classifier,
        })
    }

    fn create_classifier(classifier_type: ClassifierType) -> Result<ClassificationClient> {
        let config = Config::global();
        let prefix = match classifier_type {
            ClassifierType::Command => "COMMAND",
            ClassifierType::Prompt => "PROMPT",
        };

        let enabled = config
            .get_param::<bool>(&format!("SECURITY_{}_CLASSIFIER_ENABLED", prefix))
            .unwrap_or(false);

        if !enabled {
            anyhow::bail!("{} classifier not enabled", prefix);
        }

        let model_name = config
            .get_param::<String>(&format!("SECURITY_{}_CLASSIFIER_MODEL", prefix))
            .ok()
            .filter(|s| !s.trim().is_empty());
        let endpoint = config
            .get_param::<String>(&format!("SECURITY_{}_CLASSIFIER_ENDPOINT", prefix))
            .ok()
            .filter(|s| !s.trim().is_empty());
        let token = config
            .get_secret::<String>(&format!("SECURITY_{}_CLASSIFIER_TOKEN", prefix))
            .ok()
            .filter(|s| !s.trim().is_empty());

        if let Some(model) = model_name {
            return ClassificationClient::from_model_name(&model, None);
        }

        if let Some(endpoint_url) = endpoint {
            return ClassificationClient::from_endpoint(endpoint_url, None, token);
        }

        anyhow::bail!(
            "{} classifier requires either SECURITY_{}_CLASSIFIER_MODEL or SECURITY_{}_CLASSIFIER_ENDPOINT",
            prefix,
            prefix,
            prefix
        )
    }

    pub fn get_threshold_from_config(&self) -> f32 {
        Config::global()
            .get_param::<f64>("SECURITY_PROMPT_THRESHOLD")
            .unwrap_or(0.8) as f32
    }

    pub async fn analyze_tool_call_with_context(
        &self,
        tool_call: &CallToolRequestParam,
        messages: &[Message],
    ) -> Result<ScanResult> {
        let tool_content = self.extract_tool_content(tool_call);

        tracing::info!(
            "🔍 Scanning tool call: {} ({} chars)",
            tool_call.name,
            tool_content.len()
        );

        let (tool_result, context_result) = tokio::join!(
            self.analyze_text(&tool_content),
            self.scan_conversation(messages)
        );

        let tool_result = tool_result?;
        let context_result = context_result?;
        let threshold = self.get_threshold_from_config();

        tracing::info!(
            "📊 Classifier Results - Command: {:.3}, Prompt: {:.3}, Threshold: {:.3}",
            tool_result.confidence,
            context_result.confidence,
            threshold
        );

        let final_result =
            self.select_result_with_context_awareness(tool_result.clone(), context_result.clone(), threshold);

        tracing::info!(
            "Security analysis complete: final_confidence={:.3}, malicious={}",
            final_result.confidence,
            final_result.confidence >= threshold
        );

        // Determine detection type based on which classifier triggered
        let detection_type = if final_result.confidence >= threshold {
            if !final_result.pattern_matches.is_empty() {
                Some(DetectionType::PatternMatch)
            } else if tool_result.confidence >= context_result.confidence {
                Some(DetectionType::CommandInjection)
            } else {
                Some(DetectionType::PromptInjection)
            }
        } else {
            None
        };

        Ok(ScanResult {
            is_malicious: final_result.confidence >= threshold,
            confidence: final_result.confidence,
            explanation: self.build_explanation(&final_result, threshold, &tool_content),
            detection_type,
            command_confidence: tool_result.ml_confidence,
            prompt_confidence: context_result.ml_confidence,
        })
    }

    async fn analyze_text(&self, text: &str) -> Result<DetailedScanResult> {
        let text_preview = if text.len() > 80 {
            format!("{}...", &text[..80])
        } else {
            text.to_string()
        };

        if let Some(ml_confidence) = self.scan_command_with_classifier(text).await {
            tracing::info!(
                "🔍 [Command] conf={:.3} | {}",
                ml_confidence,
                text_preview
            );
            return Ok(DetailedScanResult {
                confidence: ml_confidence,
                pattern_matches: Vec::new(),
                ml_confidence: Some(ml_confidence),
            });
        }

        let (pattern_confidence, pattern_matches) = self.pattern_based_scanning(text);
        tracing::info!(
            "🔍 [Pattern] conf={:.3}, matches={} | {}",
            pattern_confidence,
            pattern_matches.len(),
            text_preview
        );
        Ok(DetailedScanResult {
            confidence: pattern_confidence,
            pattern_matches,
            ml_confidence: None,
        })
    }

    async fn scan_conversation(&self, messages: &[Message]) -> Result<DetailedScanResult> {
        let user_messages = self.extract_user_messages(messages, USER_SCAN_LIMIT);

        if user_messages.is_empty() || self.prompt_classifier.is_none() {
            return Ok(DetailedScanResult {
                confidence: 0.0,
                pattern_matches: Vec::new(),
                ml_confidence: None,
            });
        }

        // Create message-preview pairs for concurrent scanning
        let message_pairs: Vec<(String, String)> = user_messages
            .into_iter()
            .map(|msg| {
                let preview = if msg.len() > 60 {
                    format!("{}...", &msg[..60])
                } else {
                    msg.clone()
                };
                (msg, preview)
            })
            .collect();

        let max_confidence = stream::iter(message_pairs)
            .map(|(msg, preview)| async move {
                let result = self.scan_prompt_with_classifier(&msg).await;
                if let Some(conf) = result {
                    tracing::info!("🔍 [Prompt] conf={:.3} | {}", conf, preview);
                }
                result
            })
            .buffer_unordered(ML_SCAN_CONCURRENCY)
            .fold(0.0_f32, |acc, result| async move {
                result.unwrap_or(0.0).max(acc)
            })
            .await;

        Ok(DetailedScanResult {
            confidence: max_confidence,
            pattern_matches: Vec::new(),
            ml_confidence: Some(max_confidence),
        })
    }

    fn select_result_with_context_awareness(
        // TODO: this may need some finetuning, based on how testing goes
        &self,
        tool_result: DetailedScanResult,
        context_result: DetailedScanResult,
        threshold: f32,
    ) -> DetailedScanResult {
        let context_is_safe = context_result
            .ml_confidence
            .is_some_and(|conf| conf < threshold);

        let tool_has_only_non_critical = !tool_result.pattern_matches.is_empty()
            && tool_result
                .pattern_matches
                .iter()
                .all(|m| m.threat.risk_level != crate::security::patterns::RiskLevel::Critical);

        if context_is_safe && tool_has_only_non_critical {
            DetailedScanResult {
                confidence: 0.0,
                pattern_matches: Vec::new(),
                ml_confidence: context_result.ml_confidence,
            }
        } else if tool_result.confidence >= context_result.confidence {
            tool_result
        } else {
            context_result
        }
    }

    async fn scan_with_classifier(
        &self,
        text: &str,
        classifier: &ClassificationClient,
        classifier_type: ClassifierType,
    ) -> Option<f32> {
        let type_name = match classifier_type {
            ClassifierType::Command => "command injection",
            ClassifierType::Prompt => "prompt injection",
        };

        match classifier.classify(text).await {
            Ok(conf) => Some(conf),
            Err(e) => {
                tracing::warn!("{} classifier scan failed: {:#}", type_name, e);
                None
            }
        }
    }

    async fn scan_command_with_classifier(&self, text: &str) -> Option<f32> {
        let classifier = self.command_classifier.as_ref()?;
        self.scan_with_classifier(text, classifier, ClassifierType::Command)
            .await
    }

    async fn scan_prompt_with_classifier(&self, text: &str) -> Option<f32> {
        let classifier = self.prompt_classifier.as_ref()?;
        self.scan_with_classifier(text, classifier, ClassifierType::Prompt)
            .await
    }

    fn pattern_based_scanning(&self, text: &str) -> (f32, Vec<PatternMatch>) {
        let matches = self.pattern_matcher.scan_for_patterns(text);
        let confidence = self
            .pattern_matcher
            .get_max_risk_level(&matches)
            .map_or(0.0, |r| r.confidence_score());

        (confidence, matches)
    }

    fn build_explanation(&self, result: &DetailedScanResult, threshold: f32, tool_content: &str) -> String {
        if result.confidence < threshold {
            return "No security threats detected".to_string();
        }

        // Extract just the command from tool_content (skip "Tool: xxx" prefix)
        let command_preview = if let Some(args_start) = tool_content.find('\n') {
            let args = &tool_content[args_start + 1..];
            if args.len() > 150 {
                format!("{}...", &args[..150])
            } else {
                args.to_string()
            }
        } else {
            if tool_content.len() > 150 {
                format!("{}...", &tool_content[..150])
            } else {
                tool_content.to_string()
            }
        };

        if let Some(top_match) = result.pattern_matches.first() {
            let preview = top_match.matched_text.chars().take(50).collect::<String>();
            return format!(
                "Pattern-based detection: {} (Risk: {:?})\nFound: '{}'\n\nCommand:\n{}",
                top_match.threat.description, top_match.threat.risk_level, preview, command_preview
            );
        }

        if let Some(ml_conf) = result.ml_confidence {
            format!(
                "Command injection detected (confidence: {:.1}%)\n\nCommand:\n{}",
                ml_conf * 100.0,
                command_preview
            )
        } else {
            format!("Security threat detected\n\nCommand:\n{}", command_preview)
        }
    }

    fn extract_user_messages(&self, messages: &[Message], limit: usize) -> Vec<String> {
        messages
            .iter()
            .rev()
            .filter(|m| crate::conversation::effective_role(m) == "user")
            .take(limit)
            .map(|m| {
                m.content
                    .iter()
                    .filter_map(|c| match c {
                        crate::conversation::message::MessageContent::Text(t) => {
                            Some(t.text.clone())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn extract_tool_content(&self, tool_call: &CallToolRequestParam) -> String {
        let mut s = format!("Tool: {}", tool_call.name);
        if let Some(args) = &tool_call.arguments {
            if let Ok(json) = serde_json::to_string_pretty(args) {
                s.push('\n');
                s.push_str(&json);
            }
        }
        s
    }
}

impl Default for PromptInjectionScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::object;

    #[tokio::test]
    async fn test_text_pattern_detection() {
        let scanner = PromptInjectionScanner::new();
        let result = scanner.analyze_text("rm -rf /").await.unwrap();

        assert!(result.confidence >= 0.75); // High risk level = 0.75 confidence
        assert!(!result.pattern_matches.is_empty());
    }

    #[tokio::test]
    async fn test_conversation_scan_without_ml() {
        let scanner = PromptInjectionScanner::new();
        let result = scanner.scan_conversation(&[]).await.unwrap();

        assert_eq!(result.confidence, 0.0);
    }

    #[tokio::test]
    async fn test_tool_call_analysis() {
        let scanner = PromptInjectionScanner::new();

        let tool_call = CallToolRequestParam {
            task: None,
            name: "shell".into(),
            arguments: Some(object!({
                "command": "rm -rf /tmp/malicious"
            })),
        };

        let result = scanner
            .analyze_tool_call_with_context(&tool_call, &[])
            .await
            .unwrap();

        assert!(result.is_malicious);
        assert!(result.explanation.contains("Security threat"));
    }
}
