use crate::conversation::message::Message;
use crate::security::patterns::{PatternMatch, PatternMatcher};
use crate::security::prompt_ml_detector::MlDetector;
use anyhow::Result;
use rmcp::model::CallToolRequestParam;

const USER_SCAN_LIMIT: usize = 10;

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub is_malicious: bool,
    pub confidence: f32,
    pub explanation: String,
}

struct ToolCallScanResult {
    confidence: f32,
    pattern_matches: Vec<PatternMatch>,
    ml_confidence: Option<f32>,
}

#[derive(Default)]
struct MessageScanResult {
    confidence: f32,
    ml_confidence: Option<f32>,
}

pub struct PromptInjectionScanner {
    pattern_matcher: PatternMatcher,
    ml_detector: Option<MlDetector>,
}

impl PromptInjectionScanner {
    pub fn new() -> Self {
        Self {
            pattern_matcher: PatternMatcher::new(),
            ml_detector: None,
        }
    }

    pub fn with_ml_detection() -> Result<Self> {
        let ml_detector = MlDetector::new_from_config()?;
        Ok(Self {
            pattern_matcher: PatternMatcher::new(),
            ml_detector: Some(ml_detector),
        })
    }

    pub fn get_threshold_from_config(&self) -> f32 {
        use crate::config::Config;
        Config::global()
            .get_param::<f64>("SECURITY_PROMPT_THRESHOLD")
            .unwrap_or(0.7) as f32
    }

    pub async fn analyze_tool_call_with_context(
        &self,
        tool_call: &CallToolRequestParam,
        messages: &[Message],
    ) -> Result<ScanResult> {
        let threshold = self.get_threshold_from_config();
        let tool_content = self.extract_tool_content(tool_call);

        tracing::info!(
            "🔍 Scanning tool call: {} ({} chars)",
            tool_call.name,
            tool_content.len()
        );

        let (tool_result, context_result) = tokio::join!(
            self.scan_tool_call_content(&tool_content),
            self.scan_conversation_for_injection(messages)
        );

        let tool_result = tool_result?;
        let context_result = context_result?;

        let (max_confidence, pattern_matches, ml_confidence) =
            if tool_result.confidence >= context_result.confidence {
                (
                    tool_result.confidence,
                    Some(&tool_result.pattern_matches[..]),
                    tool_result.ml_confidence,
                )
            } else {
                (
                    context_result.confidence,
                    None,
                    context_result.ml_confidence,
                )
            };

        let explanation =
            self.build_explanation(max_confidence, threshold, pattern_matches, ml_confidence);

        tracing::info!(
            "✅ Security analysis complete: max_confidence={:.3}, malicious={}",
            max_confidence,
            max_confidence >= threshold
        );

        Ok(ScanResult {
            is_malicious: max_confidence >= threshold,
            confidence: max_confidence,
            explanation,
        })
    }

    async fn scan_tool_call_content(&self, text: &str) -> Result<ToolCallScanResult> {
        let (pattern_confidence, pattern_matches) = self.pattern_based_scanning(text);
        let ml_confidence = self.scan_with_ml(text).await;
        let confidence = ml_confidence.unwrap_or(0.0).max(pattern_confidence);
        let threshold = self.get_threshold_from_config();

        tracing::info!(
            "✅ Tool call scan complete: confidence={:.3}, malicious={}",
            confidence,
            confidence >= threshold
        );

        Ok(ToolCallScanResult {
            confidence,
            pattern_matches,
            ml_confidence,
        })
    }

    async fn scan_conversation_for_injection(
        &self,
        messages: &[Message],
    ) -> Result<MessageScanResult> {
        let user_messages = self.extract_user_messages(messages, USER_SCAN_LIMIT);

        if user_messages.is_empty() || self.ml_detector.is_none() {
            tracing::debug!(
                "ML detector not available or no user messages, skipping conversation scan"
            );
            return Ok(MessageScanResult::default());
        }

        tracing::info!(
            "🔍 Scanning {} recent user messages for injection ({} chars total)",
            user_messages.len(),
            user_messages.iter().map(|m| m.len()).sum::<usize>()
        );

        let scan_futures: Vec<_> = user_messages
            .iter()
            .map(|msg| self.scan_with_ml(msg))
            .collect();

        let results = futures::future::join_all(scan_futures).await;

        let max_confidence = results.into_iter().flatten().fold(0.0, f32::max);

        tracing::info!(
            "✅ Conversation scan complete: max_confidence={:.3}",
            max_confidence
        );

        Ok(MessageScanResult {
            confidence: max_confidence,
            ml_confidence: Some(max_confidence),
        })
    }

    async fn scan_with_ml(&self, text: &str) -> Option<f32> {
        let ml_detector = self.ml_detector.as_ref()?;

        tracing::debug!("🤖 Running ML scan ({} chars)", text.len());
        let start = std::time::Instant::now();

        match ml_detector.scan(text).await {
            Ok(conf) => {
                tracing::debug!(
                    "✅ ML scan: confidence={:.3}, duration={:.0}ms",
                    conf,
                    start.elapsed().as_secs_f64() * 1000.0
                );
                Some(conf)
            }
            Err(e) => {
                tracing::warn!("ML scan failed: {:#}", e);
                None
            }
        }
    }

    fn pattern_based_scanning(&self, text: &str) -> (f32, Vec<PatternMatch>) {
        let matches = self.pattern_matcher.scan_text(text);

        let confidence = self
            .pattern_matcher
            .get_max_risk_level(&matches)
            .map_or(0.0, |r| r.confidence_score());

        (confidence, matches)
    }

    fn build_explanation(
        &self,
        confidence: f32,
        threshold: f32,
        pattern_matches: Option<&[PatternMatch]>,
        ml_confidence: Option<f32>,
    ) -> String {
        if confidence < threshold {
            return "No security threats detected".to_string();
        }

        if let Some(top_match) = pattern_matches.and_then(|m| m.first()) {
            let preview = top_match.matched_text.chars().take(50).collect::<String>();
            return format!(
                "Security threat detected: {} (Risk: {:?}) - Found: '{}'",
                top_match.threat.description, top_match.threat.risk_level, preview
            );
        }

        if let Some(ml_conf) = ml_confidence {
            format!("Security threat detected (ML confidence: {:.2})", ml_conf)
        } else {
            "Security threat detected".to_string()
        }
    }

    fn extract_user_messages(&self, messages: &[Message], limit: usize) -> Vec<String> {
        messages
            .iter()
            .rev()
            .filter(|m| matches!(m.role, rmcp::model::Role::User))
            .take(limit)
            .filter_map(|m| {
                m.content.iter().find_map(|c| match c {
                    crate::conversation::message::MessageContent::Text(t) => Some(t.text.clone()),
                    _ => None,
                })
            })
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
    async fn test_tool_call_pattern_detection() {
        let scanner = PromptInjectionScanner::new();

        let result = scanner.scan_tool_call_content("rm -rf /").await.unwrap();

        assert!(result.confidence > 0.9);
        assert!(!result.pattern_matches.is_empty());
    }

    #[tokio::test]
    async fn test_conversation_scan_without_ml() {
        let scanner = PromptInjectionScanner::new();

        // Without ML detector, conversation scan should return 0 confidence
        let result = scanner.scan_conversation_for_injection(&[]).await.unwrap();

        assert_eq!(result.confidence, 0.0);
    }

    #[tokio::test]
    async fn test_tool_call_analysis() {
        let scanner = PromptInjectionScanner::new();

        let tool_call = CallToolRequestParam {
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
