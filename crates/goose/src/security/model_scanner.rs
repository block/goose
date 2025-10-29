use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use super::super::providers::errors::ProviderError;

/// Generic result of prompt injection detection from any model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelScanResult {
    /// Whether prompt injection was detected
    pub is_injection: bool,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Raw scores from the model (format depends on model type)
    pub raw_scores: Vec<f64>,
    /// Model-specific metadata (optional)
    pub metadata: Option<serde_json::Value>,
}

impl ModelScanResult {
    /// Create a new result
    pub fn new(is_injection: bool, confidence: f64, raw_scores: Vec<f64>) -> Self {
        Self {
            is_injection,
            confidence,
            raw_scores,
            metadata: None,
        }
    }

    /// Create a result with metadata
    pub fn with_metadata(is_injection: bool, confidence: f64, raw_scores: Vec<f64>, metadata: serde_json::Value) -> Self {
        Self {
            is_injection,
            confidence,
            raw_scores,
            metadata: Some(metadata),
        }
    }
}

/// Trait for model-based prompt injection scanners
#[async_trait]
pub trait ModelScanner: Send + Sync {
    /// Scan text for prompt injection using the underlying model
    async fn scan_text(&self, text: &str) -> Result<ModelScanResult, ProviderError>;
    
    /// Check if the model service is available
    async fn is_available(&self) -> bool;
    
    /// Get the model name/identifier
    fn model_name(&self) -> &str;
    
    /// Get the model version (optional)
    fn model_version(&self) -> Option<&str> {
        None
    }
}

/// Generic model scanning functionality
pub struct GenericModelScanner;

impl GenericModelScanner {
    /// Scan text for prompt injection using any model scanner implementation
    /// This provides common logging, error handling, and validation logic
    pub async fn scan_for_prompt_injection<T: ModelScanner>(
        scanner: &T,
        text: &str,
    ) -> Result<ModelScanResult, ProviderError> {
        let model_name = scanner.model_name();
        let model_version = scanner.model_version().unwrap_or("unknown");
        
        tracing::debug!("🔒 Starting model scan with {} (version: {}) for text length: {}", 
                       model_name, model_version, text.len());
        
        // Check if text is empty
        if text.trim().is_empty() {
            tracing::warn!("🔒 Empty text provided to model scanner");
            return Ok(ModelScanResult::new(false, 0.0, vec![]));
        }
        
        // Check if model is available before scanning
        if !scanner.is_available().await {
            return Err(ProviderError::RequestFailed(format!(
                "Model scanner {} is not available", 
                model_name
            )));
        }
        
        // Perform the actual scan
        let start_time = std::time::Instant::now();
        let result = scanner.scan_text(text).await?;
        let duration = start_time.elapsed();
        
        tracing::debug!(
            "🔒 Model scan completed in {:?} - model: {}, is_injection: {}, confidence: {:.3}, raw_scores: {:?}",
            duration, model_name, result.is_injection, result.confidence, result.raw_scores
        );
        
        // Validate result
        if result.confidence < 0.0 || result.confidence > 1.0 {
            tracing::warn!("🔒 Model {} returned invalid confidence: {:.3}", model_name, result.confidence);
        }
        
        Ok(result)
    }
    
    /// Batch scan multiple texts (useful for models that support batch processing)
    pub async fn batch_scan_for_prompt_injection<T: ModelScanner>(
        scanner: &T,
        texts: &[String],
    ) -> Result<Vec<ModelScanResult>, ProviderError> {
        let model_name = scanner.model_name();
        
        tracing::debug!("🔒 Starting batch model scan with {} for {} texts", model_name, texts.len());
        
        if texts.is_empty() {
            return Ok(vec![]);
        }
        
        // For now, scan each text individually
        // Future implementations could optimize this for models that support true batch processing
        let mut results = Vec::with_capacity(texts.len());
        
        for (i, text) in texts.iter().enumerate() {
            tracing::debug!("🔒 Scanning text {} of {} with {}", i + 1, texts.len(), model_name);
            let result = Self::scan_for_prompt_injection(scanner, text).await?;
            results.push(result);
        }
        
        tracing::debug!("🔒 Batch scan completed for {} texts with {}", texts.len(), model_name);
        Ok(results)
    }
}

/// Utility functions for working with model scan results
impl ModelScanResult {
    /// Apply softmax to convert logits to probabilities
    /// This is commonly needed for BERT-style models that output raw logits
    pub fn from_binary_logits(safe_logit: f64, injection_logit: f64) -> Self {
        tracing::debug!("Converting binary logits: safe={:.3}, injection={:.3}", safe_logit, injection_logit);
        
        // Apply softmax to convert logits to probabilities
        // softmax(x_i) = exp(x_i) / sum(exp(x_j))
        let safe_exp = safe_logit.exp();
        let injection_exp = injection_logit.exp();
        let sum_exp = safe_exp + injection_exp;
        
        let safe_prob = safe_exp / sum_exp;
        let injection_prob = injection_exp / sum_exp;
        
        tracing::debug!("Softmax probabilities: safe={:.3}, injection={:.3}", safe_prob, injection_prob);
        
        // Determine if injection is detected based on which probability is higher
        let is_injection = injection_prob > safe_prob;
        
        // Confidence is the probability of the predicted class
        let confidence = if is_injection {
            injection_prob
        } else {
            safe_prob
        };

        tracing::debug!("Final binary classification: is_injection={}, confidence={:.3}", is_injection, confidence);

        Self::new(is_injection, confidence, vec![safe_logit, injection_logit])
    }
    
    /// Create result from raw scores with validation
    pub fn from_raw_scores_with_validation(scores: Vec<f64>, expected_length: usize) -> Self {
        if scores.len() != expected_length {
            tracing::warn!("Expected {} scores from model, got {}", expected_length, scores.len());
            return Self::new(false, 0.0, scores);
        }
        
        // For binary classification (most common case)
        if expected_length == 2 {
            return Self::from_binary_logits(scores[0], scores[1]);
        }
        
        // For other cases, use simple max-based classification
        let max_idx = scores
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        
        // Assume last class is "injection" for multi-class scenarios
        let is_injection = max_idx == scores.len() - 1;
        let confidence = scores[max_idx].max(0.0).min(1.0); // Clamp to [0, 1]
        
        Self::new(is_injection, confidence, scores)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_scan_result_from_binary_logits() {
        // Test case where injection is detected (injection_logit > safe_logit)
        let result = ModelScanResult::from_binary_logits(2.0, 5.0);
        
        assert!(result.is_injection);
        // With softmax: safe_prob = exp(2)/(exp(2)+exp(5)) ≈ 0.047, injection_prob ≈ 0.953
        assert!(result.confidence > 0.9);
        assert_eq!(result.raw_scores, vec![2.0, 5.0]);

        // Test case where no injection is detected (safe_logit > injection_logit)
        let result = ModelScanResult::from_binary_logits(5.0, 2.0);
        
        assert!(!result.is_injection);
        // With softmax: safe_prob ≈ 0.953, injection_prob ≈ 0.047
        assert!(result.confidence > 0.9);
        assert_eq!(result.raw_scores, vec![5.0, 2.0]);

        // Test case with equal logits (uncertain)
        let result = ModelScanResult::from_binary_logits(3.0, 3.0);
        
        // With equal logits, softmax gives 0.5 probability for each class
        assert!((result.confidence - 0.5).abs() < 0.001);
        assert_eq!(result.raw_scores, vec![3.0, 3.0]);
    }

    #[test]
    fn test_model_scan_result_from_raw_scores_with_validation() {
        // Test binary classification
        let result = ModelScanResult::from_raw_scores_with_validation(vec![5.977, -6.504], 2);
        assert!(!result.is_injection);
        assert!(result.confidence > 0.99);

        // Test wrong number of scores
        let result = ModelScanResult::from_raw_scores_with_validation(vec![1.0], 2);
        assert!(!result.is_injection);
        assert_eq!(result.confidence, 0.0);
        assert_eq!(result.raw_scores, vec![1.0]);
    }

    #[test]
    fn test_model_scan_result_new() {
        let result = ModelScanResult::new(true, 0.85, vec![1.0, 2.0]);
        assert!(result.is_injection);
        assert_eq!(result.confidence, 0.85);
        assert_eq!(result.raw_scores, vec![1.0, 2.0]);
        assert!(result.metadata.is_none());
    }

    #[test]
    fn test_model_scan_result_with_metadata() {
        let metadata = serde_json::json!({"model": "test", "version": "1.0"});
        let result = ModelScanResult::with_metadata(false, 0.95, vec![3.0, 1.0], metadata.clone());
        assert!(!result.is_injection);
        assert_eq!(result.confidence, 0.95);
        assert_eq!(result.metadata, Some(metadata));
    }
}