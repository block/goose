use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Conditions that can trigger adaptive learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// Quality drop detected over a time window
    QualityDrop {
        threshold: f32,      // Percentage drop (e.g., 0.1 = 10%)
        window_hours: u32,   // Time window to analyze
    },
    /// Negative feedback threshold reached
    FeedbackThreshold {
        negative_percentage: f32, // Percentage of negative feedback
        min_samples: usize,       // Minimum feedback samples required
    },
    /// Sufficient new training data available
    DataVolumeThreshold {
        min_new_examples: usize, // Minimum new examples since last training
    },
    /// Scheduled retraining (time-based)
    ScheduledRetraining {
        interval_hours: u32, // Hours between retraining
    },
    /// Performance degradation detected
    PerformanceDegradation {
        response_time_increase: f32, // Percentage increase in response time
        error_rate_increase: f32,    // Percentage increase in error rate
        window_hours: u32,           // Time window to analyze
    },
    /// User satisfaction drop
    SatisfactionDrop {
        threshold: f32,      // Satisfaction score threshold
        window_hours: u32,   // Time window to analyze
    },
    /// Domain shift detected
    DomainShift {
        new_domain_percentage: f32, // Percentage of queries in new domain
        min_samples: usize,         // Minimum samples to detect shift
    },
}

impl TriggerCondition {
    pub fn name(&self) -> String {
        match self {
            TriggerCondition::QualityDrop { .. } => "quality_drop".to_string(),
            TriggerCondition::FeedbackThreshold { .. } => "feedback_threshold".to_string(),
            TriggerCondition::DataVolumeThreshold { .. } => "data_volume_threshold".to_string(),
            TriggerCondition::ScheduledRetraining { .. } => "scheduled_retraining".to_string(),
            TriggerCondition::PerformanceDegradation { .. } => "performance_degradation".to_string(),
            TriggerCondition::SatisfactionDrop { .. } => "satisfaction_drop".to_string(),
            TriggerCondition::DomainShift { .. } => "domain_shift".to_string(),
        }
    }
}

/// Learning trigger that monitors conditions and decides when to retrain
pub struct LearningTrigger {
    pub condition: TriggerCondition,
    last_check: DateTime<Utc>,
    last_triggered: Option<DateTime<Utc>>,
    trigger_count: usize,
}

impl LearningTrigger {
    pub fn new(condition: TriggerCondition) -> Self {
        Self {
            condition,
            last_check: Utc::now(),
            last_triggered: None,
            trigger_count: 0,
        }
    }

    /// Check if this trigger condition is met
    pub async fn should_trigger(&mut self) -> Result<Option<String>> {
        let now = Utc::now();
        self.last_check = now;

        match &self.condition {
            TriggerCondition::QualityDrop { threshold, window_hours } => {
                self.check_quality_drop(*threshold, *window_hours).await
            }
            TriggerCondition::FeedbackThreshold { negative_percentage, min_samples } => {
                self.check_feedback_threshold(*negative_percentage, *min_samples).await
            }
            TriggerCondition::DataVolumeThreshold { min_new_examples } => {
                self.check_data_volume(*min_new_examples).await
            }
            TriggerCondition::ScheduledRetraining { interval_hours } => {
                self.check_scheduled_retraining(*interval_hours).await
            }
            TriggerCondition::PerformanceDegradation { 
                response_time_increase, 
                error_rate_increase, 
                window_hours 
            } => {
                self.check_performance_degradation(
                    *response_time_increase, 
                    *error_rate_increase, 
                    *window_hours
                ).await
            }
            TriggerCondition::SatisfactionDrop { threshold, window_hours } => {
                self.check_satisfaction_drop(*threshold, *window_hours).await
            }
            TriggerCondition::DomainShift { new_domain_percentage, min_samples } => {
                self.check_domain_shift(*new_domain_percentage, *min_samples).await
            }
        }
    }

    pub fn condition_name(&self) -> String {
        self.condition.name()
    }

    async fn check_quality_drop(&mut self, threshold: f32, window_hours: u32) -> Result<Option<String>> {
        // TODO: Implement actual quality monitoring
        // This would check model performance metrics over the specified window
        
        debug!("Checking quality drop trigger: threshold={}, window={}h", threshold, window_hours);
        
        // Placeholder logic
        let current_quality = 0.85; // Would come from performance monitor
        let baseline_quality = 0.90; // Would come from historical data
        
        let quality_drop = (baseline_quality - current_quality) / baseline_quality;
        
        if quality_drop > threshold {
            self.trigger_count += 1;
            self.last_triggered = Some(Utc::now());
            Ok(Some(format!(
                "Quality dropped by {:.1}% (threshold: {:.1}%)", 
                quality_drop * 100.0, 
                threshold * 100.0
            )))
        } else {
            Ok(None)
        }
    }

    async fn check_feedback_threshold(&mut self, negative_percentage: f32, min_samples: usize) -> Result<Option<String>> {
        // TODO: Implement actual feedback analysis
        // This would analyze recent user feedback
        
        debug!("Checking feedback threshold: negative_percentage={}, min_samples={}", 
               negative_percentage, min_samples);
        
        // Placeholder logic
        let total_feedback = 100; // Would come from feedback collector
        let negative_feedback = 35; // Would come from feedback collector
        
        if total_feedback >= min_samples {
            let negative_rate = negative_feedback as f32 / total_feedback as f32;
            
            if negative_rate > negative_percentage {
                self.trigger_count += 1;
                self.last_triggered = Some(Utc::now());
                Ok(Some(format!(
                    "Negative feedback rate: {:.1}% (threshold: {:.1}%)", 
                    negative_rate * 100.0, 
                    negative_percentage * 100.0
                )))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn check_data_volume(&mut self, min_new_examples: usize) -> Result<Option<String>> {
        // TODO: Implement actual data volume check
        // This would check training data collector for new examples
        
        debug!("Checking data volume threshold: min_new_examples={}", min_new_examples);
        
        // Placeholder logic
        let new_examples = 150; // Would come from training data collector
        
        if new_examples >= min_new_examples {
            self.trigger_count += 1;
            self.last_triggered = Some(Utc::now());
            Ok(Some(format!(
                "New training examples available: {} (threshold: {})", 
                new_examples, 
                min_new_examples
            )))
        } else {
            Ok(None)
        }
    }

    async fn check_scheduled_retraining(&mut self, interval_hours: u32) -> Result<Option<String>> {
        debug!("Checking scheduled retraining: interval={}h", interval_hours);
        
        let should_trigger = if let Some(last_triggered) = self.last_triggered {
            let hours_since = Utc::now().signed_duration_since(last_triggered).num_hours();
            hours_since >= interval_hours as i64
        } else {
            true // First time
        };
        
        if should_trigger {
            self.trigger_count += 1;
            self.last_triggered = Some(Utc::now());
            Ok(Some(format!("Scheduled retraining ({}h interval)", interval_hours)))
        } else {
            Ok(None)
        }
    }

    async fn check_performance_degradation(
        &mut self, 
        response_time_increase: f32, 
        error_rate_increase: f32, 
        window_hours: u32
    ) -> Result<Option<String>> {
        // TODO: Implement actual performance monitoring
        // This would check system performance metrics
        
        debug!("Checking performance degradation: response_time_increase={}, error_rate_increase={}, window={}h", 
               response_time_increase, error_rate_increase, window_hours);
        
        // Placeholder logic
        let current_response_time = 200.0; // ms
        let baseline_response_time = 150.0; // ms
        let current_error_rate = 0.05; // 5%
        let baseline_error_rate = 0.02; // 2%
        
        let response_time_degradation = (current_response_time - baseline_response_time) / baseline_response_time;
        let error_rate_degradation = (current_error_rate - baseline_error_rate) / baseline_error_rate;
        
        if response_time_degradation > response_time_increase || error_rate_degradation > error_rate_increase {
            self.trigger_count += 1;
            self.last_triggered = Some(Utc::now());
            Ok(Some(format!(
                "Performance degradation detected: response_time +{:.1}%, error_rate +{:.1}%", 
                response_time_degradation * 100.0,
                error_rate_degradation * 100.0
            )))
        } else {
            Ok(None)
        }
    }

    async fn check_satisfaction_drop(&mut self, threshold: f32, window_hours: u32) -> Result<Option<String>> {
        // TODO: Implement actual satisfaction monitoring
        // This would analyze user satisfaction metrics
        
        debug!("Checking satisfaction drop: threshold={}, window={}h", threshold, window_hours);
        
        // Placeholder logic
        let current_satisfaction = 3.8; // out of 5
        
        if current_satisfaction < threshold {
            self.trigger_count += 1;
            self.last_triggered = Some(Utc::now());
            Ok(Some(format!(
                "User satisfaction below threshold: {:.1} (threshold: {:.1})", 
                current_satisfaction, 
                threshold
            )))
        } else {
            Ok(None)
        }
    }

    async fn check_domain_shift(&mut self, new_domain_percentage: f32, min_samples: usize) -> Result<Option<String>> {
        // TODO: Implement actual domain shift detection
        // This would analyze query patterns and topics
        
        debug!("Checking domain shift: new_domain_percentage={}, min_samples={}", 
               new_domain_percentage, min_samples);
        
        // Placeholder logic
        let total_queries = 200;
        let new_domain_queries = 50;
        
        if total_queries >= min_samples {
            let new_domain_rate = new_domain_queries as f32 / total_queries as f32;
            
            if new_domain_rate > new_domain_percentage {
                self.trigger_count += 1;
                self.last_triggered = Some(Utc::now());
                Ok(Some(format!(
                    "Domain shift detected: {:.1}% new domain queries (threshold: {:.1}%)", 
                    new_domain_rate * 100.0, 
                    new_domain_percentage * 100.0
                )))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Get trigger statistics
    pub fn get_stats(&self) -> TriggerStats {
        TriggerStats {
            condition_name: self.condition_name(),
            trigger_count: self.trigger_count,
            last_check: self.last_check,
            last_triggered: self.last_triggered,
        }
    }
}

/// Statistics about a trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerStats {
    pub condition_name: String,
    pub trigger_count: usize,
    pub last_check: DateTime<Utc>,
    pub last_triggered: Option<DateTime<Utc>>,
}

/// Manager for multiple learning triggers
pub struct LearningTriggerManager {
    triggers: Vec<LearningTrigger>,
}

impl LearningTriggerManager {
    pub fn new() -> Self {
        Self {
            triggers: Vec::new(),
        }
    }

    /// Add a learning trigger
    pub fn add_trigger(&mut self, condition: TriggerCondition) {
        let trigger = LearningTrigger::new(condition);
        info!("Added learning trigger: {}", trigger.condition_name());
        self.triggers.push(trigger);
    }

    /// Check all triggers and return any that should fire
    pub async fn check_all_triggers(&mut self) -> Result<Vec<(TriggerCondition, String)>> {
        let mut triggered = Vec::new();
        
        for trigger in &mut self.triggers {
            if let Some(reason) = trigger.should_trigger().await? {
                triggered.push((trigger.condition.clone(), reason));
            }
        }
        
        Ok(triggered)
    }

    /// Get statistics for all triggers
    pub fn get_all_stats(&self) -> Vec<TriggerStats> {
        self.triggers.iter().map(|t| t.get_stats()).collect()
    }

    /// Remove a trigger by condition name
    pub fn remove_trigger(&mut self, condition_name: &str) -> bool {
        let initial_len = self.triggers.len();
        self.triggers.retain(|t| t.condition_name() != condition_name);
        self.triggers.len() < initial_len
    }

    /// Clear all triggers
    pub fn clear_triggers(&mut self) {
        self.triggers.clear();
        info!("Cleared all learning triggers");
    }
}

impl Default for LearningTriggerManager {
    fn default() -> Self {
        let mut manager = Self::new();
        
        // Add default triggers
        manager.add_trigger(TriggerCondition::QualityDrop { 
            threshold: 0.1, 
            window_hours: 6 
        });
        manager.add_trigger(TriggerCondition::FeedbackThreshold { 
            negative_percentage: 0.3, 
            min_samples: 50 
        });
        manager.add_trigger(TriggerCondition::DataVolumeThreshold { 
            min_new_examples: 500 
        });
        manager.add_trigger(TriggerCondition::ScheduledRetraining { 
            interval_hours: 168 // Weekly
        });
        
        manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduled_retraining_trigger() {
        let mut trigger = LearningTrigger::new(TriggerCondition::ScheduledRetraining { 
            interval_hours: 24 
        });
        
        // Should trigger on first check
        let result = trigger.should_trigger().await.unwrap();
        assert!(result.is_some());
        assert_eq!(trigger.trigger_count, 1);
        
        // Should not trigger immediately after
        let result = trigger.should_trigger().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_trigger_manager() {
        let mut manager = LearningTriggerManager::new();
        
        manager.add_trigger(TriggerCondition::DataVolumeThreshold { 
            min_new_examples: 100 
        });
        
        assert_eq!(manager.triggers.len(), 1);
        
        let stats = manager.get_all_stats();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].condition_name, "data_volume_threshold");
        
        let removed = manager.remove_trigger("data_volume_threshold");
        assert!(removed);
        assert_eq!(manager.triggers.len(), 0);
    }

    #[test]
    fn test_trigger_condition_names() {
        let conditions = vec![
            TriggerCondition::QualityDrop { threshold: 0.1, window_hours: 6 },
            TriggerCondition::FeedbackThreshold { negative_percentage: 0.3, min_samples: 50 },
            TriggerCondition::DataVolumeThreshold { min_new_examples: 100 },
        ];
        
        let names: Vec<String> = conditions.iter().map(|c| c.name()).collect();
        
        assert_eq!(names, vec![
            "quality_drop",
            "feedback_threshold", 
            "data_volume_threshold"
        ]);
    }
}
