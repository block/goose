use super::{
    ConversationHook, FeedbackCollector, LearningTrigger, ModelSwapper, PerformanceMonitor,
    FeedbackEvent, TriggerCondition, PerformanceAlert
};
use crate::training_data::{TrainingDataCollector, TrainingExample};
use crate::model_training::{
    ModelTrainer, TrainingJobManager, ModelVersionManager
};
use crate::model_training::job_manager::{TrainingJob, JobPriority, TrainingDataFilter};
use crate::conversation::{Conversation, message::Message};
use crate::providers::base::{Provider, ProviderUsage};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, broadcast};
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Configuration for the adaptive learning system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveLearningConfig {
    pub enabled: bool,
    pub data_collection_enabled: bool,
    pub automatic_retraining_enabled: bool,
    pub min_examples_for_training: usize,
    pub max_training_frequency_hours: u32,
    pub performance_monitoring_interval_minutes: u32,
    pub feedback_collection_enabled: bool,
    pub model_swapping_enabled: bool,
    pub learning_triggers: Vec<TriggerCondition>,
}

impl Default for AdaptiveLearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            data_collection_enabled: true,
            automatic_retraining_enabled: true,
            min_examples_for_training: 100,
            max_training_frequency_hours: 24,
            performance_monitoring_interval_minutes: 15,
            feedback_collection_enabled: true,
            model_swapping_enabled: true,
            learning_triggers: vec![
                TriggerCondition::QualityDrop { threshold: 0.1, window_hours: 6 },
                TriggerCondition::FeedbackThreshold { negative_percentage: 0.3, min_samples: 50 },
                TriggerCondition::DataVolumeThreshold { min_new_examples: 500 },
            ],
        }
    }
}

/// Events in the adaptive learning system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdaptiveLearningEvent {
    ConversationCompleted {
        conversation_id: String,
        session_id: Option<String>,
        messages: Vec<Message>,
        provider_used: String,
        model_used: String,
        response_time: Option<f32>,
    },
    FeedbackReceived {
        feedback: FeedbackEvent,
    },
    LearningTriggered {
        trigger: TriggerCondition,
        reason: String,
    },
    TrainingStarted {
        job_id: Uuid,
        trigger_reason: String,
    },
    TrainingCompleted {
        job_id: Uuid,
        new_version_id: Uuid,
        performance_improvement: Option<f32>,
    },
    ModelSwapped {
        old_version_id: Uuid,
        new_version_id: Uuid,
        swap_reason: String,
    },
    PerformanceAlert {
        alert: PerformanceAlert,
    },
}

/// The main orchestrator that coordinates all adaptive learning components
pub struct AdaptiveLearningOrchestrator {
    config: Arc<RwLock<AdaptiveLearningConfig>>,
    
    // Core components
    data_collector: Arc<TrainingDataCollector>,
    job_manager: Arc<TrainingJobManager>,
    version_manager: Arc<ModelVersionManager>,
    
    // Adaptive learning components
    conversation_hook: Arc<ConversationHook>,
    feedback_collector: Arc<FeedbackCollector>,
    learning_triggers: Arc<RwLock<Vec<LearningTrigger>>>,
    model_swapper: Arc<ModelSwapper>,
    performance_monitor: Arc<PerformanceMonitor>,
    
    // Communication channels
    event_sender: broadcast::Sender<AdaptiveLearningEvent>,
    feedback_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<FeedbackEvent>>>>,
    
    // State tracking
    last_training_time: Arc<RwLock<Option<DateTime<Utc>>>>,
    active_model_version: Arc<RwLock<Option<Uuid>>>,
    performance_history: Arc<RwLock<Vec<PerformanceSnapshot>>>,
}

/// Performance snapshot for tracking model performance over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub timestamp: DateTime<Utc>,
    pub model_version_id: Uuid,
    pub response_time_ms: f32,
    pub error_rate: f32,
    pub user_satisfaction: f32,
    pub throughput_rps: f32,
}

impl AdaptiveLearningOrchestrator {
    pub fn new(
        config: AdaptiveLearningConfig,
        data_collector: Arc<TrainingDataCollector>,
        job_manager: Arc<TrainingJobManager>,
        version_manager: Arc<ModelVersionManager>,
        conversation_hook: Arc<ConversationHook>,
        feedback_collector: Arc<FeedbackCollector>,
        model_swapper: Arc<ModelSwapper>,
        performance_monitor: Arc<PerformanceMonitor>,
    ) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        Self {
            config: Arc::new(RwLock::new(config)),
            data_collector,
            job_manager,
            version_manager,
            conversation_hook,
            feedback_collector,
            learning_triggers: Arc::new(RwLock::new(Vec::new())),
            model_swapper,
            performance_monitor,
            event_sender,
            feedback_receiver: Arc::new(RwLock::new(None)),
            last_training_time: Arc::new(RwLock::new(None)),
            active_model_version: Arc::new(RwLock::new(None)),
            performance_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start the adaptive learning system
    pub async fn start(&self) -> Result<()> {
        let config = self.config.read().await;
        
        if !config.enabled {
            info!("Adaptive learning system is disabled");
            return Ok(());
        }

        info!("Starting adaptive learning orchestrator");

        // Initialize learning triggers
        self.initialize_learning_triggers(&config.learning_triggers).await?;

        // Setup feedback collection
        if config.feedback_collection_enabled {
            self.setup_feedback_collection().await?;
        }

        // Start conversation monitoring
        if config.data_collection_enabled {
            self.start_conversation_monitoring().await?;
        }

        // Start performance monitoring
        self.start_performance_monitoring(config.performance_monitoring_interval_minutes).await?;

        // Start the main orchestration loop
        self.start_orchestration_loop().await?;

        info!("Adaptive learning system started successfully");
        Ok(())
    }

    /// Process a completed conversation
    pub async fn process_conversation(
        &self,
        conversation_id: String,
        session_id: Option<String>,
        messages: Vec<Message>,
        provider_used: String,
        model_used: String,
        response_time: Option<f32>,
    ) -> Result<()> {
        let config = self.config.read().await;
        
        if !config.enabled || !config.data_collection_enabled {
            return Ok(());
        }

        debug!("Processing conversation: {}", conversation_id);

        // Collect training data
        if let Some(example_id) = self.data_collector.collect_from_conversation(
            conversation_id.clone(),
            session_id.clone(),
            &messages,
            provider_used.clone(),
            model_used.clone(),
            response_time,
        ).await? {
            debug!("Collected training example: {}", example_id);
        }

        // Emit event
        let event = AdaptiveLearningEvent::ConversationCompleted {
            conversation_id,
            session_id,
            messages,
            provider_used,
            model_used,
            response_time,
        };
        let _ = self.event_sender.send(event);

        // Check if we should trigger learning
        self.check_learning_triggers().await?;

        Ok(())
    }

    /// Process user feedback
    pub async fn process_feedback(&self, feedback: FeedbackEvent) -> Result<()> {
        let config = self.config.read().await;
        
        if !config.enabled || !config.feedback_collection_enabled {
            return Ok(());
        }

        debug!("Processing feedback: {:?}", feedback);

        // Store feedback with training data collector
        if let Some(example_id) = feedback.example_id {
            // Convert FeedbackEvent to UserFeedback
            let user_feedback = crate::training_data::schema::UserFeedback {
                rating: match feedback.rating {
                    1 => crate::training_data::schema::FeedbackRating::Terrible,
                    2 => crate::training_data::schema::FeedbackRating::Poor,
                    3 => crate::training_data::schema::FeedbackRating::Neutral,
                    4 => crate::training_data::schema::FeedbackRating::Good,
                    5 => crate::training_data::schema::FeedbackRating::Excellent,
                    _ => crate::training_data::schema::FeedbackRating::Neutral,
                },
                correction: feedback.correction,
                comments: feedback.comments,
                feedback_type: crate::training_data::schema::FeedbackType::DetailedReview,
                timestamp: feedback.timestamp,
            };

            self.data_collector.add_user_feedback(example_id, user_feedback).await?;
        }

        // Emit event
        let event = AdaptiveLearningEvent::FeedbackReceived { feedback };
        let _ = self.event_sender.send(event);

        // Check if feedback triggers learning
        self.check_learning_triggers().await?;

        Ok(())
    }

    async fn initialize_learning_triggers(&self, conditions: &[TriggerCondition]) -> Result<()> {
        let mut triggers = self.learning_triggers.write().await;
        triggers.clear();

        for condition in conditions {
            let trigger = LearningTrigger::new(condition.clone());
            triggers.push(trigger);
        }

        info!("Initialized {} learning triggers", triggers.len());
        Ok(())
    }

    async fn setup_feedback_collection(&self) -> Result<()> {
        // Setup feedback collection pipeline
        info!("Setting up feedback collection");
        Ok(())
    }

    async fn start_conversation_monitoring(&self) -> Result<()> {
        info!("Starting conversation monitoring");
        // The conversation hook will be integrated into the provider system
        Ok(())
    }

    async fn start_performance_monitoring(&self, interval_minutes: u32) -> Result<()> {
        let performance_monitor = self.performance_monitor.clone();
        let performance_history = self.performance_history.clone();
        let active_model_version = self.active_model_version.clone();
        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
            let mut interval = interval(TokioDuration::from_secs(interval_minutes as u64 * 60));
            
            loop {
                interval.tick().await;
                
                if let Ok(metrics) = performance_monitor.collect_current_metrics().await {
                    let model_version_id = active_model_version.read().await.unwrap_or(Uuid::new_v4());
                    
                    let snapshot = PerformanceSnapshot {
                        timestamp: Utc::now(),
                        model_version_id,
                        response_time_ms: metrics.response_time_ms,
                        error_rate: metrics.error_rate,
                        user_satisfaction: metrics.user_satisfaction,
                        throughput_rps: metrics.throughput_rps,
                    };

                    // Store snapshot
                    {
                        let mut history = performance_history.write().await;
                        history.push(snapshot);
                        
                        // Keep only last 24 hours of data
                        let cutoff = Utc::now() - Duration::hours(24);
                        history.retain(|s| s.timestamp > cutoff);
                    }

                    // Check for performance alerts
                    if let Some(alert) = performance_monitor.check_for_alerts(&metrics).await {
                        let event = AdaptiveLearningEvent::PerformanceAlert { alert };
                        let _ = event_sender.send(event);
                    }
                }
            }
        });

        info!("Performance monitoring started with {}-minute intervals", interval_minutes);
        Ok(())
    }

    async fn start_orchestration_loop(&self) -> Result<()> {
        let mut event_receiver = self.event_sender.subscribe();
        let orchestrator = Arc::new(self.clone());

        tokio::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                if let Err(e) = orchestrator.handle_event(event).await {
                    error!("Error handling adaptive learning event: {}", e);
                }
            }
        });

        info!("Orchestration loop started");
        Ok(())
    }

    async fn handle_event(&self, event: AdaptiveLearningEvent) -> Result<()> {
        match event {
            AdaptiveLearningEvent::LearningTriggered { trigger, reason } => {
                self.handle_learning_trigger(trigger, reason).await?;
            }
            AdaptiveLearningEvent::TrainingCompleted { job_id, new_version_id, performance_improvement } => {
                self.handle_training_completion(job_id, new_version_id, performance_improvement).await?;
            }
            AdaptiveLearningEvent::PerformanceAlert { alert } => {
                self.handle_performance_alert(alert).await?;
            }
            _ => {
                // Other events are handled elsewhere
            }
        }
        Ok(())
    }

    async fn check_learning_triggers(&self) -> Result<()> {
        let triggers = self.learning_triggers.read().await;
        
        for trigger in triggers.iter() {
            if let Some(reason) = trigger.should_trigger().await? {
                info!("Learning trigger activated: {} - {}", trigger.condition_name(), reason);
                
                let event = AdaptiveLearningEvent::LearningTriggered {
                    trigger: trigger.condition.clone(),
                    reason: reason.clone(),
                };
                let _ = self.event_sender.send(event);
            }
        }
        
        Ok(())
    }

    async fn handle_learning_trigger(&self, trigger: TriggerCondition, reason: String) -> Result<()> {
        let config = self.config.read().await;
        
        if !config.automatic_retraining_enabled {
            info!("Automatic retraining is disabled, skipping trigger");
            return Ok(());
        }

        // Check if we've trained recently
        {
            let last_training = self.last_training_time.read().await;
            if let Some(last_time) = *last_training {
                let hours_since = Utc::now().signed_duration_since(last_time).num_hours();
                if hours_since < config.max_training_frequency_hours as i64 {
                    info!("Training frequency limit reached, skipping trigger");
                    return Ok(());
                }
            }
        }

        // Get training examples
        let examples = self.data_collector.get_training_examples(
            Some(config.min_examples_for_training * 2), // Get more than minimum
            Some(0.6), // Quality threshold
            None, // No domain filter
        ).await?;

        if examples.len() < config.min_examples_for_training {
            info!("Not enough training examples ({} < {}), skipping training", 
                  examples.len(), config.min_examples_for_training);
            return Ok(());
        }

        // Create training job
        let job = self.create_adaptive_training_job(trigger, reason.clone(), examples.len()).await?;
        let job_id = self.job_manager.submit_job(job).await?;

        // Update last training time
        {
            let mut last_training = self.last_training_time.write().await;
            *last_training = Some(Utc::now());
        }

        info!("Started adaptive training job: {} (reason: {})", job_id, reason);

        let event = AdaptiveLearningEvent::TrainingStarted {
            job_id,
            trigger_reason: reason,
        };
        let _ = self.event_sender.send(event);

        Ok(())
    }

    async fn create_adaptive_training_job(
        &self,
        trigger: TriggerCondition,
        reason: String,
        num_examples: usize,
    ) -> Result<TrainingJob> {
        use crate::model_training::job_manager::{TrainingJobBuilder, TrainingDataFilter, ResourceRequirements};
        use std::path::PathBuf;

        let job = TrainingJobBuilder::new(
            format!("adaptive-training-{}", Utc::now().format("%Y%m%d-%H%M%S")),
            PathBuf::from("models/base"), // TODO: Get actual base model path
        )
        .description(format!("Adaptive training triggered by: {}", reason))
        .priority(JobPriority::High)
        .training_data_filter(TrainingDataFilter {
            min_quality_score: Some(0.7),
            max_examples: Some(num_examples),
            require_feedback: false,
            ..Default::default()
        })
        .resource_requirements(ResourceRequirements {
            min_memory_gb: 8.0,
            preferred_device: "cuda".to_string(),
            max_training_time_hours: Some(6.0),
            disk_space_gb: 20.0,
        })
        .created_by("adaptive_learning_system".to_string())
        .tags(vec![
            "adaptive".to_string(),
            "automatic".to_string(),
            format!("trigger_{}", trigger.name()),
        ])
        .build();

        Ok(job)
    }

    async fn handle_training_completion(
        &self,
        job_id: Uuid,
        new_version_id: Uuid,
        performance_improvement: Option<f32>,
    ) -> Result<()> {
        info!("Training job {} completed, new version: {}", job_id, new_version_id);

        let config = self.config.read().await;
        
        if config.model_swapping_enabled {
            // Evaluate if we should swap to the new model
            if let Some(improvement) = performance_improvement {
                if improvement > 0.05 { // 5% improvement threshold
                    info!("Performance improvement detected ({}%), initiating model swap", improvement * 100.0);
                    
                    if let Err(e) = self.model_swapper.initiate_swap(new_version_id, "performance_improvement".to_string()).await {
                        error!("Failed to initiate model swap: {}", e);
                    }
                } else {
                    info!("Performance improvement too small ({}%), keeping current model", improvement * 100.0);
                }
            } else {
                info!("No performance improvement data, keeping current model");
            }
        }

        Ok(())
    }

    async fn handle_performance_alert(&self, alert: PerformanceAlert) -> Result<()> {
        warn!("Performance alert: {:?}", alert);

        match alert {
            PerformanceAlert::QualityDegradation { .. } => {
                // Trigger immediate retraining
                let event = AdaptiveLearningEvent::LearningTriggered {
                    trigger: TriggerCondition::QualityDrop { threshold: 0.1, window_hours: 1 },
                    reason: "Performance alert: quality degradation detected".to_string(),
                };
                let _ = self.event_sender.send(event);
            }
            PerformanceAlert::HighErrorRate { .. } => {
                // Consider rolling back to previous version
                warn!("High error rate detected, consider rollback");
            }
            PerformanceAlert::SlowResponse { .. } => {
                // Optimize current model or trigger retraining
                info!("Slow response detected, monitoring for improvement");
            }
        }

        Ok(())
    }

    /// Get current system status
    pub async fn get_status(&self) -> AdaptiveLearningStatus {
        let config = self.config.read().await;
        let last_training = *self.last_training_time.read().await;
        let active_version = *self.active_model_version.read().await;
        let performance_history = self.performance_history.read().await;

        AdaptiveLearningStatus {
            enabled: config.enabled,
            data_collection_enabled: config.data_collection_enabled,
            automatic_retraining_enabled: config.automatic_retraining_enabled,
            last_training_time: last_training,
            active_model_version: active_version,
            recent_performance: performance_history.last().cloned(),
            total_performance_snapshots: performance_history.len(),
        }
    }

    /// Subscribe to adaptive learning events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<AdaptiveLearningEvent> {
        self.event_sender.subscribe()
    }
}

// Clone implementation for Arc usage in async tasks
impl Clone for AdaptiveLearningOrchestrator {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            data_collector: self.data_collector.clone(),
            job_manager: self.job_manager.clone(),
            version_manager: self.version_manager.clone(),
            conversation_hook: self.conversation_hook.clone(),
            feedback_collector: self.feedback_collector.clone(),
            learning_triggers: self.learning_triggers.clone(),
            model_swapper: self.model_swapper.clone(),
            performance_monitor: self.performance_monitor.clone(),
            event_sender: self.event_sender.clone(),
            feedback_receiver: self.feedback_receiver.clone(),
            last_training_time: self.last_training_time.clone(),
            active_model_version: self.active_model_version.clone(),
            performance_history: self.performance_history.clone(),
        }
    }
}

/// Status information for the adaptive learning system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveLearningStatus {
    pub enabled: bool,
    pub data_collection_enabled: bool,
    pub automatic_retraining_enabled: bool,
    pub last_training_time: Option<DateTime<Utc>>,
    pub active_model_version: Option<Uuid>,
    pub recent_performance: Option<PerformanceSnapshot>,
    pub total_performance_snapshots: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_learning_config() {
        let config = AdaptiveLearningConfig::default();
        assert!(config.enabled);
        assert!(config.data_collection_enabled);
        assert_eq!(config.min_examples_for_training, 100);
    }

    #[test]
    fn test_performance_snapshot() {
        let snapshot = PerformanceSnapshot {
            timestamp: Utc::now(),
            model_version_id: Uuid::new_v4(),
            response_time_ms: 150.0,
            error_rate: 0.01,
            user_satisfaction: 4.2,
            throughput_rps: 100.0,
        };
        
        assert_eq!(snapshot.response_time_ms, 150.0);
        assert_eq!(snapshot.error_rate, 0.01);
    }
}
