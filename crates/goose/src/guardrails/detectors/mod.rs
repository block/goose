//! Guardrails detectors
//!
//! This module contains all detector implementations for the guardrails system.

mod jailbreak_detector;
mod keyword_detector;
mod pii_detector;
mod prompt_injection_detector;
mod secret_detector;
mod topic_detector;

pub use jailbreak_detector::JailbreakDetector;
pub use keyword_detector::KeywordDetector;
pub use pii_detector::{PiiDetector, PiiType};
pub use prompt_injection_detector::PromptInjectionDetector;
pub use secret_detector::{SecretDetector, SecretType};
pub use topic_detector::TopicDetector;

use crate::guardrails::Severity;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context for detection operations
#[derive(Debug, Clone, Default)]
pub struct DetectionContext {
    /// Session identifier
    pub session_id: String,

    /// Optional user identifier
    pub user_id: Option<String>,

    /// Conversation history (for context-aware detection)
    pub conversation_history: Vec<String>,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DetectionContext {
    /// Create a new context with session ID
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            user_id: None,
            conversation_history: vec![],
            metadata: HashMap::new(),
        }
    }

    /// Add user ID to context
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Add conversation history
    pub fn with_history(mut self, history: Vec<String>) -> Self {
        self.conversation_history = history;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Result from a detector scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    /// Name of the detector
    pub detector_name: String,

    /// Whether a threat was detected
    pub detected: bool,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,

    /// Confidence threshold used
    pub threshold: f64,

    /// Severity of the detection
    pub severity: Severity,

    /// Evidence for the detection
    pub evidence: Vec<String>,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DetectionResult {
    /// Create a new detection result
    pub fn new(detector_name: &str) -> Self {
        Self {
            detector_name: detector_name.to_string(),
            detected: false,
            confidence: 0.0,
            threshold: 0.7,
            severity: Severity::Low,
            evidence: vec![],
            metadata: HashMap::new(),
        }
    }

    /// Mark as detected with confidence
    pub fn with_detection(mut self, confidence: f64, severity: Severity) -> Self {
        self.detected = true;
        self.confidence = confidence;
        self.severity = severity;
        self
    }

    /// Add evidence
    pub fn with_evidence(mut self, evidence: Vec<String>) -> Self {
        self.evidence = evidence;
        self
    }

    /// Set threshold
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    /// Create a "no detection" result
    pub fn no_detection(detector_name: &str, threshold: f64) -> Self {
        Self {
            detector_name: detector_name.to_string(),
            detected: false,
            confidence: 0.0,
            threshold,
            severity: Severity::Low,
            evidence: vec![],
            metadata: HashMap::new(),
        }
    }
}

/// Trait for all detectors
#[async_trait]
pub trait Detector: Send + Sync {
    /// Unique name for this detector
    fn name(&self) -> &'static str;

    /// Description of what this detector does
    fn description(&self) -> &'static str;

    /// Run detection on input text
    async fn detect(&self, input: &str, context: &DetectionContext) -> Result<DetectionResult>;

    /// Check if this detector is enabled
    fn is_enabled(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detection_context_creation() {
        let context = DetectionContext::new("session-123")
            .with_user_id("user-456")
            .with_history(vec!["Hello".to_string(), "World".to_string()])
            .with_metadata("key", serde_json::json!("value"));

        assert_eq!(context.session_id, "session-123");
        assert_eq!(context.user_id, Some("user-456".to_string()));
        assert_eq!(context.conversation_history.len(), 2);
        assert!(context.metadata.contains_key("key"));
    }

    #[test]
    fn test_detection_result_builder() {
        let result = DetectionResult::new("test_detector")
            .with_detection(0.85, Severity::High)
            .with_evidence(vec!["Found threat".to_string()])
            .with_threshold(0.7)
            .with_metadata("category", serde_json::json!("injection"));

        assert!(result.detected);
        assert_eq!(result.confidence, 0.85);
        assert_eq!(result.severity, Severity::High);
        assert!(!result.evidence.is_empty());
    }
}
