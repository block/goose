//! Topic Detector
//!
//! Detects banned or off-topic content based on configurable topic lists.
//! Supports both blocklist (ban specific topics) and allowlist (only allow specific topics) modes.

use super::{DetectionContext, DetectionResult, Detector};
use crate::guardrails::config::{TopicConfig, TopicMode};
use crate::guardrails::Severity;
use anyhow::Result;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Built-in topic patterns with associated keywords and phrases
static TOPIC_PATTERNS: Lazy<HashMap<&'static str, TopicPattern>> = Lazy::new(|| {
    let mut map = HashMap::new();

    map.insert(
        "violence",
        TopicPattern {
            name: "violence",
            severity: Severity::High,
            keywords: vec![
                "kill", "murder", "assault", "attack", "stab", "shoot",
                "bomb", "explosion", "massacre", "torture", "harm",
                "weapon", "violent", "hurt", "injure", "wound",
            ],
            phrases: vec![
                "how to kill",
                "ways to hurt",
                "make a bomb",
                "build a weapon",
                "cause harm to",
            ],
        },
    );

    map.insert(
        "illegal_activities",
        TopicPattern {
            name: "illegal_activities",
            severity: Severity::Critical,
            keywords: vec![
                "illegal", "crime", "criminal", "theft", "steal", "fraud",
                "smuggling", "trafficking", "counterfeiting", "hacking",
                "piracy", "embezzlement", "bribery", "extortion",
            ],
            phrases: vec![
                "how to hack",
                "break into",
                "steal money",
                "commit fraud",
                "evade taxes",
                "launder money",
            ],
        },
    );

    map.insert(
        "self_harm",
        TopicPattern {
            name: "self_harm",
            severity: Severity::Critical,
            keywords: vec![
                "suicide", "self-harm", "cutting", "overdose", "kill myself",
                "end my life", "die", "death wish",
            ],
            phrases: vec![
                "want to die",
                "kill myself",
                "end my life",
                "hurt myself",
                "ways to die",
                "painless death",
            ],
        },
    );

    map.insert(
        "hate_speech",
        TopicPattern {
            name: "hate_speech",
            severity: Severity::High,
            keywords: vec![
                "racist", "racism", "sexist", "sexism", "homophobic",
                "transphobic", "xenophobic", "discriminate", "supremacy",
                "hate", "bigot", "slur",
            ],
            phrases: vec![
                "inferior race",
                "hate group",
                "ethnic cleansing",
            ],
        },
    );

    map.insert(
        "drugs",
        TopicPattern {
            name: "drugs",
            severity: Severity::Medium,
            keywords: vec![
                "cocaine", "heroin", "methamphetamine", "meth", "fentanyl",
                "mdma", "lsd", "ecstasy", "opioid", "narcotic",
            ],
            phrases: vec![
                "how to make drugs",
                "synthesize drugs",
                "cook meth",
                "grow marijuana",
                "make lsd",
            ],
        },
    );

    map.insert(
        "explicit_content",
        TopicPattern {
            name: "explicit_content",
            severity: Severity::Medium,
            keywords: vec![
                "pornography", "explicit", "nsfw", "xxx", "adult content",
                "sexual content", "erotic",
            ],
            phrases: vec![
                "sexual fantasy",
                "explicit scene",
                "adult story",
            ],
        },
    );

    map.insert(
        "gambling",
        TopicPattern {
            name: "gambling",
            severity: Severity::Low,
            keywords: vec![
                "gambling", "casino", "betting", "wager", "poker",
                "slot machine", "lottery", "bookmaker",
            ],
            phrases: vec![
                "gambling strategy",
                "beat the casino",
                "winning system",
            ],
        },
    );

    map.insert(
        "financial_advice",
        TopicPattern {
            name: "financial_advice",
            severity: Severity::Low,
            keywords: vec![
                "investment advice", "stock tip", "guarantee returns",
                "get rich quick", "insider trading",
            ],
            phrases: vec![
                "guaranteed profit",
                "can't lose money",
                "insider information",
                "pump and dump",
            ],
        },
    );

    map.insert(
        "medical_advice",
        TopicPattern {
            name: "medical_advice",
            severity: Severity::Medium,
            keywords: vec![
                "prescription", "diagnosis", "treatment for",
                "cure for", "medical advice",
            ],
            phrases: vec![
                "prescribe medication",
                "diagnose my condition",
                "should I take",
                "stop taking medication",
            ],
        },
    );

    map.insert(
        "legal_advice",
        TopicPattern {
            name: "legal_advice",
            severity: Severity::Low,
            keywords: vec![
                "legal advice", "sue someone", "lawsuit", "attorney",
                "legal strategy", "court case",
            ],
            phrases: vec![
                "should I sue",
                "legal defense",
                "avoid prosecution",
            ],
        },
    );

    map
});

/// Topic pattern definition
struct TopicPattern {
    name: &'static str,
    severity: Severity,
    keywords: Vec<&'static str>,
    phrases: Vec<&'static str>,
}

/// Topic detector
#[derive(Default)]
pub struct TopicDetector {
    config: TopicConfig,
}

impl TopicDetector {
    /// Create with custom configuration
    pub fn with_config(config: TopicConfig) -> Self {
        Self { config }
    }

    /// Check if a topic is in the banned list
    fn is_banned(&self, topic: &str) -> bool {
        self.config.banned_topics.iter().any(|t| {
            t.eq_ignore_ascii_case(topic) || topic.eq_ignore_ascii_case(t)
        })
    }

    /// Check if a topic is in the allowed list
    fn is_allowed(&self, topic: &str) -> bool {
        self.config.allowed_topics.iter().any(|t| {
            t.eq_ignore_ascii_case(topic) || topic.eq_ignore_ascii_case(t)
        })
    }

    /// Detect topics in input text
    fn detect_topics(&self, input: &str) -> Vec<(&'static str, Severity, Vec<String>)> {
        let input_lower = input.to_lowercase();
        let mut detected: Vec<(&'static str, Severity, Vec<String>)> = Vec::new();

        for (topic_name, pattern) in TOPIC_PATTERNS.iter() {
            // Check based on mode
            match self.config.mode {
                TopicMode::Blocklist => {
                    if !self.is_banned(topic_name) {
                        continue;
                    }
                }
                TopicMode::Allowlist => {
                    if self.is_allowed(topic_name) {
                        continue;
                    }
                }
            }

            let mut matches: Vec<String> = Vec::new();

            // Check phrases first (more specific)
            for phrase in &pattern.phrases {
                if input_lower.contains(phrase) {
                    matches.push(format!("phrase: \"{}\"", phrase));
                }
            }

            // Check keywords
            for keyword in &pattern.keywords {
                // Use word boundary matching
                let keyword_pattern = format!(r"\b{}\b", regex::escape(keyword));
                if let Ok(re) = regex::Regex::new(&format!("(?i){}", keyword_pattern)) {
                    if re.is_match(&input_lower) {
                        matches.push(format!("keyword: \"{}\"", keyword));
                    }
                }
            }

            if !matches.is_empty() {
                detected.push((pattern.name, pattern.severity, matches));
            }
        }

        detected
    }
}


#[async_trait]
impl Detector for TopicDetector {
    fn name(&self) -> &'static str {
        "topic"
    }

    fn description(&self) -> &'static str {
        "Detects banned or off-topic content based on configurable topic lists"
    }

    async fn detect(&self, input: &str, _context: &DetectionContext) -> Result<DetectionResult> {
        if !self.config.enabled {
            return Ok(DetectionResult::no_detection(
                self.name(),
                self.config.confidence_threshold,
            ));
        }

        let detected_topics = self.detect_topics(input);

        if detected_topics.is_empty() {
            return Ok(DetectionResult::no_detection(
                self.name(),
                self.config.confidence_threshold,
            ));
        }

        // Get highest severity
        let max_severity = detected_topics
            .iter()
            .map(|(_, s, _)| *s)
            .max()
            .unwrap_or(Severity::Low);

        // Build evidence
        let evidence: Vec<String> = detected_topics
            .iter()
            .map(|(topic, severity, matches)| {
                format!(
                    "Topic '{}' ({:?}): {}",
                    topic,
                    severity,
                    matches.join(", ")
                )
            })
            .collect();

        // Calculate confidence
        let base_confidence = match max_severity {
            Severity::Critical => 0.9,
            Severity::High => 0.8,
            Severity::Medium => 0.7,
            Severity::Low => 0.6,
        };

        let topic_boost = ((detected_topics.len() - 1) as f64 * 0.03).min(0.09);
        let confidence =
            (base_confidence + topic_boost) * self.config.sensitivity.multiplier();

        let detected = confidence >= self.config.confidence_threshold;

        // Build metadata
        let mut metadata = std::collections::HashMap::new();
        let topics: Vec<String> = detected_topics.iter().map(|(t, _, _)| t.to_string()).collect();
        metadata.insert("topics_found".to_string(), serde_json::json!(topics));

        Ok(DetectionResult {
            detector_name: self.name().to_string(),
            detected,
            confidence: confidence.min(0.99),
            threshold: self.config.confidence_threshold,
            severity: max_severity,
            evidence,
            metadata,
        })
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_banned_topics(topics: Vec<&str>) -> TopicConfig {
        TopicConfig {
            enabled: true,
            sensitivity: crate::guardrails::config::Sensitivity::Medium,
            confidence_threshold: 0.7,
            banned_topics: topics.into_iter().map(String::from).collect(),
            allowed_topics: vec![],
            mode: TopicMode::Blocklist,
        }
    }

    #[tokio::test]
    async fn test_violence_detection() {
        let config = config_with_banned_topics(vec!["violence"]);
        let detector = TopicDetector::with_config(config);
        let context = DetectionContext::default();

        let inputs = vec![
            "How to kill someone",
            "Ways to hurt people",
            "Build a bomb at home",
        ];

        for input in inputs {
            let result = detector.detect(input, &context).await.unwrap();
            assert!(result.detected, "Should detect violence in: {}", input);
        }
    }

    #[tokio::test]
    async fn test_illegal_activities_detection() {
        let config = config_with_banned_topics(vec!["illegal_activities"]);
        let detector = TopicDetector::with_config(config);
        let context = DetectionContext::default();

        let inputs = vec![
            "How to hack into a bank",
            "Ways to steal money",
            "Commit fraud without getting caught",
        ];

        for input in inputs {
            let result = detector.detect(input, &context).await.unwrap();
            assert!(
                result.detected,
                "Should detect illegal activity in: {}",
                input
            );
        }
    }

    #[tokio::test]
    async fn test_self_harm_detection() {
        let config = config_with_banned_topics(vec!["self_harm"]);
        let detector = TopicDetector::with_config(config);
        let context = DetectionContext::default();

        let result = detector
            .detect("I want to kill myself", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert_eq!(result.severity, Severity::Critical);
    }

    #[tokio::test]
    async fn test_unbanned_topic_allowed() {
        let config = config_with_banned_topics(vec!["violence"]); // Only violence banned
        let detector = TopicDetector::with_config(config);
        let context = DetectionContext::default();

        // Gambling is not banned, so should not trigger
        let result = detector
            .detect("Best gambling strategy for poker", &context)
            .await
            .unwrap();

        assert!(!result.detected);
    }

    #[tokio::test]
    async fn test_allowlist_mode() {
        let config = TopicConfig {
            enabled: true,
            sensitivity: crate::guardrails::config::Sensitivity::Medium,
            confidence_threshold: 0.7,
            banned_topics: vec![],
            allowed_topics: vec!["gambling".to_string()], // Only gambling allowed
            mode: TopicMode::Allowlist,
        };

        let detector = TopicDetector::with_config(config);
        let context = DetectionContext::default();

        // Gambling is allowed
        let gambling_result = detector
            .detect("Gambling strategy for poker", &context)
            .await
            .unwrap();

        // Gambling should not trigger in allowlist mode when it's allowed
        // Note: In allowlist mode, topics NOT in the allow list are blocked
        // The implementation blocks everything except allowed topics
        assert!(!gambling_result.detected || gambling_result.confidence < gambling_result.threshold);
    }

    #[tokio::test]
    async fn test_safe_content() {
        let config = config_with_banned_topics(vec!["violence", "illegal_activities"]);
        let detector = TopicDetector::with_config(config);
        let context = DetectionContext::default();

        let inputs = vec![
            "How do I write a sorting algorithm?",
            "What is the capital of France?",
            "Explain quantum computing",
        ];

        for input in inputs {
            let result = detector.detect(input, &context).await.unwrap();
            assert!(!result.detected, "False positive for: {}", input);
        }
    }

    #[tokio::test]
    async fn test_multiple_topics() {
        let config = config_with_banned_topics(vec!["violence", "drugs"]);
        let detector = TopicDetector::with_config(config);
        let context = DetectionContext::default();

        // Use explicit keywords from both topics: "kill" (violence) + "cocaine" (drugs)
        let result = detector
            .detect("How to kill someone with cocaine", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert!(result.evidence.len() >= 2, "Expected 2+ evidence items, got: {:?}", result.evidence); // Both topics detected
    }
}
