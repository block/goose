//! Secret Detector
//!
//! Detects API keys, tokens, passwords, and other credentials in text.
//! Based on patterns from tools like TruffleHog, GitLeaks, and ZenGuard.

use super::{DetectionContext, DetectionResult, Detector};
use crate::guardrails::config::DetectorConfig;
use crate::guardrails::Severity;
use anyhow::Result;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Types of secrets that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecretType {
    AwsAccessKey,
    AwsSecretKey,
    GithubToken,
    GitlabToken,
    GoogleApiKey,
    OpenAiApiKey,
    AnthropicApiKey,
    SlackToken,
    StripeKey,
    TwilioKey,
    PrivateKey,
    JwtToken,
    BasicAuthCredentials,
    GenericApiKey,
    GenericPassword,
    DatabaseUrl,
    SshKey,
    NpmToken,
    PypiToken,
    HerokuApiKey,
    SendgridKey,
    MailgunKey,
    AzureKey,
}

impl SecretType {
    /// Get severity for this secret type
    pub fn severity(&self) -> Severity {
        match self {
            SecretType::AwsSecretKey
            | SecretType::PrivateKey
            | SecretType::SshKey
            | SecretType::DatabaseUrl => Severity::Critical,

            SecretType::AwsAccessKey
            | SecretType::GithubToken
            | SecretType::GitlabToken
            | SecretType::StripeKey
            | SecretType::OpenAiApiKey
            | SecretType::AnthropicApiKey
            | SecretType::JwtToken => Severity::High,

            SecretType::GoogleApiKey
            | SecretType::SlackToken
            | SecretType::TwilioKey
            | SecretType::GenericApiKey
            | SecretType::BasicAuthCredentials
            | SecretType::NpmToken
            | SecretType::PypiToken
            | SecretType::HerokuApiKey
            | SecretType::SendgridKey
            | SecretType::MailgunKey
            | SecretType::AzureKey => Severity::Medium,

            SecretType::GenericPassword => Severity::Low,
        }
    }

    /// Get display name
    pub fn as_str(&self) -> &'static str {
        match self {
            SecretType::AwsAccessKey => "AWS Access Key",
            SecretType::AwsSecretKey => "AWS Secret Key",
            SecretType::GithubToken => "GitHub Token",
            SecretType::GitlabToken => "GitLab Token",
            SecretType::GoogleApiKey => "Google API Key",
            SecretType::OpenAiApiKey => "OpenAI API Key",
            SecretType::AnthropicApiKey => "Anthropic API Key",
            SecretType::SlackToken => "Slack Token",
            SecretType::StripeKey => "Stripe Key",
            SecretType::TwilioKey => "Twilio Key",
            SecretType::PrivateKey => "Private Key",
            SecretType::JwtToken => "JWT Token",
            SecretType::BasicAuthCredentials => "Basic Auth Credentials",
            SecretType::GenericApiKey => "Generic API Key",
            SecretType::GenericPassword => "Generic Password",
            SecretType::DatabaseUrl => "Database URL",
            SecretType::SshKey => "SSH Key",
            SecretType::NpmToken => "NPM Token",
            SecretType::PypiToken => "PyPI Token",
            SecretType::HerokuApiKey => "Heroku API Key",
            SecretType::SendgridKey => "SendGrid Key",
            SecretType::MailgunKey => "Mailgun Key",
            SecretType::AzureKey => "Azure Key",
        }
    }
}

/// Secret pattern definition
struct SecretPattern {
    secret_type: SecretType,
    regex: Regex,
    description: &'static str,
}

/// All secret detection patterns
static SECRET_PATTERNS: Lazy<Vec<SecretPattern>> = Lazy::new(|| {
    vec![
        // AWS Keys
        SecretPattern {
            secret_type: SecretType::AwsAccessKey,
            regex: Regex::new(r"(?i)(AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}").unwrap(),
            description: "AWS Access Key ID",
        },
        SecretPattern {
            secret_type: SecretType::AwsSecretKey,
            regex: Regex::new(r#"(?i)aws[_\-]?secret[_\-]?(?:access)?[_\-]?key['"]?\s*[=:]\s*['"]?([A-Za-z0-9/+=]{40})"#).unwrap(),
            description: "AWS Secret Access Key",
        },

        // GitHub Tokens
        SecretPattern {
            secret_type: SecretType::GithubToken,
            regex: Regex::new(r"(?i)ghp_[A-Za-z0-9]{36}").unwrap(),
            description: "GitHub Personal Access Token",
        },
        SecretPattern {
            secret_type: SecretType::GithubToken,
            regex: Regex::new(r"(?i)gho_[A-Za-z0-9]{36}").unwrap(),
            description: "GitHub OAuth Token",
        },
        SecretPattern {
            secret_type: SecretType::GithubToken,
            regex: Regex::new(r"(?i)ghu_[A-Za-z0-9]{36}").unwrap(),
            description: "GitHub User Token",
        },
        SecretPattern {
            secret_type: SecretType::GithubToken,
            regex: Regex::new(r"(?i)ghs_[A-Za-z0-9]{36}").unwrap(),
            description: "GitHub Server Token",
        },
        SecretPattern {
            secret_type: SecretType::GithubToken,
            regex: Regex::new(r"(?i)ghr_[A-Za-z0-9]{36}").unwrap(),
            description: "GitHub Refresh Token",
        },

        // GitLab Token
        SecretPattern {
            secret_type: SecretType::GitlabToken,
            regex: Regex::new(r"(?i)glpat-[A-Za-z0-9\-_]{20,}").unwrap(),
            description: "GitLab Personal Access Token",
        },

        // Google API Key
        SecretPattern {
            secret_type: SecretType::GoogleApiKey,
            regex: Regex::new(r"AIza[0-9A-Za-z\-_]{35}").unwrap(),
            description: "Google API Key",
        },

        // OpenAI API Key
        SecretPattern {
            secret_type: SecretType::OpenAiApiKey,
            regex: Regex::new(r"sk-[A-Za-z0-9]{20}T3BlbkFJ[A-Za-z0-9]{20}").unwrap(),
            description: "OpenAI API Key (legacy)",
        },
        SecretPattern {
            secret_type: SecretType::OpenAiApiKey,
            regex: Regex::new(r"sk-proj-[A-Za-z0-9\-_]{40,}").unwrap(),
            description: "OpenAI Project API Key",
        },
        SecretPattern {
            secret_type: SecretType::OpenAiApiKey,
            regex: Regex::new(r"sk-[a-zA-Z0-9]{48,}").unwrap(),
            description: "OpenAI API Key",
        },

        // Anthropic API Key
        SecretPattern {
            secret_type: SecretType::AnthropicApiKey,
            regex: Regex::new(r"sk-ant-[A-Za-z0-9\-_]{40,}").unwrap(),
            description: "Anthropic API Key",
        },

        // Slack Tokens
        SecretPattern {
            secret_type: SecretType::SlackToken,
            regex: Regex::new(r"xox[baprs]-[0-9]{10,13}-[0-9]{10,13}[a-zA-Z0-9-]*").unwrap(),
            description: "Slack Token",
        },

        // Stripe Keys
        SecretPattern {
            secret_type: SecretType::StripeKey,
            regex: Regex::new(r"sk_live_[0-9a-zA-Z]{24,}").unwrap(),
            description: "Stripe Live Secret Key",
        },
        SecretPattern {
            secret_type: SecretType::StripeKey,
            regex: Regex::new(r"sk_test_[0-9a-zA-Z]{24,}").unwrap(),
            description: "Stripe Test Secret Key",
        },
        SecretPattern {
            secret_type: SecretType::StripeKey,
            regex: Regex::new(r"pk_live_[0-9a-zA-Z]{24,}").unwrap(),
            description: "Stripe Live Publishable Key",
        },
        SecretPattern {
            secret_type: SecretType::StripeKey,
            regex: Regex::new(r"rk_live_[0-9a-zA-Z]{24,}").unwrap(),
            description: "Stripe Live Restricted Key",
        },

        // Twilio
        SecretPattern {
            secret_type: SecretType::TwilioKey,
            regex: Regex::new(r"SK[0-9a-fA-F]{32}").unwrap(),
            description: "Twilio API Key",
        },

        // Private Keys (RSA, EC, etc.)
        SecretPattern {
            secret_type: SecretType::PrivateKey,
            regex: Regex::new(r"-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap(),
            description: "Private Key Header",
        },

        // SSH Keys
        SecretPattern {
            secret_type: SecretType::SshKey,
            regex: Regex::new(r"-----BEGIN OPENSSH PRIVATE KEY-----").unwrap(),
            description: "OpenSSH Private Key",
        },

        // JWT Tokens
        SecretPattern {
            secret_type: SecretType::JwtToken,
            regex: Regex::new(r"eyJ[A-Za-z0-9\-_]+\.eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+").unwrap(),
            description: "JWT Token",
        },

        // Basic Auth in URLs
        SecretPattern {
            secret_type: SecretType::BasicAuthCredentials,
            regex: Regex::new(r"https?://[^:\s]+:[^@\s]+@[^\s]+").unwrap(),
            description: "Basic Auth in URL",
        },

        // Database URLs
        SecretPattern {
            secret_type: SecretType::DatabaseUrl,
            regex: Regex::new(r"(?i)(postgres|mysql|mongodb|redis)://[^:\s]+:[^@\s]+@[^\s]+").unwrap(),
            description: "Database Connection URL",
        },

        // NPM Token
        SecretPattern {
            secret_type: SecretType::NpmToken,
            regex: Regex::new(r"npm_[A-Za-z0-9]{36}").unwrap(),
            description: "NPM Access Token",
        },

        // PyPI Token
        SecretPattern {
            secret_type: SecretType::PypiToken,
            regex: Regex::new(r"pypi-[A-Za-z0-9_\-]{50,}").unwrap(),
            description: "PyPI API Token",
        },

        // Heroku API Key
        SecretPattern {
            secret_type: SecretType::HerokuApiKey,
            regex: Regex::new(r#"(?i)heroku[_\-]?api[_\-]?key['"]?\s*[=:]\s*['"]?([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})"#).unwrap(),
            description: "Heroku API Key",
        },

        // SendGrid
        SecretPattern {
            secret_type: SecretType::SendgridKey,
            regex: Regex::new(r"SG\.[A-Za-z0-9\-_]{22}\.[A-Za-z0-9\-_]{43}").unwrap(),
            description: "SendGrid API Key",
        },

        // Mailgun
        SecretPattern {
            secret_type: SecretType::MailgunKey,
            regex: Regex::new(r"key-[0-9a-fA-F]{32}").unwrap(),
            description: "Mailgun API Key",
        },

        // Azure
        SecretPattern {
            secret_type: SecretType::AzureKey,
            regex: Regex::new(r#"(?i)azure[_\-]?(?:storage)?[_\-]?(?:account)?[_\-]?key['"]?\s*[=:]\s*['"]?([A-Za-z0-9+/]{86}==)"#).unwrap(),
            description: "Azure Storage Key",
        },

        // Generic API Key pattern (more generic, lower priority)
        SecretPattern {
            secret_type: SecretType::GenericApiKey,
            regex: Regex::new(r#"(?i)(?:api[_\-]?key|apikey|api_secret|apisecret)['"]?\s*[=:]\s*['"]?([A-Za-z0-9\-_]{20,})"#).unwrap(),
            description: "Generic API Key",
        },

        // Generic password pattern
        SecretPattern {
            secret_type: SecretType::GenericPassword,
            regex: Regex::new(r#"(?i)(?:password|passwd|pwd|secret)['"]?\s*[=:]\s*['"]?([^\s'"]{8,})"#).unwrap(),
            description: "Generic Password",
        },
    ]
});

/// Secret detector
#[derive(Default)]
pub struct SecretDetector {
    config: DetectorConfig,
}

impl SecretDetector {
    /// Create with custom configuration
    pub fn with_config(config: DetectorConfig) -> Self {
        Self { config }
    }

    /// Redact a secret value
    #[allow(clippy::string_slice)]
    fn redact_secret(&self, value: &str) -> String {
        let len = value.len();
        if len <= 8 {
            "*".repeat(len)
        } else {
            format!("{}...{}", &value[..4], &value[len - 4..])
        }
    }

    /// Calculate entropy of a string (for generic secret detection)
    #[allow(dead_code)]
    fn calculate_entropy(&self, s: &str) -> f64 {
        use std::collections::HashMap;

        let mut freq: HashMap<char, usize> = HashMap::new();
        for c in s.chars() {
            *freq.entry(c).or_insert(0) += 1;
        }

        let len = s.len() as f64;
        freq.values()
            .map(|&count| {
                let p = count as f64 / len;
                -p * p.log2()
            })
            .sum()
    }
}


#[async_trait]
impl Detector for SecretDetector {
    fn name(&self) -> &'static str {
        "secret"
    }

    fn description(&self) -> &'static str {
        "Detects API keys, tokens, passwords, and other credentials"
    }

    async fn detect(&self, input: &str, _context: &DetectionContext) -> Result<DetectionResult> {
        if !self.config.enabled {
            return Ok(DetectionResult::no_detection(
                self.name(),
                self.config.confidence_threshold,
            ));
        }

        let mut detected_secrets: Vec<(SecretType, String, &str)> = Vec::new();
        let mut max_severity = Severity::Low;

        for pattern in SECRET_PATTERNS.iter() {
            for capture in pattern.regex.find_iter(input) {
                let value = capture.as_str();
                detected_secrets.push((pattern.secret_type, value.to_string(), pattern.description));

                if pattern.secret_type.severity() > max_severity {
                    max_severity = pattern.secret_type.severity();
                }
            }
        }

        if detected_secrets.is_empty() {
            return Ok(DetectionResult::no_detection(
                self.name(),
                self.config.confidence_threshold,
            ));
        }

        // Build evidence with redacted values
        let evidence: Vec<String> = detected_secrets
            .iter()
            .map(|(secret_type, value, desc)| {
                format!(
                    "{} ({}): {}",
                    desc,
                    secret_type.as_str(),
                    self.redact_secret(value)
                )
            })
            .collect();

        // High confidence for regex-based detection
        let confidence = 0.95 * self.config.sensitivity.multiplier();

        // Build metadata
        let mut metadata = std::collections::HashMap::new();
        let secret_types: Vec<String> = detected_secrets
            .iter()
            .map(|(t, _, _)| t.as_str().to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        metadata.insert("secret_types_found".to_string(), serde_json::json!(secret_types));
        metadata.insert("count".to_string(), serde_json::json!(detected_secrets.len()));

        let detected = confidence >= self.config.confidence_threshold;

        Ok(DetectionResult {
            detector_name: self.name().to_string(),
            detected,
            confidence: confidence.min(0.99),
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
    async fn test_aws_access_key() {
        let detector = SecretDetector::default();
        let context = DetectionContext::default();

        let result = detector
            .detect("My AWS key is AKIAIOSFODNN7EXAMPLE", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert!(result.evidence.iter().any(|e| e.contains("AWS")));
    }

    #[tokio::test]
    async fn test_github_token() {
        let detector = SecretDetector::default();
        let context = DetectionContext::default();

        let tokens = vec![
            "ghp_aBcDeFgHiJkLmNoPqRsTuVwXyZ1234567890",
            "gho_aBcDeFgHiJkLmNoPqRsTuVwXyZ1234567890",
        ];

        for token in tokens {
            let result = detector.detect(token, &context).await.unwrap();
            assert!(result.detected, "Should detect GitHub token: {}", token);
        }
    }

    #[tokio::test]
    async fn test_openai_key() {
        let detector = SecretDetector::default();
        let context = DetectionContext::default();

        let result = detector
            .detect("sk-proj-abcdefghijklmnopqrstuvwxyz123456789012345678", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert!(result.evidence.iter().any(|e| e.contains("OpenAI")));
    }

    #[tokio::test]
    async fn test_anthropic_key() {
        let detector = SecretDetector::default();
        let context = DetectionContext::default();

        let result = detector
            .detect("sk-ant-abcdefghijklmnopqrstuvwxyz123456789012345678", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert!(result.evidence.iter().any(|e| e.contains("Anthropic")));
    }

    #[tokio::test]
    async fn test_stripe_key() {
        let detector = SecretDetector::default();
        let context = DetectionContext::default();

        let keys = vec![
            "sk_live_abcdefghijklmnopqrstuvwxyz",
            "sk_test_abcdefghijklmnopqrstuvwxyz",
        ];

        for key in keys {
            let result = detector.detect(key, &context).await.unwrap();
            assert!(result.detected, "Should detect Stripe key: {}", key);
        }
    }

    #[tokio::test]
    async fn test_private_key() {
        let detector = SecretDetector::default();
        let context = DetectionContext::default();

        let result = detector
            .detect("-----BEGIN RSA PRIVATE KEY-----\nMIIE...", &context)
            .await
            .unwrap();

        assert!(result.detected);
        assert_eq!(result.severity, Severity::Critical);
    }

    #[tokio::test]
    async fn test_jwt_token() {
        let detector = SecretDetector::default();
        let context = DetectionContext::default();

        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

        let result = detector.detect(jwt, &context).await.unwrap();

        assert!(result.detected);
        assert!(result.evidence.iter().any(|e| e.contains("JWT")));
    }

    #[tokio::test]
    async fn test_database_url() {
        let detector = SecretDetector::default();
        let context = DetectionContext::default();

        let urls = vec![
            "postgres://user:password@localhost:5432/db",
            "mysql://admin:secret@host.com/database",
            "mongodb://user:pass@cluster.mongodb.net/app",
        ];

        for url in urls {
            let result = detector.detect(url, &context).await.unwrap();
            assert!(result.detected, "Should detect database URL: {}", url);
        }
    }

    #[tokio::test]
    async fn test_generic_api_key() {
        let detector = SecretDetector::default();
        let context = DetectionContext::default();

        let result = detector
            .detect("api_key = 'abcdefghijklmnopqrstuvwxyz123456'", &context)
            .await
            .unwrap();

        assert!(result.detected);
    }

    #[tokio::test]
    async fn test_no_secrets() {
        let detector = SecretDetector::default();
        let context = DetectionContext::default();

        let result = detector
            .detect("This is just regular text without any secrets", &context)
            .await
            .unwrap();

        assert!(!result.detected);
        assert!(result.evidence.is_empty());
    }

    #[tokio::test]
    async fn test_redaction() {
        let detector = SecretDetector::default();

        assert_eq!(detector.redact_secret("short"), "*****");
        assert_eq!(detector.redact_secret("1234567890"), "1234...7890");
        assert_eq!(
            detector.redact_secret("abcdefghijklmnopqrstuvwxyz"),
            "abcd...wxyz"
        );
    }

    #[tokio::test]
    async fn test_multiple_secrets() {
        let detector = SecretDetector::default();
        let context = DetectionContext::default();

        let input = "AWS key: AKIAIOSFODNN7EXAMPLE, GitHub: ghp_aBcDeFgHiJkLmNoPqRsTuVwXyZ1234567890";
        let result = detector.detect(input, &context).await.unwrap();

        assert!(result.detected);
        assert!(result.evidence.len() >= 2);
    }
}
