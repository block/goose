use crate::conversation::{message::Message, Conversation};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a single training example derived from a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    pub id: Uuid,
    pub conversation_id: String,
    pub session_id: Option<String>,
    pub messages: Vec<Message>,
    pub user_feedback: Option<UserFeedback>,
    pub quality_metrics: QualityMetrics,
    pub domain_tags: Vec<String>,
    pub metadata: TrainingMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User feedback on model responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    pub rating: FeedbackRating,
    pub correction: Option<String>,
    pub comments: Option<String>,
    pub feedback_type: FeedbackType,
    pub timestamp: DateTime<Utc>,
}

/// Rating scale for user feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackRating {
    Excellent,
    Good,
    Neutral,
    Poor,
    Terrible,
}

impl FeedbackRating {
    pub fn to_score(&self) -> f32 {
        match self {
            FeedbackRating::Excellent => 1.0,
            FeedbackRating::Good => 0.75,
            FeedbackRating::Neutral => 0.5,
            FeedbackRating::Poor => 0.25,
            FeedbackRating::Terrible => 0.0,
        }
    }
}

/// Type of feedback provided
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackType {
    Explicit,
    ThumbsUp,
    ThumbsDown,
    Correction,
    DetailedReview,
    ImplicitPositive, // Derived from user continuing conversation
    ImplicitNegative, // Derived from user immediately asking for clarification
}

/// Automated quality metrics for training examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub overall_score: f32,
    pub coherence_score: f32,
    pub helpfulness_score: f32,
    pub safety_score: f32,
    pub tool_usage_score: Option<f32>,
    pub conversation_length: usize,
    pub response_time: Option<f32>, // in seconds
    pub contains_code: bool,
    pub contains_tools: bool,
    pub language_detected: Option<String>,
}

impl Default for QualityMetrics {
    fn default() -> Self {
        Self {
            overall_score: 0.5,
            coherence_score: 0.5,
            helpfulness_score: 0.5,
            safety_score: 1.0, // Default to safe
            tool_usage_score: None,
            conversation_length: 0,
            response_time: None,
            contains_code: false,
            contains_tools: false,
            language_detected: None,
        }
    }
}

/// Additional metadata for training examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetadata {
    pub provider_used: String,
    pub model_used: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub context_length: usize,
    pub user_agent: Option<String>,
    pub source: TrainingSource,
    pub privacy_level: PrivacyLevel,
    pub custom_fields: HashMap<String, serde_json::Value>,
}

/// Source of the training data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrainingSource {
    LiveConversation,
    ImportedData,
    SyntheticGeneration,
    UserSubmission,
}

/// Privacy level for training data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyLevel {
    Public,    // Can be used freely
    Internal,  // Can be used within organization
    Private,   // Requires explicit consent
    Sensitive, // Should not be used for training
}

/// Configuration for training data collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    pub enabled: bool,
    pub min_quality_threshold: f32,
    pub max_examples_per_session: usize,
    pub collect_tool_interactions: bool,
    pub collect_code_examples: bool,
    pub require_user_consent: bool,
    pub retention_days: Option<u32>,
    pub excluded_domains: Vec<String>,
    pub privacy_mode: PrivacyLevel,
}

impl Default for CollectionConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Default to disabled for privacy
            min_quality_threshold: 0.6,
            max_examples_per_session: 10,
            collect_tool_interactions: true,
            collect_code_examples: true,
            require_user_consent: true,
            retention_days: Some(90), // 3 months default retention
            excluded_domains: vec![
                "password".to_string(),
                "secret".to_string(),
                "private".to_string(),
            ],
            privacy_mode: PrivacyLevel::Private,
        }
    }
}

impl TrainingExample {
    pub fn new(
        conversation_id: String,
        messages: Vec<Message>,
        provider_used: String,
        model_used: String,
    ) -> Self {
        let now = Utc::now();
        let conversation_length = messages.len();

        Self {
            id: Uuid::new_v4(),
            conversation_id,
            session_id: None,
            messages,
            user_feedback: None,
            quality_metrics: QualityMetrics {
                conversation_length,
                ..Default::default()
            },
            domain_tags: Vec::new(),
            metadata: TrainingMetadata {
                provider_used,
                model_used,
                temperature: None,
                max_tokens: None,
                context_length: conversation_length,
                user_agent: None,
                source: TrainingSource::LiveConversation,
                privacy_level: PrivacyLevel::Private,
                custom_fields: HashMap::new(),
            },
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_feedback(mut self, feedback: UserFeedback) -> Self {
        // Update quality score based on feedback
        if let Some(ref feedback) = self.user_feedback {
            self.quality_metrics.overall_score =
                (self.quality_metrics.overall_score + feedback.rating.to_score()) / 2.0;
        } else {
            self.quality_metrics.overall_score = feedback.rating.to_score();
        }

        self.user_feedback = Some(feedback);
        self.updated_at = Utc::now();
        self
    }

    pub fn with_quality_metrics(mut self, metrics: QualityMetrics) -> Self {
        self.quality_metrics = metrics;
        self.updated_at = Utc::now();
        self
    }

    pub fn with_domain_tags(mut self, tags: Vec<String>) -> Self {
        self.domain_tags = tags;
        self.updated_at = Utc::now();
        self
    }

    pub fn add_domain_tag(mut self, tag: String) -> Self {
        if !self.domain_tags.contains(&tag) {
            self.domain_tags.push(tag);
            self.updated_at = Utc::now();
        }
        self
    }

    /// Check if this example meets quality thresholds for training
    pub fn is_suitable_for_training(&self, config: &CollectionConfig) -> bool {
        // Check overall quality threshold
        if self.quality_metrics.overall_score < config.min_quality_threshold {
            return false;
        }

        // Check safety score
        if self.quality_metrics.safety_score < 0.8 {
            return false;
        }

        // Check privacy level
        match self.metadata.privacy_level {
            PrivacyLevel::Sensitive => return false,
            PrivacyLevel::Private if config.require_user_consent => {
                return self.user_feedback.is_some();
            }
            _ => {}
        }

        // Check for excluded domains
        let text_content = self.get_text_content();
        for excluded in &config.excluded_domains {
            if text_content
                .to_lowercase()
                .contains(&excluded.to_lowercase())
            {
                return false;
            }
        }

        true
    }

    /// Extract all text content from messages for analysis
    pub fn get_text_content(&self) -> String {
        self.messages
            .iter()
            .map(|msg| msg.as_concat_text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Convert to format suitable for fine-tuning (e.g., ChatML, Alpaca)
    pub fn to_chat_format(&self) -> serde_json::Value {
        let mut messages = Vec::new();

        for message in &self.messages {
            let role = match message.role {
                rmcp::model::Role::User => "user",
                rmcp::model::Role::Assistant => "assistant",
            };

            messages.push(serde_json::json!({
                "role": role,
                "content": message.as_concat_text()
            }));
        }

        serde_json::json!({
            "messages": messages,
            "id": self.id,
            "quality_score": self.quality_metrics.overall_score,
            "domain_tags": self.domain_tags
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;

    #[test]
    fn test_training_example_creation() {
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there! How can I help you?"),
        ];

        let example = TrainingExample::new(
            "conv_123".to_string(),
            messages,
            "openai".to_string(),
            "gpt-4".to_string(),
        );

        assert_eq!(example.conversation_id, "conv_123");
        assert_eq!(example.messages.len(), 2);
        assert_eq!(example.quality_metrics.conversation_length, 2);
        assert_eq!(example.metadata.provider_used, "openai");
        assert_eq!(example.metadata.model_used, "gpt-4");
    }

    #[test]
    fn test_feedback_rating_scores() {
        assert_eq!(FeedbackRating::Excellent.to_score(), 1.0);
        assert_eq!(FeedbackRating::Good.to_score(), 0.75);
        assert_eq!(FeedbackRating::Neutral.to_score(), 0.5);
        assert_eq!(FeedbackRating::Poor.to_score(), 0.25);
        assert_eq!(FeedbackRating::Terrible.to_score(), 0.0);
    }

    #[test]
    fn test_training_suitability() {
        let messages = vec![
            Message::user().with_text("What's the weather like?"),
            Message::assistant().with_text("I'd be happy to help with weather information!"),
        ];

        let mut example = TrainingExample::new(
            "conv_123".to_string(),
            messages,
            "native".to_string(),
            "llama-3.2-3b".to_string(),
        );

        // Set high quality score
        example.quality_metrics.overall_score = 0.8;
        example.quality_metrics.safety_score = 0.9;

        let config = CollectionConfig::default();
        assert!(example.is_suitable_for_training(&config));

        // Test with low quality score
        example.quality_metrics.overall_score = 0.3;
        assert!(!example.is_suitable_for_training(&config));
    }

    #[test]
    fn test_chat_format_conversion() {
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there!"),
        ];

        let example = TrainingExample::new(
            "conv_123".to_string(),
            messages,
            "native".to_string(),
            "llama-3.2-3b".to_string(),
        );

        let chat_format = example.to_chat_format();
        let messages_array = chat_format["messages"].as_array().unwrap();

        assert_eq!(messages_array.len(), 2);
        assert_eq!(messages_array[0]["role"], "user");
        assert_eq!(messages_array[0]["content"], "Hello");
        assert_eq!(messages_array[1]["role"], "assistant");
        assert_eq!(messages_array[1]["content"], "Hi there!");
    }
}

/// Pairwise preference example for DPO/IPO/KTO/ORPO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceExample {
    pub id: Uuid,
    pub prompt: String,
    pub chosen: String,
    pub rejected: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl PreferenceExample {
    pub fn new(prompt: String, chosen: String, rejected: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            prompt,
            chosen,
            rejected,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }
}
