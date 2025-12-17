use super::quality::QualityScorer;
use super::schema::{CollectionConfig, QualityMetrics, TrainingExample, UserFeedback};
use super::storage::TrainingDataStorage;
use crate::conversation::{message::Message, Conversation};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Service responsible for collecting training data from conversations
pub struct TrainingDataCollector {
    config: Arc<RwLock<CollectionConfig>>,
    storage: Arc<dyn TrainingDataStorage>,
    quality_scorer: Arc<dyn QualityScorer>,
    session_examples: Arc<RwLock<std::collections::HashMap<String, Vec<Uuid>>>>,
}

impl TrainingDataCollector {
    pub fn new(
        config: CollectionConfig,
        storage: Arc<dyn TrainingDataStorage>,
        quality_scorer: Arc<dyn QualityScorer>,
    ) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            storage,
            quality_scorer,
            session_examples: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Collect training data from a conversation
    pub async fn collect_from_conversation(
        &self,
        conversation_id: String,
        session_id: Option<String>,
        messages: &[Message],
        provider_used: String,
        model_used: String,
        response_time: Option<f32>,
    ) -> Result<Option<Uuid>> {
        let config = self.config.read().await;

        // Check if collection is enabled
        if !config.enabled {
            debug!("Training data collection is disabled");
            return Ok(None);
        }

        // Check session limits
        if let Some(ref session_id) = session_id {
            let session_examples = self.session_examples.read().await;
            if let Some(examples) = session_examples.get(session_id) {
                if examples.len() >= config.max_examples_per_session {
                    debug!("Session {} has reached max examples limit", session_id);
                    return Ok(None);
                }
            }
        }

        drop(config); // Release the lock

        // Create training example
        let mut example = TrainingExample::new(
            conversation_id,
            messages.to_vec(),
            provider_used,
            model_used,
        );

        if let Some(session_id) = session_id {
            example = example.with_session_id(session_id.clone());
        }

        // Score the quality of this example
        let quality_metrics = self
            .quality_scorer
            .score_conversation(messages, response_time)
            .await?;
        example = example.with_quality_metrics(quality_metrics);

        // Check if example meets quality thresholds
        let config = self.config.read().await;
        if !example.is_suitable_for_training(&config) {
            debug!("Example {} does not meet quality thresholds", example.id);
            return Ok(None);
        }
        drop(config);

        // Store the example
        let example_id = example.id;
        self.storage.store_example(example.clone()).await?;

        // Track session examples
        if let Some(session_id) = &example.session_id {
            let mut session_examples = self.session_examples.write().await;
            session_examples
                .entry(session_id.clone())
                .or_insert_with(Vec::new)
                .push(example_id);
        }

        info!("Collected training example {}", example_id);
        Ok(Some(example_id))
    }

    /// Add user feedback to an existing training example
    pub async fn add_user_feedback(&self, example_id: Uuid, feedback: UserFeedback) -> Result<()> {
        // Retrieve the example
        let mut example = self
            .storage
            .get_example(example_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Training example not found: {}", example_id))?;

        // Add feedback
        let updated_example = example.with_feedback(feedback);

        // Re-evaluate if it's suitable for training
        let config = self.config.read().await;
        if !updated_example.is_suitable_for_training(&config) {
            warn!(
                "Example {} no longer suitable for training after feedback",
                example_id
            );
        }

        // Update storage
        self.storage.update_example(updated_example).await?;

        info!("Updated training example {} with user feedback", example_id);
        Ok(())
    }

    /// Collect implicit feedback based on user behavior
    pub async fn collect_implicit_feedback(
        &self,
        example_id: Uuid,
        user_continued_conversation: bool,
        user_asked_clarification: bool,
        time_to_next_message: Option<f32>,
    ) -> Result<()> {
        use super::schema::{FeedbackRating, FeedbackType};
        use chrono::Utc;

        let feedback = if user_continued_conversation
            && time_to_next_message.map_or(true, |t| t > 30.0)
        {
            // User continued after a reasonable pause - likely positive
            UserFeedback {
                rating: FeedbackRating::Good,
                correction: None,
                comments: Some(
                    "Implicit positive feedback from continued conversation".to_string(),
                ),
                feedback_type: FeedbackType::ImplicitPositive,
                timestamp: Utc::now(),
            }
        } else if user_asked_clarification {
            // User immediately asked for clarification - likely negative
            UserFeedback {
                rating: FeedbackRating::Poor,
                correction: None,
                comments: Some(
                    "Implicit negative feedback from immediate clarification request".to_string(),
                ),
                feedback_type: FeedbackType::ImplicitNegative,
                timestamp: Utc::now(),
            }
        } else {
            return Ok(()); // No clear signal
        };

        self.add_user_feedback(example_id, feedback).await
    }

    /// Get training examples suitable for fine-tuning
    pub async fn get_training_examples(
        &self,
        limit: Option<usize>,
        min_quality_score: Option<f32>,
        domain_tags: Option<Vec<String>>,
    ) -> Result<Vec<TrainingExample>> {
        self.storage
            .get_examples_for_training(limit, min_quality_score, domain_tags)
            .await
    }

    /// Export training data in various formats
    pub async fn export_training_data(
        &self,
        format: ExportFormat,
        output_path: &str,
        filters: ExportFilters,
    ) -> Result<()> {
        let examples = self
            .storage
            .get_examples_for_training(
                filters.limit,
                filters.min_quality_score,
                filters.domain_tags,
            )
            .await?;

        match format {
            ExportFormat::JsonL => {
                self.export_jsonl(&examples, output_path).await?;
            }
            ExportFormat::ChatML => {
                self.export_chatml(&examples, output_path).await?;
            }
            ExportFormat::Alpaca => {
                self.export_alpaca(&examples, output_path).await?;
            }
        }

        info!(
            "Exported {} training examples to {}",
            examples.len(),
            output_path
        );
        Ok(())
    }

    /// Update collection configuration
    pub async fn update_config(&self, new_config: CollectionConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
        info!("Updated training data collection configuration");
    }

    /// Get current configuration
    pub async fn get_config(&self) -> CollectionConfig {
        self.config.read().await.clone()
    }

    /// Clean up old training examples based on retention policy
    pub async fn cleanup_old_examples(&self) -> Result<usize> {
        let config = self.config.read().await;
        if let Some(retention_days) = config.retention_days {
            let deleted_count = self.storage.delete_old_examples(retention_days).await?;
            info!("Cleaned up {} old training examples", deleted_count);
            Ok(deleted_count)
        } else {
            Ok(0)
        }
    }

    // Private helper methods for export formats
    async fn export_jsonl(&self, examples: &[TrainingExample], output_path: &str) -> Result<()> {
        use tokio::fs::File;
        use tokio::io::{AsyncWriteExt, BufWriter};

        let file = File::create(output_path).await?;
        let mut writer = BufWriter::new(file);

        for example in examples {
            let json_line = serde_json::to_string(&example.to_chat_format())?;
            writer.write_all(json_line.as_bytes()).await?;
            writer.write_all(b"\n").await?;
        }

        writer.flush().await?;
        Ok(())
    }

    async fn export_chatml(&self, examples: &[TrainingExample], output_path: &str) -> Result<()> {
        use tokio::fs::File;
        use tokio::io::{AsyncWriteExt, BufWriter};

        let file = File::create(output_path).await?;
        let mut writer = BufWriter::new(file);

        for example in examples {
            let mut chatml = String::new();

            for message in &example.messages {
                let role = match message.role {
                    rmcp::model::Role::User => "user",
                    rmcp::model::Role::Assistant => "assistant",
                };
                chatml.push_str(&format!(
                    "<|im_start|>{}\n{}<|im_end|>\n",
                    role,
                    message.as_concat_text()
                ));
            }

            let json_line = serde_json::json!({
                "text": chatml,
                "id": example.id,
                "quality_score": example.quality_metrics.overall_score
            });

            writer
                .write_all(serde_json::to_string(&json_line)?.as_bytes())
                .await?;
            writer.write_all(b"\n").await?;
        }

        writer.flush().await?;
        Ok(())
    }

    async fn export_alpaca(&self, examples: &[TrainingExample], output_path: &str) -> Result<()> {
        use tokio::fs::File;
        use tokio::io::{AsyncWriteExt, BufWriter};

        let file = File::create(output_path).await?;
        let mut writer = BufWriter::new(file);

        for example in examples {
            if example.messages.len() >= 2 {
                let instruction = example.messages[0].as_concat_text();
                let output = example.messages[1].as_concat_text();

                let alpaca_format = serde_json::json!({
                    "instruction": instruction,
                    "input": "",
                    "output": output,
                    "id": example.id,
                    "quality_score": example.quality_metrics.overall_score
                });

                writer
                    .write_all(serde_json::to_string(&alpaca_format)?.as_bytes())
                    .await?;
                writer.write_all(b"\n").await?;
            }
        }

        writer.flush().await?;
        Ok(())
    }
}

/// Export format options
#[derive(Debug, Clone)]
pub enum ExportFormat {
    JsonL,  // JSON Lines format
    ChatML, // ChatML format for training
    Alpaca, // Alpaca instruction format
}

/// Filters for exporting training data
#[derive(Debug, Clone)]
pub struct ExportFilters {
    pub limit: Option<usize>,
    pub min_quality_score: Option<f32>,
    pub domain_tags: Option<Vec<String>>,
}

impl Default for ExportFilters {
    fn default() -> Self {
        Self {
            limit: None,
            min_quality_score: Some(0.7),
            domain_tags: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use crate::training_data::quality::SimpleQualityScorer;
    use crate::training_data::storage::InMemoryTrainingDataStorage;

    #[tokio::test]
    async fn test_collect_from_conversation() {
        let config = CollectionConfig {
            enabled: true,
            min_quality_threshold: 0.5,
            ..Default::default()
        };

        let storage = Arc::new(InMemoryTrainingDataStorage::new());
        let quality_scorer = Arc::new(SimpleQualityScorer::new());

        let collector = TrainingDataCollector::new(config, storage.clone(), quality_scorer);

        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there! How can I help you?"),
        ];

        let result = collector
            .collect_from_conversation(
                "conv_123".to_string(),
                Some("session_456".to_string()),
                &messages,
                "native".to_string(),
                "llama-3.2-3b".to_string(),
                Some(1.5),
            )
            .await;

        assert!(result.is_ok());
        let example_id = result.unwrap();
        assert!(example_id.is_some());

        // Verify the example was stored
        let stored_example = storage.get_example(example_id.unwrap()).await.unwrap();
        assert!(stored_example.is_some());
    }

    #[tokio::test]
    async fn test_disabled_collection() {
        let config = CollectionConfig {
            enabled: false,
            ..Default::default()
        };

        let storage = Arc::new(InMemoryTrainingDataStorage::new());
        let quality_scorer = Arc::new(SimpleQualityScorer::new());

        let collector = TrainingDataCollector::new(config, storage, quality_scorer);

        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there!"),
        ];

        let result = collector
            .collect_from_conversation(
                "conv_123".to_string(),
                None,
                &messages,
                "native".to_string(),
                "llama-3.2-3b".to_string(),
                None,
            )
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
