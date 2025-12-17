use super::schema::QualityMetrics;
use crate::conversation::message::Message;
use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashSet;

/// Trait for scoring the quality of conversations for training data
#[async_trait]
pub trait QualityScorer: Send + Sync {
    /// Score a conversation and return quality metrics
    async fn score_conversation(
        &self,
        messages: &[Message],
        response_time: Option<f32>,
    ) -> Result<QualityMetrics>;
}

/// Simple rule-based quality scorer for development and testing
pub struct SimpleQualityScorer {
    code_patterns: Vec<Regex>,
    unsafe_patterns: Vec<Regex>,
    helpful_patterns: Vec<Regex>,
}

impl SimpleQualityScorer {
    pub fn new() -> Self {
        let code_patterns = vec![
            Regex::new(r"```[\s\S]*?```").unwrap(),
            Regex::new(r"`[^`]+`").unwrap(),
            Regex::new(r"def\s+\w+\s*\(").unwrap(),
            Regex::new(r"function\s+\w+\s*\(").unwrap(),
            Regex::new(r"class\s+\w+").unwrap(),
        ];

        let unsafe_patterns = vec![
            Regex::new(r#"(?i)(password|secret|token|key)\s*[:=]\s*['"][^'"]+['"]"#).unwrap(),
            Regex::new(r"(?i)(hack|exploit|malware|virus)").unwrap(),
            Regex::new(r"(?i)(illegal|harmful|dangerous)").unwrap(),
        ];

        let helpful_patterns = vec![
            Regex::new(r"(?i)(here's how|let me help|i can assist|step by step)").unwrap(),
            Regex::new(r"(?i)(example|for instance|specifically)").unwrap(),
            Regex::new(r"(?i)(explanation|because|reason|due to)").unwrap(),
        ];

        Self {
            code_patterns,
            unsafe_patterns,
            helpful_patterns,
        }
    }

    fn analyze_text_content(&self, text: &str) -> (bool, bool, f32, f32) {
        let contains_code = self
            .code_patterns
            .iter()
            .any(|pattern| pattern.is_match(text));

        let has_unsafe_content = self
            .unsafe_patterns
            .iter()
            .any(|pattern| pattern.is_match(text));

        let helpful_matches = self
            .helpful_patterns
            .iter()
            .map(|pattern| pattern.find_iter(text).count())
            .sum::<usize>();

        let helpfulness_score = (helpful_matches as f32 * 0.2).min(1.0);

        let safety_score = if has_unsafe_content { 0.0 } else { 1.0 };

        (
            contains_code,
            has_unsafe_content,
            helpfulness_score,
            safety_score,
        )
    }

    fn calculate_coherence_score(&self, messages: &[Message]) -> f32 {
        if messages.len() < 2 {
            return 0.5;
        }

        let mut coherence_score = 0.0;
        let mut valid_pairs = 0;

        for i in 0..messages.len() - 1 {
            let current_text = messages[i].as_concat_text().to_lowercase();
            let next_text = messages[i + 1].as_concat_text().to_lowercase();

            // Check for topic continuity (simple word overlap)
            let current_words: HashSet<&str> = current_text.split_whitespace().collect();
            let next_words: HashSet<&str> = next_text.split_whitespace().collect();

            let overlap = current_words.intersection(&next_words).count();
            let total_words = current_words.union(&next_words).count();

            if total_words > 0 {
                let similarity = overlap as f32 / total_words as f32;
                coherence_score += similarity;
                valid_pairs += 1;
            }
        }

        if valid_pairs > 0 {
            coherence_score / valid_pairs as f32
        } else {
            0.5
        }
    }

    fn detect_language(&self, text: &str) -> Option<String> {
        // Simple language detection based on character patterns
        let ascii_ratio = text.chars().filter(|c| c.is_ascii()).count() as f32 / text.len() as f32;

        if ascii_ratio > 0.95 {
            Some("en".to_string())
        } else {
            // Could be extended with more sophisticated language detection
            None
        }
    }

    fn score_tool_usage(&self, messages: &[Message]) -> Option<f32> {
        let tool_messages = messages
            .iter()
            .filter(|msg| msg.is_tool_call() || msg.is_tool_response())
            .count();

        if tool_messages == 0 {
            return None;
        }

        // Simple scoring: successful tool usage gets higher scores
        let successful_tools = messages
            .iter()
            .filter(|msg| {
                msg.content.iter().any(|content| {
                    if let Some(tool_response) = content.as_tool_response() {
                        tool_response.tool_result.is_ok()
                    } else {
                        false
                    }
                })
            })
            .count();

        if tool_messages > 0 {
            Some(successful_tools as f32 / tool_messages as f32)
        } else {
            None
        }
    }
}

impl Default for SimpleQualityScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QualityScorer for SimpleQualityScorer {
    async fn score_conversation(
        &self,
        messages: &[Message],
        response_time: Option<f32>,
    ) -> Result<QualityMetrics> {
        if messages.is_empty() {
            return Ok(QualityMetrics::default());
        }

        // Combine all text content for analysis
        let full_text = messages
            .iter()
            .map(|msg| msg.as_concat_text())
            .collect::<Vec<_>>()
            .join(" ");

        let (contains_code, has_unsafe_content, helpfulness_score, safety_score) =
            self.analyze_text_content(&full_text);

        let coherence_score = self.calculate_coherence_score(messages);
        let language_detected = self.detect_language(&full_text);
        let tool_usage_score = self.score_tool_usage(messages);
        let contains_tools = tool_usage_score.is_some();

        // Calculate overall score as weighted average
        let mut overall_score = 0.0;
        let mut weight_sum = 0.0;

        // Coherence (30%)
        overall_score += coherence_score * 0.3;
        weight_sum += 0.3;

        // Helpfulness (25%)
        overall_score += helpfulness_score * 0.25;
        weight_sum += 0.25;

        // Safety (25%)
        overall_score += safety_score * 0.25;
        weight_sum += 0.25;

        // Tool usage (10% if present)
        if let Some(tool_score) = tool_usage_score {
            overall_score += tool_score * 0.1;
            weight_sum += 0.1;
        }

        // Response time bonus/penalty (10%)
        let response_time_score = match response_time {
            Some(time) if time <= 2.0 => 1.0,  // Very fast
            Some(time) if time <= 5.0 => 0.8,  // Fast
            Some(time) if time <= 10.0 => 0.6, // Moderate
            Some(time) if time <= 30.0 => 0.4, // Slow
            Some(_) => 0.2,                    // Very slow
            None => 0.6,                       // Unknown
        };
        overall_score += response_time_score * 0.1;
        weight_sum += 0.1;

        // Normalize the overall score
        overall_score = if weight_sum > 0.0 {
            overall_score / weight_sum
        } else {
            0.5
        };

        // Apply penalties
        if has_unsafe_content {
            overall_score *= 0.1; // Severe penalty for unsafe content
        }

        // Length bonus for substantial conversations
        if messages.len() >= 4 {
            overall_score = (overall_score * 1.1).min(1.0);
        }

        Ok(QualityMetrics {
            overall_score,
            coherence_score,
            helpfulness_score,
            safety_score,
            tool_usage_score,
            conversation_length: messages.len(),
            response_time,
            contains_code,
            contains_tools,
            language_detected,
        })
    }
}

/// Advanced quality scorer that could use ML models for better scoring
pub struct MLQualityScorer {
    // Placeholder for future ML-based scoring
    fallback_scorer: SimpleQualityScorer,
}

impl MLQualityScorer {
    pub fn new() -> Self {
        Self {
            fallback_scorer: SimpleQualityScorer::new(),
        }
    }
}

#[async_trait]
impl QualityScorer for MLQualityScorer {
    async fn score_conversation(
        &self,
        messages: &[Message],
        response_time: Option<f32>,
    ) -> Result<QualityMetrics> {
        // For now, delegate to the simple scorer
        // In the future, this could use trained models for:
        // - Semantic coherence scoring
        // - Helpfulness classification
        // - Safety detection
        // - Domain-specific quality metrics

        self.fallback_scorer
            .score_conversation(messages, response_time)
            .await
    }
}

/// Composite quality scorer that combines multiple scoring strategies
pub struct CompositeQualityScorer {
    scorers: Vec<Box<dyn QualityScorer>>,
    weights: Vec<f32>,
}

impl CompositeQualityScorer {
    pub fn new() -> Self {
        Self {
            scorers: vec![Box::new(SimpleQualityScorer::new())],
            weights: vec![1.0],
        }
    }

    pub fn add_scorer(mut self, scorer: Box<dyn QualityScorer>, weight: f32) -> Self {
        self.scorers.push(scorer);
        self.weights.push(weight);
        self
    }
}

#[async_trait]
impl QualityScorer for CompositeQualityScorer {
    async fn score_conversation(
        &self,
        messages: &[Message],
        response_time: Option<f32>,
    ) -> Result<QualityMetrics> {
        if self.scorers.is_empty() {
            return Ok(QualityMetrics::default());
        }

        let mut combined_metrics = QualityMetrics::default();
        let mut total_weight = 0.0;

        for (scorer, &weight) in self.scorers.iter().zip(self.weights.iter()) {
            let metrics = scorer.score_conversation(messages, response_time).await?;

            combined_metrics.overall_score += metrics.overall_score * weight;
            combined_metrics.coherence_score += metrics.coherence_score * weight;
            combined_metrics.helpfulness_score += metrics.helpfulness_score * weight;
            combined_metrics.safety_score += metrics.safety_score * weight;

            if let Some(tool_score) = metrics.tool_usage_score {
                combined_metrics.tool_usage_score =
                    Some(combined_metrics.tool_usage_score.unwrap_or(0.0) + tool_score * weight);
            }

            total_weight += weight;
        }

        // Normalize by total weight
        if total_weight > 0.0 {
            combined_metrics.overall_score /= total_weight;
            combined_metrics.coherence_score /= total_weight;
            combined_metrics.helpfulness_score /= total_weight;
            combined_metrics.safety_score /= total_weight;

            if let Some(ref mut tool_score) = combined_metrics.tool_usage_score {
                *tool_score /= total_weight;
            }
        }

        // Use the first scorer's non-numeric metrics
        let first_metrics = self.scorers[0]
            .score_conversation(messages, response_time)
            .await?;
        combined_metrics.conversation_length = first_metrics.conversation_length;
        combined_metrics.response_time = first_metrics.response_time;
        combined_metrics.contains_code = first_metrics.contains_code;
        combined_metrics.contains_tools = first_metrics.contains_tools;
        combined_metrics.language_detected = first_metrics.language_detected;

        Ok(combined_metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;

    #[tokio::test]
    async fn test_simple_quality_scorer() {
        let scorer = SimpleQualityScorer::new();

        let messages = vec![
            Message::user().with_text("Can you help me write a Python function?"),
            Message::assistant().with_text("I'd be happy to help! Here's how you can write a function:\n\n```python\ndef hello_world():\n    print('Hello, World!')\n```"),
        ];

        let metrics = scorer
            .score_conversation(&messages, Some(2.0))
            .await
            .unwrap();

        assert!(metrics.overall_score > 0.5);
        assert!(metrics.contains_code);
        assert_eq!(metrics.safety_score, 1.0);
        assert!(metrics.helpfulness_score > 0.0);
        assert_eq!(metrics.conversation_length, 2);
    }

    #[tokio::test]
    async fn test_unsafe_content_detection() {
        let scorer = SimpleQualityScorer::new();

        let messages = vec![
            Message::user().with_text("What's my password?"),
            Message::assistant().with_text("Your password is: secret123"),
        ];

        let metrics = scorer.score_conversation(&messages, None).await.unwrap();

        assert_eq!(metrics.safety_score, 0.0);
        assert!(metrics.overall_score < 0.5); // Should be penalized
    }

    #[tokio::test]
    async fn test_coherence_scoring() {
        let scorer = SimpleQualityScorer::new();

        // Coherent conversation
        let coherent_messages = vec![
            Message::user().with_text("Tell me about Python programming"),
            Message::assistant()
                .with_text("Python is a programming language that's great for beginners"),
            Message::user().with_text("What makes Python good for programming?"),
            Message::assistant()
                .with_text("Python has simple syntax and powerful programming libraries"),
        ];

        let coherent_metrics = scorer
            .score_conversation(&coherent_messages, None)
            .await
            .unwrap();

        // Incoherent conversation
        let incoherent_messages = vec![
            Message::user().with_text("Tell me about cats"),
            Message::assistant().with_text("The weather is nice today"),
            Message::user().with_text("I like pizza"),
            Message::assistant().with_text("Quantum physics is complex"),
        ];

        let incoherent_metrics = scorer
            .score_conversation(&incoherent_messages, None)
            .await
            .unwrap();

        assert!(coherent_metrics.coherence_score > incoherent_metrics.coherence_score);
    }

    #[tokio::test]
    async fn test_composite_scorer() {
        let mut composite = CompositeQualityScorer::new()
            .add_scorer(Box::new(SimpleQualityScorer::new()), 0.7)
            .add_scorer(Box::new(MLQualityScorer::new()), 0.3);

        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there! How can I help you today?"),
        ];

        let metrics = composite
            .score_conversation(&messages, Some(1.5))
            .await
            .unwrap();

        assert!(metrics.overall_score >= 0.0 && metrics.overall_score <= 1.0);
        assert_eq!(metrics.conversation_length, 2);
    }
}
