//! PII (Personally Identifiable Information) Detector
//!
//! Detects various types of PII in text including emails, phone numbers,
//! SSNs, credit cards, and more.

use super::{DetectionContext, DetectionResult, Detector};
use crate::guardrails::config::PiiConfig;
use crate::guardrails::Severity;
use anyhow::Result;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Types of PII that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PiiType {
    Email,
    PhoneNumber,
    SSN,
    CreditCard,
    IpAddress,
    DateOfBirth,
    Address,
    DriversLicense,
    Passport,
    BankAccount,
    HealthId,
}

impl PiiType {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            PiiType::Email => "email",
            PiiType::PhoneNumber => "phone_number",
            PiiType::SSN => "ssn",
            PiiType::CreditCard => "credit_card",
            PiiType::IpAddress => "ip_address",
            PiiType::DateOfBirth => "date_of_birth",
            PiiType::Address => "address",
            PiiType::DriversLicense => "drivers_license",
            PiiType::Passport => "passport",
            PiiType::BankAccount => "bank_account",
            PiiType::HealthId => "health_id",
        }
    }

    /// Get severity for this PII type
    pub fn severity(&self) -> Severity {
        match self {
            PiiType::SSN | PiiType::CreditCard | PiiType::BankAccount => Severity::Critical,
            PiiType::Passport | PiiType::DriversLicense | PiiType::HealthId => Severity::High,
            PiiType::PhoneNumber | PiiType::DateOfBirth | PiiType::Address => Severity::Medium,
            PiiType::Email | PiiType::IpAddress => Severity::Low,
        }
    }
}

/// PII pattern with its regex
struct PiiPattern {
    pii_type: PiiType,
    regex: Regex,
    description: &'static str,
}

/// All PII detection patterns
static PII_PATTERNS: Lazy<Vec<PiiPattern>> = Lazy::new(|| {
    vec![
        // Email addresses
        PiiPattern {
            pii_type: PiiType::Email,
            regex: Regex::new(
                r"(?i)[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}"
            ).unwrap(),
            description: "Email address",
        },
        // Phone numbers (US/International formats)
        PiiPattern {
            pii_type: PiiType::PhoneNumber,
            regex: Regex::new(
                r"(?:\+?1[-.\s]?)?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}"
            ).unwrap(),
            description: "Phone number",
        },
        // International phone numbers
        PiiPattern {
            pii_type: PiiType::PhoneNumber,
            regex: Regex::new(
                r"\+[1-9]\d{1,14}"
            ).unwrap(),
            description: "International phone number",
        },
        // Social Security Numbers (US)
        PiiPattern {
            pii_type: PiiType::SSN,
            regex: Regex::new(
                r"\b\d{3}[-.\s]?\d{2}[-.\s]?\d{4}\b"
            ).unwrap(),
            description: "Social Security Number",
        },
        // Credit Card Numbers (Visa, MasterCard, Amex, Discover)
        PiiPattern {
            pii_type: PiiType::CreditCard,
            regex: Regex::new(
                r"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12})\b"
            ).unwrap(),
            description: "Credit card number",
        },
        // Credit card with separators
        PiiPattern {
            pii_type: PiiType::CreditCard,
            regex: Regex::new(
                r"\b(?:4[0-9]{3}|5[1-5][0-9]{2}|3[47][0-9]{2}|6(?:011|5[0-9]{2}))[-\s]?[0-9]{4}[-\s]?[0-9]{4}[-\s]?[0-9]{4}\b"
            ).unwrap(),
            description: "Credit card number (with separators)",
        },
        // IPv4 Addresses
        PiiPattern {
            pii_type: PiiType::IpAddress,
            regex: Regex::new(
                r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b"
            ).unwrap(),
            description: "IPv4 address",
        },
        // IPv6 Addresses (simplified)
        PiiPattern {
            pii_type: PiiType::IpAddress,
            regex: Regex::new(
                r"(?i)\b(?:[0-9a-f]{1,4}:){7}[0-9a-f]{1,4}\b"
            ).unwrap(),
            description: "IPv6 address",
        },
        // Dates of Birth (various formats)
        PiiPattern {
            pii_type: PiiType::DateOfBirth,
            regex: Regex::new(
                r"\b(?:0[1-9]|1[0-2])[/\-](?:0[1-9]|[12][0-9]|3[01])[/\-](?:19|20)\d{2}\b"
            ).unwrap(),
            description: "Date (MM/DD/YYYY)",
        },
        PiiPattern {
            pii_type: PiiType::DateOfBirth,
            regex: Regex::new(
                r"\b(?:19|20)\d{2}[/\-](?:0[1-9]|1[0-2])[/\-](?:0[1-9]|[12][0-9]|3[01])\b"
            ).unwrap(),
            description: "Date (YYYY/MM/DD)",
        },
        // US Driver's License (simplified pattern for common formats)
        PiiPattern {
            pii_type: PiiType::DriversLicense,
            regex: Regex::new(
                r"(?i)\b(?:DL|D\.L\.|driver'?s?\s*license)[:\s#]*[A-Z0-9]{5,15}\b"
            ).unwrap(),
            description: "Driver's license number",
        },
        // Passport numbers (simplified)
        PiiPattern {
            pii_type: PiiType::Passport,
            regex: Regex::new(
                r"(?i)\bpassport[:\s#]*[A-Z0-9]{6,12}\b"
            ).unwrap(),
            description: "Passport number",
        },
        // Bank account numbers (various formats)
        PiiPattern {
            pii_type: PiiType::BankAccount,
            regex: Regex::new(
                r"(?i)\b(?:account|acct)[:\s#]*\d{8,17}\b"
            ).unwrap(),
            description: "Bank account number",
        },
        // Routing numbers (US)
        PiiPattern {
            pii_type: PiiType::BankAccount,
            regex: Regex::new(
                r"(?i)\b(?:routing|aba)[:\s#]*\d{9}\b"
            ).unwrap(),
            description: "Bank routing number",
        },
        // IBAN
        PiiPattern {
            pii_type: PiiType::BankAccount,
            regex: Regex::new(
                r"(?i)\b[A-Z]{2}\d{2}[A-Z0-9]{4}\d{7}(?:[A-Z0-9]?){0,16}\b"
            ).unwrap(),
            description: "IBAN",
        },
        // Health Insurance ID (Medicare, etc.)
        PiiPattern {
            pii_type: PiiType::HealthId,
            regex: Regex::new(
                r"(?i)\b(?:medicare|medicaid|health\s*id)[:\s#]*[A-Z0-9]{10,15}\b"
            ).unwrap(),
            description: "Health insurance ID",
        },
    ]
});

/// PII detector
pub struct PiiDetector {
    config: PiiConfig,
    allowed_types: HashSet<PiiType>,
}

impl PiiDetector {
    /// Create with custom configuration
    pub fn with_config(config: PiiConfig) -> Self {
        let allowed_types: HashSet<PiiType> = config
            .allowed_types
            .iter()
            .filter_map(|s| match s.as_str() {
                "email" => Some(PiiType::Email),
                "phone_number" => Some(PiiType::PhoneNumber),
                "ssn" => Some(PiiType::SSN),
                "credit_card" => Some(PiiType::CreditCard),
                "ip_address" => Some(PiiType::IpAddress),
                "date_of_birth" => Some(PiiType::DateOfBirth),
                "address" => Some(PiiType::Address),
                "drivers_license" => Some(PiiType::DriversLicense),
                "passport" => Some(PiiType::Passport),
                "bank_account" => Some(PiiType::BankAccount),
                "health_id" => Some(PiiType::HealthId),
                _ => None,
            })
            .collect();

        Self {
            config,
            allowed_types,
        }
    }

    /// Redact a value for safe logging
    #[allow(clippy::string_slice)]
    fn redact_value(&self, value: &str) -> String {
        if !self.config.redact_on_detect {
            return value.to_string();
        }

        let len = value.len();
        if len <= 4 {
            "*".repeat(len)
        } else if len <= 8 {
            format!("{}***", &value[..2])
        } else {
            format!("{}***{}", &value[..2], &value[len - 2..])
        }
    }

    /// Validate credit card using Luhn algorithm
    fn validate_luhn(&self, number: &str) -> bool {
        let digits: Vec<u32> = number
            .chars()
            .filter(|c| c.is_ascii_digit())
            .filter_map(|c| c.to_digit(10))
            .collect();

        if digits.len() < 13 {
            return false;
        }

        let sum: u32 = digits
            .iter()
            .rev()
            .enumerate()
            .map(|(i, &d)| {
                if i % 2 == 1 {
                    let doubled = d * 2;
                    if doubled > 9 {
                        doubled - 9
                    } else {
                        doubled
                    }
                } else {
                    d
                }
            })
            .sum();

        sum.is_multiple_of(10)
    }
}

#[allow(clippy::derivable_impls)]
impl Default for PiiDetector {
    fn default() -> Self {
        Self {
            config: PiiConfig::default(),
            allowed_types: HashSet::new(),
        }
    }
}

#[async_trait]
impl Detector for PiiDetector {
    fn name(&self) -> &'static str {
        "pii"
    }

    fn description(&self) -> &'static str {
        "Detects personally identifiable information (PII) in text"
    }

    async fn detect(&self, input: &str, _context: &DetectionContext) -> Result<DetectionResult> {
        if !self.config.enabled {
            return Ok(DetectionResult::no_detection(
                self.name(),
                self.config.confidence_threshold,
            ));
        }

        let mut detected_pii: Vec<(PiiType, String, &str)> = Vec::new();
        let mut max_severity = Severity::Low;

        for pattern in PII_PATTERNS.iter() {
            // Skip allowed types
            if self.allowed_types.contains(&pattern.pii_type) {
                continue;
            }

            for capture in pattern.regex.find_iter(input) {
                let value = capture.as_str();

                // Additional validation for credit cards
                if pattern.pii_type == PiiType::CreditCard && !self.validate_luhn(value) {
                    continue; // Skip invalid credit card numbers
                }

                detected_pii.push((pattern.pii_type, value.to_string(), pattern.description));
                if pattern.pii_type.severity() > max_severity {
                    max_severity = pattern.pii_type.severity();
                }
            }
        }

        if detected_pii.is_empty() {
            return Ok(DetectionResult::no_detection(
                self.name(),
                self.config.confidence_threshold,
            ));
        }

        // Build evidence with redacted values
        let evidence: Vec<String> = detected_pii
            .iter()
            .map(|(pii_type, value, desc)| {
                format!("{} ({}): {}", desc, pii_type.as_str(), self.redact_value(value))
            })
            .collect();

        // Confidence is high for PII detection (regex-based)
        let confidence = 0.95 * self.config.sensitivity.multiplier();

        // Build metadata
        let mut metadata = std::collections::HashMap::new();
        let pii_types: Vec<String> = detected_pii
            .iter()
            .map(|(t, _, _)| t.as_str().to_string())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        metadata.insert(
            "pii_types_found".to_string(),
            serde_json::json!(pii_types),
        );
        metadata.insert("count".to_string(), serde_json::json!(detected_pii.len()));

        let detected = confidence >= self.config.confidence_threshold;

        Ok(DetectionResult {
            detector_name: self.name().to_string(),
            detected,
            confidence,
            threshold: self.config.confidence_threshold,
            severity: max_severity,
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

    #[tokio::test]
    async fn test_email_detection() {
        let detector = PiiDetector::default();
        let context = DetectionContext::default();

        let result = detector
            .detect("My email is john.doe@example.com, please contact me", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert!(result.evidence.iter().any(|e| e.contains("Email")));
    }

    #[tokio::test]
    async fn test_ssn_detection() {
        let detector = PiiDetector::default();
        let context = DetectionContext::default();

        let inputs = vec![
            "My SSN is 123-45-6789",
            "SSN: 123.45.6789",
            "Social Security Number 123 45 6789",
        ];

        for input in inputs {
            let result = detector.detect(input, &context).await.unwrap();
            assert!(result.detected, "Should detect SSN in: {}", input);
            assert_eq!(result.severity, Severity::Critical);
        }
    }

    #[tokio::test]
    async fn test_credit_card_detection() {
        let detector = PiiDetector::default();
        let context = DetectionContext::default();

        // Valid Visa test number (passes Luhn)
        let result = detector
            .detect("Charge to card 4111111111111111", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert!(result.evidence.iter().any(|e| e.contains("Credit card")));
    }

    #[tokio::test]
    async fn test_invalid_credit_card_rejected() {
        let detector = PiiDetector::default();
        let context = DetectionContext::default();

        // Invalid credit card (fails Luhn)
        let result = detector
            .detect("Card number 1234567890123456", &context)
            .await
            .unwrap();

        // Should not detect as credit card
        let has_cc = result.evidence.iter().any(|e| e.contains("Credit card"));
        assert!(!has_cc, "Should reject invalid credit card");
    }

    #[tokio::test]
    async fn test_phone_detection() {
        let detector = PiiDetector::default();
        let context = DetectionContext::default();

        let inputs = vec![
            "Call me at 555-123-4567",
            "Phone: (555) 123-4567",
            "My number is +1 555 123 4567",
        ];

        for input in inputs {
            let result = detector.detect(input, &context).await.unwrap();
            assert!(result.detected, "Should detect phone in: {}", input);
        }
    }

    #[tokio::test]
    async fn test_ip_address_detection() {
        let detector = PiiDetector::default();
        let context = DetectionContext::default();

        let result = detector
            .detect("Server IP is 192.168.1.100", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert!(result.evidence.iter().any(|e| e.contains("IP")));
    }

    #[tokio::test]
    async fn test_allowed_types() {
        let mut config = PiiConfig::default();
        config.allowed_types.insert("email".to_string());

        let detector = PiiDetector::with_config(config);
        let context = DetectionContext::default();

        // Email should not trigger detection
        let result = detector
            .detect("Contact me at test@example.com", &context)
            .await
            .unwrap();

        let has_email = result.evidence.iter().any(|e| e.contains("Email"));
        assert!(!has_email, "Email should be allowed");
    }

    #[tokio::test]
    async fn test_no_pii() {
        let detector = PiiDetector::default();
        let context = DetectionContext::default();

        let result = detector
            .detect("This is just normal text without any PII", &context)
            .await
            .unwrap();

        assert!(!result.detected);
        assert!(result.evidence.is_empty());
    }

    #[tokio::test]
    async fn test_redaction() {
        let detector = PiiDetector::default();

        assert_eq!(detector.redact_value("test"), "****");
        assert_eq!(detector.redact_value("test@example.com"), "te***om");
        assert_eq!(detector.redact_value("123-45-6789"), "12***89");
    }

    #[tokio::test]
    async fn test_multiple_pii_types() {
        let detector = PiiDetector::default();
        let context = DetectionContext::default();

        let result = detector
            .detect(
                "Email: john@example.com, SSN: 123-45-6789, Phone: 555-123-4567",
                &context,
            )
            .await
            .unwrap();

        assert!(result.detected);
        assert!(result.evidence.len() >= 3);
    }
}
