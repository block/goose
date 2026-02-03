//! Security Validator - secret scanning, dangerous patterns

use super::{
    IssueSeverity, ValidationContext, ValidationDetails, ValidationIssue, ValidationResult,
    Validator,
};
use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use std::path::PathBuf;

pub struct SecurityValidator {
    secret_patterns: Vec<SecretPattern>,
    dangerous_patterns: Vec<DangerousPattern>,
}

struct SecretPattern {
    name: String,
    pattern: Regex,
    severity: IssueSeverity,
}

struct DangerousPattern {
    name: String,
    pattern: Regex,
    description: String,
    severity: IssueSeverity,
}

impl SecurityValidator {
    pub fn new() -> Self {
        Self {
            secret_patterns: Self::default_secret_patterns(),
            dangerous_patterns: Self::default_dangerous_patterns(),
        }
    }

    fn default_secret_patterns() -> Vec<SecretPattern> {
        vec![
            SecretPattern {
                name: "AWS Access Key".to_string(),
                pattern: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
                severity: IssueSeverity::Error,
            },
            SecretPattern {
                name: "AWS Secret Key".to_string(),
                pattern: Regex::new(
                    r#"(?i)aws_secret_access_key\s*=\s*['"][A-Za-z0-9/+=]{40}['"]"#,
                )
                .unwrap(),
                severity: IssueSeverity::Error,
            },
            SecretPattern {
                name: "GitHub Token".to_string(),
                pattern: Regex::new(r"gh[pousr]_[A-Za-z0-9_]{36,}").unwrap(),
                severity: IssueSeverity::Error,
            },
            SecretPattern {
                name: "Generic API Key".to_string(),
                pattern: Regex::new(
                    r#"(?i)(api[_-]?key|apikey)\s*[=:]\s*['"][A-Za-z0-9_\-]{20,}['"]"#,
                )
                .unwrap(),
                severity: IssueSeverity::Warning,
            },
            SecretPattern {
                name: "Generic Secret".to_string(),
                pattern: Regex::new(
                    r#"(?i)(secret|password|passwd|pwd)\s*[=:]\s*['"][^'"]{8,}['"]"#,
                )
                .unwrap(),
                severity: IssueSeverity::Warning,
            },
            SecretPattern {
                name: "Private Key".to_string(),
                pattern: Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----")
                    .unwrap(),
                severity: IssueSeverity::Error,
            },
            SecretPattern {
                name: "Slack Token".to_string(),
                pattern: Regex::new(r"xox[baprs]-[0-9]{10,13}-[0-9]{10,13}[a-zA-Z0-9-]*").unwrap(),
                severity: IssueSeverity::Error,
            },
            SecretPattern {
                name: "Stripe Key".to_string(),
                pattern: Regex::new(r"sk_live_[0-9a-zA-Z]{24,}").unwrap(),
                severity: IssueSeverity::Error,
            },
        ]
    }

    fn default_dangerous_patterns() -> Vec<DangerousPattern> {
        vec![
            DangerousPattern {
                name: "Dangerous rm command".to_string(),
                pattern: Regex::new(r"rm\s+.*-[rf]").unwrap(),
                description: "Potentially dangerous rm command with force/recursive flags".to_string(),
                severity: IssueSeverity::Warning,
            },
            DangerousPattern {
                name: "Sudo usage".to_string(),
                pattern: Regex::new(r"sudo\s+").unwrap(),
                description: "Code uses sudo which may have unintended side effects".to_string(),
                severity: IssueSeverity::Warning,
            },
            DangerousPattern {
                name: "Eval usage".to_string(),
                pattern: Regex::new(r"\beval\s*\(").unwrap(),
                description: "Use of eval() is dangerous and should be avoided".to_string(),
                severity: IssueSeverity::Warning,
            },
            DangerousPattern {
                name: "Shell injection risk".to_string(),
                pattern: Regex::new(r"(?i)(subprocess|os\.system|shell=True)").unwrap(),
                description: "Potential shell injection vulnerability".to_string(),
                severity: IssueSeverity::Warning,
            },
            DangerousPattern {
                name: "SQL injection risk".to_string(),
                pattern: Regex::new(r#"(?i)(execute|query)\s*\(\s*['"].*\+|f['"].*\{"#).unwrap(),
                description: "Potential SQL injection through string concatenation".to_string(),
                severity: IssueSeverity::Warning,
            },
            DangerousPattern {
                name: "Hardcoded credentials".to_string(),
                pattern: Regex::new(r#"(?i)(username|user|login)\s*=\s*['"][^'"]+['"].*(?i)(password|pwd|pass)\s*=\s*['"][^'"]+['"]"#).unwrap(),
                description: "Hardcoded credentials detected".to_string(),
                severity: IssueSeverity::Error,
            },
        ]
    }

    fn scan_content(&self, content: &str, file_path: &PathBuf) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check for secrets
        for pattern in &self.secret_patterns {
            if pattern.pattern.is_match(content) {
                issues.push(ValidationIssue {
                    file: Some(file_path.clone()),
                    line: None,
                    column: None,
                    message: format!("Potential {} detected", pattern.name),
                    code: Some(format!(
                        "SECRET_{}",
                        pattern.name.to_uppercase().replace(' ', "_")
                    )),
                    severity: pattern.severity,
                });
            }
        }

        // Check for dangerous patterns
        for pattern in &self.dangerous_patterns {
            if pattern.pattern.is_match(content) {
                issues.push(ValidationIssue {
                    file: Some(file_path.clone()),
                    line: None,
                    column: None,
                    message: pattern.description.clone(),
                    code: Some(format!(
                        "SECURITY_{}",
                        pattern.name.to_uppercase().replace(' ', "_")
                    )),
                    severity: pattern.severity,
                });
            }
        }

        issues
    }
}

impl Default for SecurityValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Validator for SecurityValidator {
    fn name(&self) -> &str {
        "security"
    }

    fn description(&self) -> &str {
        "Scans for secrets and dangerous code patterns"
    }

    fn should_run(&self, _context: &ValidationContext) -> bool {
        true
    }

    async fn validate(&self, context: &ValidationContext) -> Result<ValidationResult> {
        let start = std::time::Instant::now();
        let mut details = ValidationDetails::default();
        let mut has_errors = false;

        // Scan changed files
        for file in &context.changed_files {
            let full_path = context.working_dir.join(file);

            // Skip binary files and common non-code files
            let ext = file
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            if matches!(
                ext.as_str(),
                "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "ico"
                    | "woff"
                    | "woff2"
                    | "ttf"
                    | "eot"
                    | "pdf"
                    | "zip"
                    | "tar"
                    | "gz"
            ) {
                continue;
            }

            if let Ok(content) = tokio::fs::read_to_string(&full_path).await {
                let issues = self.scan_content(&content, file);

                for issue in issues {
                    if matches!(issue.severity, IssueSeverity::Error) {
                        has_errors = true;
                        details.errors.push(issue);
                    } else {
                        details.warnings.push(issue);
                    }
                }
            }
        }

        let duration = start.elapsed().as_millis() as u64;
        let warning_count = details.warnings.len();
        let error_count = details.errors.len();

        if has_errors {
            Ok(ValidationResult::failure(
                self.name(),
                format!("Found {} security issue(s)", error_count),
            )
            .with_duration(duration)
            .with_details(details)
            .with_recommendation("Remove or rotate exposed secrets and fix security issues"))
        } else if warning_count > 0 {
            Ok(ValidationResult::success(self.name())
                .with_duration(duration)
                .with_details(details)
                .with_action(format!("Found {} warning(s) to review", warning_count)))
        } else {
            Ok(ValidationResult::success(self.name())
                .with_duration(duration)
                .with_action(format!(
                    "Scanned {} files, no issues found",
                    context.changed_files.len()
                )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_validator_detects_aws_key() {
        let validator = SecurityValidator::new();
        let content = "aws_key = 'AKIAIOSFODNN7EXAMPLE'";
        let issues = validator.scan_content(content, &PathBuf::from("config.py"));

        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.message.contains("AWS")));
    }

    #[test]
    fn test_security_validator_detects_github_token() {
        let validator = SecurityValidator::new();
        let content = "token = 'ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'";
        let issues = validator.scan_content(content, &PathBuf::from("script.py"));

        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.message.contains("GitHub")));
    }

    #[test]
    fn test_security_validator_detects_dangerous_rm() {
        let validator = SecurityValidator::new();
        let content = "rm -rf /tmp/*";
        let issues = validator.scan_content(content, &PathBuf::from("cleanup.sh"));

        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.message.contains("rm")));
    }

    #[test]
    fn test_security_validator_clean_content() {
        let validator = SecurityValidator::new();
        let content = "fn main() { println!(\"Hello, world!\"); }";
        let issues = validator.scan_content(content, &PathBuf::from("main.rs"));

        assert!(issues.is_empty());
    }
}
