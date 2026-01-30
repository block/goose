use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{info, debug, warn};
use uuid::Uuid;
use anyhow::Result;

/// User feedback event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEvent {
    pub id: Uuid,
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
    pub example_id: Option<Uuid>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub feedback_type: FeedbackType,
    pub rating: i32, // 1-5 scale
    pub correction: Option<String>,
    pub comments: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackType {
    ThumbsUp,
    ThumbsDown,
    Rating,
    Correction,
    Report,
    Suggestion,
    Implicit, // Derived from user behavior
}

/// Implicit feedback signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplicitFeedback {
    pub conversation_id: String,
    pub signal_type: ImplicitSignalType,
    pub confidence: f32, // 0.0 - 1.0
    pub timestamp: DateTime<Utc>,
}

/// Types of implicit feedback signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImplicitSignalType {
    ContinuedConversation { time_to_next_message: f32 },
    ImmediateClarification,
    ConversationAbandonment { time_spent: f32 },
    RepeatQuestion,
    PositiveFollowup,
    TaskCompletion,
}

/// Feedback collector that gathers user feedback for adaptive learning
pub struct FeedbackCollector {
    feedback_sender: mpsc::UnboundedSender<FeedbackEvent>,
    implicit_feedback_sender: mpsc::UnboundedSender<ImplicitFeedback>,
    enabled: Arc<RwLock<bool>>,
    feedback_history: Arc<RwLock<Vec<FeedbackEvent>>>,
    implicit_history: Arc<RwLock<Vec<ImplicitFeedback>>>,
}

impl FeedbackCollector {
    pub fn new(
        feedback_sender: mpsc::UnboundedSender<FeedbackEvent>,
        implicit_feedback_sender: mpsc::UnboundedSender<ImplicitFeedback>,
    ) -> Self {
        Self {
            feedback_sender,
            implicit_feedback_sender,
            enabled: Arc::new(RwLock::new(true)),
            feedback_history: Arc::new(RwLock::new(Vec::new())),
            implicit_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Enable or disable feedback collection
    pub async fn set_enabled(&self, enabled: bool) {
        let mut enabled_guard = self.enabled.write().await;
        *enabled_guard = enabled;
        info!("Feedback collection {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Check if feedback collection is enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Collect explicit user feedback
    pub async fn collect_feedback(&self, mut feedback: FeedbackEvent) -> Result<()> {
        if !self.is_enabled().await {
            return Ok(());
        }

        feedback.id = Uuid::new_v4();
        feedback.timestamp = Utc::now();

        debug!("Collecting feedback: {:?} for conversation {:?}", 
               feedback.feedback_type, feedback.conversation_id);

        // Store in history
        {
            let mut history = self.feedback_history.write().await;
            history.push(feedback.clone());
            
            // Keep only last 1000 feedback items
            if history.len() > 1000 {
                history.drain(0..history.len() - 1000);
            }
        }

        // Send to processing pipeline
        if let Err(e) = self.feedback_sender.send(feedback) {
            warn!("Failed to send feedback event: {}", e);
        }

        Ok(())
    }

    /// Collect implicit feedback signals
    pub async fn collect_implicit_feedback(&self, implicit: ImplicitFeedback) -> Result<()> {
        if !self.is_enabled().await {
            return Ok(());
        }

        debug!("Collecting implicit feedback: {:?} for conversation {}", 
               implicit.signal_type, implicit.conversation_id);

        // Store in history
        {
            let mut history = self.implicit_history.write().await;
            history.push(implicit.clone());
            
            // Keep only last 1000 implicit feedback items
            if history.len() > 1000 {
                history.drain(0..history.len() - 1000);
            }
        }

        // Send to processing pipeline
        if let Err(e) = self.implicit_feedback_sender.send(implicit) {
            warn!("Failed to send implicit feedback: {}", e);
        }

        Ok(())
    }

    /// Create a thumbs up feedback event
    pub fn create_thumbs_up(
        conversation_id: String,
        message_id: Option<String>,
        user_id: Option<String>,
    ) -> FeedbackEvent {
        FeedbackEvent {
            id: Uuid::new_v4(),
            conversation_id: Some(conversation_id),
            message_id,
            example_id: None,
            session_id: None,
            user_id,
            feedback_type: FeedbackType::ThumbsUp,
            rating: 5,
            correction: None,
            comments: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a thumbs down feedback event
    pub fn create_thumbs_down(
        conversation_id: String,
        message_id: Option<String>,
        user_id: Option<String>,
        reason: Option<String>,
    ) -> FeedbackEvent {
        FeedbackEvent {
            id: Uuid::new_v4(),
            conversation_id: Some(conversation_id),
            message_id,
            example_id: None,
            session_id: None,
            user_id,
            feedback_type: FeedbackType::ThumbsDown,
            rating: 1,
            correction: None,
            comments: reason,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a correction feedback event
    pub fn create_correction(
        conversation_id: String,
        message_id: String,
        correction: String,
        user_id: Option<String>,
    ) -> FeedbackEvent {
        FeedbackEvent {
            id: Uuid::new_v4(),
            conversation_id: Some(conversation_id),
            message_id: Some(message_id),
            example_id: None,
            session_id: None,
            user_id,
            feedback_type: FeedbackType::Correction,
            rating: 2, // Corrections imply dissatisfaction
            correction: Some(correction),
            comments: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a rating feedback event
    pub fn create_rating(
        conversation_id: String,
        rating: i32,
        user_id: Option<String>,
        comments: Option<String>,
    ) -> FeedbackEvent {
        FeedbackEvent {
            id: Uuid::new_v4(),
            conversation_id: Some(conversation_id),
            message_id: None,
            example_id: None,
            session_id: None,
            user_id,
            feedback_type: FeedbackType::Rating,
            rating: rating.clamp(1, 5),
            correction: None,
            comments,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Analyze user behavior to generate implicit feedback
    pub async fn analyze_conversation_behavior(
        &self,
        conversation_id: String,
        messages: &[crate::conversation::message::Message],
        session_duration: f32,
        abandoned: bool,
    ) -> Result<()> {
        if messages.len() < 2 {
            return Ok(());
        }

        // Analyze patterns in the conversation
        let mut implicit_signals = Vec::new();

        // Check for immediate clarification requests
        for i in 1..messages.len() {
            let prev_msg = &messages[i - 1];
            let curr_msg = &messages[i];
            
            if prev_msg.role == rmcp::model::Role::Assistant && 
               curr_msg.role == rmcp::model::Role::User {
                
                let curr_text = curr_msg.as_concat_text().to_lowercase();
                if curr_text.contains("what") || curr_text.contains("clarify") || 
                   curr_text.contains("explain") || curr_text.contains("mean") {
                    implicit_signals.push(ImplicitFeedback {
                        conversation_id: conversation_id.clone(),
                        signal_type: ImplicitSignalType::ImmediateClarification,
                        confidence: 0.7,
                        timestamp: Utc::now(),
                    });
                }
            }
        }

        // Check for conversation abandonment
        if abandoned && session_duration < 30.0 {
            implicit_signals.push(ImplicitFeedback {
                conversation_id: conversation_id.clone(),
                signal_type: ImplicitSignalType::ConversationAbandonment { 
                    time_spent: session_duration 
                },
                confidence: 0.8,
                timestamp: Utc::now(),
            });
        }

        // Check for positive follow-up patterns
        let last_user_msg = messages.iter()
            .filter(|msg| msg.role == rmcp::model::Role::User)
            .last();
        
        if let Some(last_msg) = last_user_msg {
            let text = last_msg.as_concat_text().to_lowercase();
            if text.contains("thank") || text.contains("perfect") || 
               text.contains("exactly") || text.contains("great") {
                implicit_signals.push(ImplicitFeedback {
                    conversation_id: conversation_id.clone(),
                    signal_type: ImplicitSignalType::PositiveFollowup,
                    confidence: 0.9,
                    timestamp: Utc::now(),
                });
            }
        }

        // Send all implicit signals
        for signal in implicit_signals {
            self.collect_implicit_feedback(signal).await?;
        }

        Ok(())
    }

    /// Get feedback statistics
    pub async fn get_feedback_stats(&self) -> FeedbackStats {
        let feedback_history = self.feedback_history.read().await;
        let implicit_history = self.implicit_history.read().await;

        let mut stats = FeedbackStats {
            total_feedback: feedback_history.len(),
            total_implicit_signals: implicit_history.len(),
            feedback_by_type: HashMap::new(),
            average_rating: 0.0,
            recent_feedback_trend: FeedbackTrend::Stable,
        };

        // Count feedback by type
        for feedback in feedback_history.iter() {
            *stats.feedback_by_type.entry(format!("{:?}", feedback.feedback_type)).or_insert(0) += 1;
        }

        // Calculate average rating
        let ratings: Vec<i32> = feedback_history.iter().map(|f| f.rating).collect();
        if !ratings.is_empty() {
            stats.average_rating = ratings.iter().sum::<i32>() as f32 / ratings.len() as f32;
        }

        // Determine trend (simplified)
        if feedback_history.len() >= 10 {
            let recent_ratings: Vec<i32> = feedback_history.iter()
                .rev()
                .take(10)
                .map(|f| f.rating)
                .collect();
            
            let older_ratings: Vec<i32> = feedback_history.iter()
                .rev()
                .skip(10)
                .take(10)
                .map(|f| f.rating)
                .collect();

            if !recent_ratings.is_empty() && !older_ratings.is_empty() {
                let recent_avg = recent_ratings.iter().sum::<i32>() as f32 / recent_ratings.len() as f32;
                let older_avg = older_ratings.iter().sum::<i32>() as f32 / older_ratings.len() as f32;
                
                let diff = recent_avg - older_avg;
                stats.recent_feedback_trend = if diff > 0.5 {
                    FeedbackTrend::Improving
                } else if diff < -0.5 {
                    FeedbackTrend::Declining
                } else {
                    FeedbackTrend::Stable
                };
            }
        }

        stats
    }

    /// Get recent feedback for analysis
    pub async fn get_recent_feedback(&self, limit: usize) -> Vec<FeedbackEvent> {
        let history = self.feedback_history.read().await;
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get recent implicit feedback
    pub async fn get_recent_implicit_feedback(&self, limit: usize) -> Vec<ImplicitFeedback> {
        let history = self.implicit_history.read().await;
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
}

/// Feedback statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackStats {
    pub total_feedback: usize,
    pub total_implicit_signals: usize,
    pub feedback_by_type: HashMap<String, usize>,
    pub average_rating: f32,
    pub recent_feedback_trend: FeedbackTrend,
}

/// Feedback trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackTrend {
    Improving,
    Stable,
    Declining,
}

/// Factory for creating feedback collectors
pub struct FeedbackCollectorFactory;

impl FeedbackCollectorFactory {
    /// Create a new feedback collector with channels
    pub fn create() -> (FeedbackCollector, mpsc::UnboundedReceiver<FeedbackEvent>, mpsc::UnboundedReceiver<ImplicitFeedback>) {
        let (feedback_sender, feedback_receiver) = mpsc::unbounded_channel();
        let (implicit_sender, implicit_receiver) = mpsc::unbounded_channel();
        
        let collector = FeedbackCollector::new(feedback_sender, implicit_sender);
        
        (collector, feedback_receiver, implicit_receiver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;

    #[tokio::test]
    async fn test_feedback_collection() {
        let (collector, mut feedback_receiver, _) = FeedbackCollectorFactory::create();

        let feedback = FeedbackCollector::create_thumbs_up(
            "conv_123".to_string(),
            Some("msg_456".to_string()),
            Some("user_789".to_string()),
        );

        collector.collect_feedback(feedback.clone()).await.unwrap();

        let received = feedback_receiver.recv().await.unwrap();
        assert_eq!(received.conversation_id, Some("conv_123".to_string()));
        assert_eq!(received.rating, 5);
        assert!(matches!(received.feedback_type, FeedbackType::ThumbsUp));
    }

    #[tokio::test]
    async fn test_implicit_feedback_analysis() {
        let (collector, _, mut implicit_receiver) = FeedbackCollectorFactory::create();

        let messages = vec![
            Message::user().with_text("How do I bake a cake?"),
            Message::assistant().with_text("Here's how to bake a cake..."),
            Message::user().with_text("What do you mean by 'fold the batter'?"),
        ];

        collector.analyze_conversation_behavior(
            "conv_123".to_string(),
            &messages,
            45.0,
            false,
        ).await.unwrap();

        let implicit = implicit_receiver.recv().await.unwrap();
        assert!(matches!(implicit.signal_type, ImplicitSignalType::ImmediateClarification));
        assert_eq!(implicit.conversation_id, "conv_123");
    }

    #[tokio::test]
    async fn test_feedback_stats() {
        let (collector, _feedback_receiver, _) = FeedbackCollectorFactory::create();

        // Add some test feedback
        let feedback1 = FeedbackCollector::create_rating("conv1".to_string(), 5, None, None);
        let feedback2 = FeedbackCollector::create_rating("conv2".to_string(), 3, None, None);
        let feedback3 = FeedbackCollector::create_thumbs_up("conv3".to_string(), None, None);

        collector.collect_feedback(feedback1).await.unwrap();
        collector.collect_feedback(feedback2).await.unwrap();
        collector.collect_feedback(feedback3).await.unwrap();

        let stats = collector.get_feedback_stats().await;
        assert_eq!(stats.total_feedback, 3);
        assert!((stats.average_rating - 4.33).abs() < 0.1); // (5+3+5)/3 ≈ 4.33
    }
}
