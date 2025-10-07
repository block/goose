use serde::{Deserialize, Serialize};

/// Planning complexity levels for better plan categorization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlanningComplexity {
    Low,
    Medium,
    High,
    Expert,
}

impl PlanningComplexity {
    /// Estimate duration range for each complexity level
    pub fn estimated_duration(&self) -> &'static str {
        match self {
            PlanningComplexity::Low => "< 30 minutes",
            PlanningComplexity::Medium => "30-90 minutes", 
            PlanningComplexity::High => "1-3 hours",
            PlanningComplexity::Expert => "3+ hours",
        }
    }

    /// Get recommended max steps for each complexity level
    pub fn max_steps(&self) -> usize {
        match self {
            PlanningComplexity::Low => 3,
            PlanningComplexity::Medium => 8,
            PlanningComplexity::High => 15,
            PlanningComplexity::Expert => 25,
        }
    }
}

/// Planning metrics for tracking and optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningMetrics {
    pub complexity: PlanningComplexity,
    pub estimated_duration: String,
    pub steps_count: usize,
    pub parallel_opportunities: usize,
    pub risk_count: usize,
    pub planning_time_ms: u128,
    pub tokens_used: u32,
}

impl PlanningMetrics {
    pub fn new(
        complexity: PlanningComplexity,
        steps_count: usize,
        parallel_opportunities: usize,
        risk_count: usize,
        planning_time_ms: u128,
        tokens_used: u32,
    ) -> Self {
        Self {
            estimated_duration: complexity.estimated_duration().to_string(),
            complexity,
            steps_count,
            parallel_opportunities,
            risk_count,
            planning_time_ms,
            tokens_used,
        }
    }

    /// Calculate planning efficiency score (higher is better)
    pub fn efficiency_score(&self) -> f64 {
        let base_score = 100.0;
        let time_penalty = (self.planning_time_ms as f64) / 1000.0; // Penalty for slow planning
        let token_penalty = (self.tokens_used as f64) / 1000.0; // Penalty for high token usage
        let parallel_bonus = (self.parallel_opportunities as f64) * 5.0; // Bonus for identifying parallel tasks
        
        base_score - time_penalty - token_penalty + parallel_bonus
    }
}
