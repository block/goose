//! Keyword Detector
//!
//! Detects custom keyword blocklists in text.
//! Supports exact match, case-insensitive, and fuzzy matching.

use super::{DetectionContext, DetectionResult, Detector};
use crate::guardrails::config::KeywordConfig;
use crate::guardrails::Severity;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashSet;

/// Keyword detector
pub struct KeywordDetector {
    config: KeywordConfig,
    /// Compiled keywords for fast lookup
    keywords_lower: HashSet<String>,
}

impl KeywordDetector {
    /// Create with custom configuration
    pub fn with_config(config: KeywordConfig) -> Self {
        let keywords_lower: HashSet<String> = config
            .blocked_keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();

        Self {
            config,
            keywords_lower,
        }
    }

    /// Check for exact keyword matches
    fn find_exact_matches(&self, input: &str) -> Vec<(String, usize)> {
        let mut matches = Vec::new();
        let words: Vec<&str> = input.split_whitespace().collect();

        for (idx, word) in words.iter().enumerate() {
            let word_clean = word.trim_matches(|c: char| !c.is_alphanumeric());

            if self.config.case_sensitive {
                if self.config.blocked_keywords.contains(&word_clean.to_string()) {
                    matches.push((word_clean.to_string(), idx));
                }
            } else if self.keywords_lower.contains(&word_clean.to_lowercase()) {
                matches.push((word_clean.to_string(), idx));
            }
        }

        matches
    }

    /// Check for phrase matches
    fn find_phrase_matches(&self, input: &str) -> Vec<String> {
        let mut matches = Vec::new();
        let input_check = if self.config.case_sensitive {
            input.to_string()
        } else {
            input.to_lowercase()
        };

        for keyword in &self.config.blocked_keywords {
            let keyword_check = if self.config.case_sensitive {
                keyword.clone()
            } else {
                keyword.to_lowercase()
            };

            // Check if it's a phrase (contains spaces)
            if keyword.contains(' ') && input_check.contains(&keyword_check) {
                matches.push(keyword.clone());
            }
        }

        matches
    }

    /// Simple fuzzy match using edit distance
    fn fuzzy_match(&self, word: &str, keyword: &str, max_distance: usize) -> bool {
        if !self.config.use_fuzzy_match {
            return false;
        }

        let word = if self.config.case_sensitive {
            word.to_string()
        } else {
            word.to_lowercase()
        };

        let keyword = if self.config.case_sensitive {
            keyword.to_string()
        } else {
            keyword.to_lowercase()
        };

        // Simple length check first
        if (word.len() as i32 - keyword.len() as i32).unsigned_abs() as usize > max_distance {
            return false;
        }

        // Levenshtein distance calculation
        self.levenshtein_distance(&word, &keyword) <= max_distance
    }

    /// Calculate Levenshtein edit distance
    fn levenshtein_distance(&self, s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();

        let m = s1_chars.len();
        let n = s2_chars.len();

        if m == 0 {
            return n;
        }
        if n == 0 {
            return m;
        }

        let mut matrix = vec![vec![0usize; n + 1]; m + 1];

        #[allow(clippy::needless_range_loop)]
        for i in 0..=m {
            matrix[i][0] = i;
        }
        #[allow(clippy::needless_range_loop)]
        for j in 0..=n {
            matrix[0][j] = j;
        }

        for i in 1..=m {
            for j in 1..=n {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };

                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[m][n]
    }

    /// Find fuzzy matches
    fn find_fuzzy_matches(&self, input: &str) -> Vec<(String, String)> {
        if !self.config.use_fuzzy_match {
            return Vec::new();
        }

        let mut matches = Vec::new();
        let words: Vec<&str> = input.split_whitespace().collect();

        for word in words {
            let word_clean = word.trim_matches(|c: char| !c.is_alphanumeric());

            // Skip short words for fuzzy matching
            if word_clean.len() < 4 {
                continue;
            }

            for keyword in &self.config.blocked_keywords {
                // Skip phrases for fuzzy matching
                if keyword.contains(' ') {
                    continue;
                }

                // Max distance scales with keyword length
                let max_distance = match keyword.len() {
                    0..=4 => 1,
                    5..=8 => 2,
                    _ => 3,
                };

                if self.fuzzy_match(word_clean, keyword, max_distance) {
                    // Avoid duplicates if exact match already found
                    if word_clean.to_lowercase() != keyword.to_lowercase() {
                        matches.push((word_clean.to_string(), keyword.clone()));
                    }
                }
            }
        }

        matches
    }
}

#[allow(clippy::derivable_impls)]
impl Default for KeywordDetector {
    fn default() -> Self {
        Self {
            config: KeywordConfig::default(),
            keywords_lower: HashSet::new(),
        }
    }
}

#[async_trait]
impl Detector for KeywordDetector {
    fn name(&self) -> &'static str {
        "keyword"
    }

    fn description(&self) -> &'static str {
        "Detects custom keyword blocklists in text"
    }

    async fn detect(&self, input: &str, _context: &DetectionContext) -> Result<DetectionResult> {
        if !self.config.enabled {
            return Ok(DetectionResult::no_detection(
                self.name(),
                self.config.confidence_threshold,
            ));
        }

        // No keywords configured
        if self.config.blocked_keywords.is_empty() {
            return Ok(DetectionResult::no_detection(
                self.name(),
                self.config.confidence_threshold,
            ));
        }

        let exact_matches = self.find_exact_matches(input);
        let phrase_matches = self.find_phrase_matches(input);
        let fuzzy_matches = self.find_fuzzy_matches(input);

        let total_matches = exact_matches.len() + phrase_matches.len() + fuzzy_matches.len();

        if total_matches == 0 {
            return Ok(DetectionResult::no_detection(
                self.name(),
                self.config.confidence_threshold,
            ));
        }

        // Build evidence
        let mut evidence = Vec::new();

        for (word, _) in &exact_matches {
            evidence.push(format!("Exact match: \"{}\"", word));
        }
        for phrase in &phrase_matches {
            evidence.push(format!("Phrase match: \"{}\"", phrase));
        }
        for (word, keyword) in &fuzzy_matches {
            evidence.push(format!("Fuzzy match: \"{}\" (similar to \"{}\")", word, keyword));
        }

        // Calculate confidence
        // Exact and phrase matches have higher confidence
        let exact_confidence: f64 = if !exact_matches.is_empty() || !phrase_matches.is_empty() {
            0.95
        } else {
            0.0
        };

        let fuzzy_confidence: f64 = if !fuzzy_matches.is_empty() {
            0.75
        } else {
            0.0
        };

        let base_confidence = exact_confidence.max(fuzzy_confidence);
        let match_boost = ((total_matches - 1) as f64 * 0.02).min(0.04);

        let confidence =
            (base_confidence + match_boost) * self.config.sensitivity.multiplier();

        let detected = confidence >= self.config.confidence_threshold;

        // Severity based on number of matches
        let severity = match total_matches {
            1 => Severity::Low,
            2..=3 => Severity::Medium,
            _ => Severity::High,
        };

        // Build metadata
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("exact_matches".to_string(), serde_json::json!(exact_matches.len()));
        metadata.insert("phrase_matches".to_string(), serde_json::json!(phrase_matches.len()));
        metadata.insert("fuzzy_matches".to_string(), serde_json::json!(fuzzy_matches.len()));

        Ok(DetectionResult {
            detector_name: self.name().to_string(),
            detected,
            confidence: confidence.min(0.99),
            threshold: self.config.confidence_threshold,
            severity,
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

    fn config_with_keywords(keywords: Vec<&str>) -> KeywordConfig {
        KeywordConfig {
            enabled: true,
            sensitivity: crate::guardrails::config::Sensitivity::Medium,
            confidence_threshold: 0.7,
            blocked_keywords: keywords.into_iter().map(String::from).collect(),
            case_sensitive: false,
            use_fuzzy_match: false,
        }
    }

    #[tokio::test]
    async fn test_exact_match() {
        let config = config_with_keywords(vec!["forbidden", "blocked"]);
        let detector = KeywordDetector::with_config(config);
        let context = DetectionContext::default();

        let result = detector
            .detect("This text contains a forbidden word", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert!(result.evidence.iter().any(|e| e.contains("forbidden")));
    }

    #[tokio::test]
    async fn test_case_insensitive() {
        let config = config_with_keywords(vec!["forbidden"]);
        let detector = KeywordDetector::with_config(config);
        let context = DetectionContext::default();

        let inputs = vec!["FORBIDDEN", "Forbidden", "forbidden", "FoRbIdDeN"];

        for input in inputs {
            let result = detector.detect(input, &context).await.unwrap();
            assert!(result.detected, "Should match case-insensitively: {}", input);
        }
    }

    #[tokio::test]
    async fn test_case_sensitive() {
        let mut config = config_with_keywords(vec!["Forbidden"]);
        config.case_sensitive = true;

        let detector = KeywordDetector::with_config(config);
        let context = DetectionContext::default();

        // Should match
        let result1 = detector.detect("Forbidden word", &context).await.unwrap();
        assert!(result1.detected);

        // Should not match
        let result2 = detector.detect("forbidden word", &context).await.unwrap();
        assert!(!result2.detected);
    }

    #[tokio::test]
    async fn test_phrase_match() {
        let config = config_with_keywords(vec!["banned phrase"]);
        let detector = KeywordDetector::with_config(config);
        let context = DetectionContext::default();

        let result = detector
            .detect("This contains a banned phrase in the text", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert!(result.evidence.iter().any(|e| e.contains("Phrase match")));
    }

    #[tokio::test]
    async fn test_fuzzy_match() {
        let mut config = config_with_keywords(vec!["forbidden"]);
        config.use_fuzzy_match = true;

        let detector = KeywordDetector::with_config(config);
        let context = DetectionContext::default();

        // Typo: "forbidn" -> should fuzzy match "forbidden"
        let result = detector
            .detect("This has a forbidn typo", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert!(result.evidence.iter().any(|e| e.contains("Fuzzy match")));
    }

    #[tokio::test]
    async fn test_no_keywords_configured() {
        let config = config_with_keywords(vec![]);
        let detector = KeywordDetector::with_config(config);
        let context = DetectionContext::default();

        let result = detector
            .detect("Any text here", &context)
            .await
            .unwrap();

        assert!(!result.detected);
    }

    #[tokio::test]
    async fn test_no_match() {
        let config = config_with_keywords(vec!["forbidden", "blocked"]);
        let detector = KeywordDetector::with_config(config);
        let context = DetectionContext::default();

        let result = detector
            .detect("This text is completely safe and normal", &context)
            .await
            .unwrap();

        assert!(!result.detected);
    }

    #[tokio::test]
    async fn test_multiple_matches() {
        let config = config_with_keywords(vec!["bad", "evil", "wrong"]);
        let detector = KeywordDetector::with_config(config);
        let context = DetectionContext::default();

        let result = detector
            .detect("This is bad and evil and wrong", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert!(result.evidence.len() >= 3);
        // 3 matches = Medium severity (2..=3 => Medium, >3 => High)
        assert_eq!(result.severity, Severity::Medium);
    }

    #[tokio::test]
    async fn test_levenshtein_distance() {
        let config = config_with_keywords(vec![]);
        let detector = KeywordDetector::with_config(config);

        assert_eq!(detector.levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(detector.levenshtein_distance("hello", "hello"), 0);
        assert_eq!(detector.levenshtein_distance("", "abc"), 3);
        assert_eq!(detector.levenshtein_distance("abc", ""), 3);
    }
}
