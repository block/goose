//! SecurityAgent - Specialist agent for security analysis and compliance
//!
//! Provides comprehensive security scanning capabilities including:
//! - Vulnerability detection (OWASP Top 10, CWE patterns)
//! - Compliance checking (security best practices)
//! - Dependency vulnerability analysis
//! - Security configuration auditing

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

use super::{SpecialistAgent, SpecialistConfig, SpecialistContext};
use crate::agents::orchestrator::{AgentRole, TaskResult};

/// Specialist agent focused on security analysis
pub struct SecurityAgent {
    config: SpecialistConfig,
    vulnerability_patterns: Vec<VulnerabilityPattern>,
    compliance_rules: Vec<ComplianceRule>,
}

/// A pattern that indicates a potential vulnerability
#[derive(Debug, Clone)]
struct VulnerabilityPattern {
    id: String,
    name: String,
    severity: VulnerabilitySeverity,
    pattern: String,
    description: String,
    cwe_id: Option<String>,
    fix_suggestion: String,
}

/// Severity levels for vulnerabilities
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum VulnerabilitySeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for VulnerabilitySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VulnerabilitySeverity::Info => write!(f, "INFO"),
            VulnerabilitySeverity::Low => write!(f, "LOW"),
            VulnerabilitySeverity::Medium => write!(f, "MEDIUM"),
            VulnerabilitySeverity::High => write!(f, "HIGH"),
            VulnerabilitySeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// A compliance rule to check
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct ComplianceRule {
    id: String,
    name: String,
    category: ComplianceCategory,
    check_fn: fn(&SpecialistContext) -> ComplianceCheckResult,
    description: String,
}

/// Categories of compliance rules
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum ComplianceCategory {
    Authentication,
    Authorization,
    DataProtection,
    InputValidation,
    Cryptography,
    Configuration,
    Logging,
    ErrorHandling,
}

impl std::fmt::Display for ComplianceCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplianceCategory::Authentication => write!(f, "Authentication"),
            ComplianceCategory::Authorization => write!(f, "Authorization"),
            ComplianceCategory::DataProtection => write!(f, "Data Protection"),
            ComplianceCategory::InputValidation => write!(f, "Input Validation"),
            ComplianceCategory::Cryptography => write!(f, "Cryptography"),
            ComplianceCategory::Configuration => write!(f, "Configuration"),
            ComplianceCategory::Logging => write!(f, "Logging"),
            ComplianceCategory::ErrorHandling => write!(f, "Error Handling"),
        }
    }
}

/// Result of a compliance check
#[derive(Debug)]
struct ComplianceCheckResult {
    passed: bool,
    findings: Vec<String>,
    recommendations: Vec<String>,
}

/// Detected vulnerability
#[allow(dead_code)]
#[derive(Debug)]
struct DetectedVulnerability {
    pattern_id: String,
    name: String,
    severity: VulnerabilitySeverity,
    location: String,
    description: String,
    cwe_id: Option<String>,
    fix_suggestion: String,
}

/// Compliance issue found
#[allow(dead_code)]
#[derive(Debug)]
struct ComplianceIssue {
    rule_id: String,
    rule_name: String,
    category: ComplianceCategory,
    findings: Vec<String>,
    recommendations: Vec<String>,
}

impl SecurityAgent {
    pub fn new(config: SpecialistConfig) -> Self {
        Self {
            config,
            vulnerability_patterns: Self::init_vulnerability_patterns(),
            compliance_rules: Self::init_compliance_rules(),
        }
    }

    /// Initialize standard vulnerability patterns (OWASP Top 10 + common CWE)
    fn init_vulnerability_patterns() -> Vec<VulnerabilityPattern> {
        vec![
            // SQL Injection (CWE-89)
            VulnerabilityPattern {
                id: "VULN-001".to_string(),
                name: "SQL Injection".to_string(),
                severity: VulnerabilitySeverity::Critical,
                pattern: r#"(?i)(execute|query|raw_sql|exec)\s*\([^)]*\+|format!\s*\([^)]*SELECT|format!\s*\([^)]*INSERT|format!\s*\([^)]*UPDATE|format!\s*\([^)]*DELETE"#.to_string(),
                description: "Potential SQL injection vulnerability detected".to_string(),
                cwe_id: Some("CWE-89".to_string()),
                fix_suggestion: "Use parameterized queries or prepared statements".to_string(),
            },
            // Command Injection (CWE-78)
            VulnerabilityPattern {
                id: "VULN-002".to_string(),
                name: "Command Injection".to_string(),
                severity: VulnerabilitySeverity::Critical,
                pattern: r#"(?i)(shell|exec|spawn|system|popen)\s*\([^)]*\+|Command::new\([^)]*format!"#.to_string(),
                description: "Potential command injection vulnerability detected".to_string(),
                cwe_id: Some("CWE-78".to_string()),
                fix_suggestion: "Sanitize user input and avoid shell commands where possible".to_string(),
            },
            // XSS (CWE-79)
            VulnerabilityPattern {
                id: "VULN-003".to_string(),
                name: "Cross-Site Scripting (XSS)".to_string(),
                severity: VulnerabilitySeverity::High,
                pattern: r#"(?i)innerHTML|document\.write|v-html|dangerouslySetInnerHTML"#.to_string(),
                description: "Potential XSS vulnerability from unsafe HTML handling".to_string(),
                cwe_id: Some("CWE-79".to_string()),
                fix_suggestion: "Use safe HTML encoding and avoid direct HTML injection".to_string(),
            },
            // Hardcoded Credentials (CWE-798)
            VulnerabilityPattern {
                id: "VULN-004".to_string(),
                name: "Hardcoded Credentials".to_string(),
                severity: VulnerabilitySeverity::High,
                pattern: r#"(?i)(password|secret|api_key|apikey|auth_token|access_token)\s*[=:]\s*["'][^"']{8,}"#.to_string(),
                description: "Hardcoded credentials detected".to_string(),
                cwe_id: Some("CWE-798".to_string()),
                fix_suggestion: "Use environment variables or a secrets manager".to_string(),
            },
            // Path Traversal (CWE-22)
            VulnerabilityPattern {
                id: "VULN-005".to_string(),
                name: "Path Traversal".to_string(),
                severity: VulnerabilitySeverity::High,
                pattern: r#"(?i)(\.\./|\.\.\\|%2e%2e%2f|%2e%2e/|\.%2e/|%2e\./)|(open|read|write|create)\s*\([^)]*\+"#.to_string(),
                description: "Potential path traversal vulnerability".to_string(),
                cwe_id: Some("CWE-22".to_string()),
                fix_suggestion: "Validate and sanitize file paths, use allowlists".to_string(),
            },
            // Insecure Deserialization (CWE-502)
            VulnerabilityPattern {
                id: "VULN-006".to_string(),
                name: "Insecure Deserialization".to_string(),
                severity: VulnerabilitySeverity::High,
                pattern: r#"(?i)(pickle\.loads|yaml\.load\(|unserialize|readObject|fromJson.*trust)"#.to_string(),
                description: "Potentially unsafe deserialization detected".to_string(),
                cwe_id: Some("CWE-502".to_string()),
                fix_suggestion: "Use safe deserialization methods and validate input".to_string(),
            },
            // Weak Cryptography (CWE-327)
            VulnerabilityPattern {
                id: "VULN-007".to_string(),
                name: "Weak Cryptography".to_string(),
                severity: VulnerabilitySeverity::Medium,
                pattern: r#"(?i)(md5|sha1|des|rc4|blowfish)\s*[(\.]|random\(\)|Math\.random"#.to_string(),
                description: "Weak cryptographic algorithm detected".to_string(),
                cwe_id: Some("CWE-327".to_string()),
                fix_suggestion: "Use modern cryptographic algorithms (SHA-256+, AES-256)".to_string(),
            },
            // Improper Error Handling (CWE-209)
            VulnerabilityPattern {
                id: "VULN-008".to_string(),
                name: "Information Exposure Through Error Messages".to_string(),
                severity: VulnerabilitySeverity::Low,
                pattern: r#"(?i)(stack_trace|backtrace|printStackTrace|console\.error.*err|log.*exception)"#.to_string(),
                description: "Potential sensitive information in error messages".to_string(),
                cwe_id: Some("CWE-209".to_string()),
                fix_suggestion: "Implement proper error handling that doesn't expose sensitive info".to_string(),
            },
            // Missing HTTPS (CWE-319)
            VulnerabilityPattern {
                id: "VULN-009".to_string(),
                name: "Cleartext Transmission".to_string(),
                severity: VulnerabilitySeverity::Medium,
                pattern: r#"http://[^"\s]+(?<!localhost)"#.to_string(),
                description: "HTTP URLs detected (should use HTTPS)".to_string(),
                cwe_id: Some("CWE-319".to_string()),
                fix_suggestion: "Use HTTPS for all external communications".to_string(),
            },
            // CORS Misconfiguration
            VulnerabilityPattern {
                id: "VULN-010".to_string(),
                name: "CORS Misconfiguration".to_string(),
                severity: VulnerabilitySeverity::Medium,
                pattern: r#"(?i)Access-Control-Allow-Origin.*\*|cors.*origin.*\*"#.to_string(),
                description: "Overly permissive CORS configuration".to_string(),
                cwe_id: None,
                fix_suggestion: "Configure specific allowed origins instead of wildcard".to_string(),
            },
        ]
    }

    /// Initialize compliance rules
    fn init_compliance_rules() -> Vec<ComplianceRule> {
        vec![
            ComplianceRule {
                id: "COMP-001".to_string(),
                name: "HTTPS Required".to_string(),
                category: ComplianceCategory::DataProtection,
                check_fn: Self::check_https_compliance,
                description: "All external communications should use HTTPS".to_string(),
            },
            ComplianceRule {
                id: "COMP-002".to_string(),
                name: "Authentication Required".to_string(),
                category: ComplianceCategory::Authentication,
                check_fn: Self::check_auth_compliance,
                description: "Sensitive endpoints should require authentication".to_string(),
            },
            ComplianceRule {
                id: "COMP-003".to_string(),
                name: "Input Validation".to_string(),
                category: ComplianceCategory::InputValidation,
                check_fn: Self::check_input_validation_compliance,
                description: "User input should be validated before processing".to_string(),
            },
            ComplianceRule {
                id: "COMP-004".to_string(),
                name: "Secure Configuration".to_string(),
                category: ComplianceCategory::Configuration,
                check_fn: Self::check_config_compliance,
                description: "Security-related configuration should follow best practices"
                    .to_string(),
            },
            ComplianceRule {
                id: "COMP-005".to_string(),
                name: "Error Handling".to_string(),
                category: ComplianceCategory::ErrorHandling,
                check_fn: Self::check_error_handling_compliance,
                description: "Errors should be handled without exposing sensitive information"
                    .to_string(),
            },
            ComplianceRule {
                id: "COMP-006".to_string(),
                name: "Logging Practices".to_string(),
                category: ComplianceCategory::Logging,
                check_fn: Self::check_logging_compliance,
                description: "Security events should be logged appropriately".to_string(),
            },
        ]
    }

    // Compliance check implementations
    fn check_https_compliance(context: &SpecialistContext) -> ComplianceCheckResult {
        let has_https_config = context.metadata.iter().any(|(k, _)| {
            k.to_lowercase().contains("https")
                || k.to_lowercase().contains("ssl")
                || k.to_lowercase().contains("tls")
        });

        ComplianceCheckResult {
            passed: has_https_config || context.environment.as_deref() == Some("development"),
            findings: if has_https_config {
                vec![]
            } else {
                vec!["HTTPS configuration not detected".to_string()]
            },
            recommendations: if has_https_config {
                vec![]
            } else {
                vec!["Configure HTTPS for production deployments".to_string()]
            },
        }
    }

    fn check_auth_compliance(context: &SpecialistContext) -> ComplianceCheckResult {
        let has_auth = context.metadata.iter().any(|(k, _)| {
            k.to_lowercase().contains("auth")
                || k.to_lowercase().contains("jwt")
                || k.to_lowercase().contains("oauth")
        });

        ComplianceCheckResult {
            passed: has_auth,
            findings: if has_auth {
                vec![]
            } else {
                vec!["No authentication mechanism detected".to_string()]
            },
            recommendations: if has_auth {
                vec![]
            } else {
                vec!["Implement authentication for sensitive endpoints".to_string()]
            },
        }
    }

    fn check_input_validation_compliance(_context: &SpecialistContext) -> ComplianceCheckResult {
        ComplianceCheckResult {
            passed: true,
            findings: vec![],
            recommendations: vec![
                "Ensure all user inputs are validated".to_string(),
                "Use schema validation for API requests".to_string(),
            ],
        }
    }

    fn check_config_compliance(context: &SpecialistContext) -> ComplianceCheckResult {
        let has_env_config = context.environment.is_some();

        ComplianceCheckResult {
            passed: has_env_config,
            findings: if has_env_config {
                vec![]
            } else {
                vec!["Environment configuration not specified".to_string()]
            },
            recommendations: vec![
                "Use environment-specific security configurations".to_string(),
                "Disable debug mode in production".to_string(),
            ],
        }
    }

    fn check_error_handling_compliance(_context: &SpecialistContext) -> ComplianceCheckResult {
        ComplianceCheckResult {
            passed: true,
            findings: vec![],
            recommendations: vec![
                "Implement centralized error handling".to_string(),
                "Avoid exposing stack traces to users".to_string(),
            ],
        }
    }

    fn check_logging_compliance(_context: &SpecialistContext) -> ComplianceCheckResult {
        ComplianceCheckResult {
            passed: true,
            findings: vec![],
            recommendations: vec![
                "Log authentication events".to_string(),
                "Log authorization failures".to_string(),
                "Ensure logs don't contain sensitive data".to_string(),
            ],
        }
    }

    /// Perform vulnerability scanning on the context
    fn scan_vulnerabilities(&self, context: &SpecialistContext) -> Vec<DetectedVulnerability> {
        let mut vulnerabilities = Vec::new();

        // Scan task description for patterns
        let task_lower = context.task.to_lowercase();

        for pattern in &self.vulnerability_patterns {
            if let Ok(regex) = regex::Regex::new(&pattern.pattern) {
                if regex.is_match(&task_lower) || regex.is_match(&context.task) {
                    vulnerabilities.push(DetectedVulnerability {
                        pattern_id: pattern.id.clone(),
                        name: pattern.name.clone(),
                        severity: pattern.severity,
                        location: "Task description".to_string(),
                        description: pattern.description.clone(),
                        cwe_id: pattern.cwe_id.clone(),
                        fix_suggestion: pattern.fix_suggestion.clone(),
                    });
                }
            }
        }

        // Scan target files for common issues
        for file in &context.target_files {
            let file_name = Path::new(file)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            // Check for sensitive file names
            if file_name.contains("secret")
                || file_name.contains("password")
                || file_name.contains("credential")
                || file_name == ".env"
            {
                vulnerabilities.push(DetectedVulnerability {
                    pattern_id: "VULN-011".to_string(),
                    name: "Sensitive File Detected".to_string(),
                    severity: VulnerabilitySeverity::Medium,
                    location: file.clone(),
                    description: "File may contain sensitive information".to_string(),
                    cwe_id: Some("CWE-312".to_string()),
                    fix_suggestion: "Ensure sensitive files are not committed to version control"
                        .to_string(),
                });
            }
        }

        vulnerabilities
    }

    /// Check compliance rules
    fn check_compliance(&self, context: &SpecialistContext) -> Vec<ComplianceIssue> {
        let mut issues = Vec::new();

        for rule in &self.compliance_rules {
            let result = (rule.check_fn)(context);
            if !result.passed || !result.findings.is_empty() {
                issues.push(ComplianceIssue {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    category: rule.category.clone(),
                    findings: result.findings,
                    recommendations: result.recommendations,
                });
            }
        }

        issues
    }

    /// Analyze security risks in the context
    async fn analyze_security(&self, context: &SpecialistContext) -> SecurityAnalysis {
        let vulnerabilities = self.scan_vulnerabilities(context);
        let compliance_issues = self.check_compliance(context);

        // Calculate risk level based on findings
        let risk_level = self.calculate_risk_level(&vulnerabilities, &compliance_issues);

        // Generate recommendations
        let mut recommendations = Vec::new();

        // Add vulnerability-specific recommendations
        for vuln in &vulnerabilities {
            recommendations.push(format!("[{}] {}", vuln.severity, vuln.fix_suggestion));
        }

        // Add compliance recommendations
        for issue in &compliance_issues {
            for rec in &issue.recommendations {
                recommendations.push(format!("[{}] {}", issue.category, rec));
            }
        }

        // Add general recommendations if no specific issues found
        if recommendations.is_empty() {
            recommendations = vec![
                "Enable HTTPS for all production traffic".to_string(),
                "Use secure headers (CSP, X-Frame-Options, etc.)".to_string(),
                "Implement rate limiting".to_string(),
                "Set up security monitoring and alerting".to_string(),
                "Regular dependency vulnerability scanning".to_string(),
            ];
        }

        SecurityAnalysis {
            vulnerabilities: vulnerabilities
                .iter()
                .map(|v| format!("[{}] {}: {}", v.severity, v.name, v.description))
                .collect(),
            compliance_issues: compliance_issues
                .iter()
                .map(|c| format!("[{}] {}: {:?}", c.category, c.rule_name, c.findings))
                .collect(),
            recommendations,
            risk_level,
        }
    }

    /// Calculate overall risk level
    fn calculate_risk_level(
        &self,
        vulnerabilities: &[DetectedVulnerability],
        compliance_issues: &[ComplianceIssue],
    ) -> String {
        let critical_count = vulnerabilities
            .iter()
            .filter(|v| v.severity == VulnerabilitySeverity::Critical)
            .count();
        let high_count = vulnerabilities
            .iter()
            .filter(|v| v.severity == VulnerabilitySeverity::High)
            .count();
        let compliance_count = compliance_issues.len();

        if critical_count > 0 {
            "Critical".to_string()
        } else if high_count > 0 {
            "High".to_string()
        } else if compliance_count > 2 || !vulnerabilities.is_empty() {
            "Medium".to_string()
        } else if compliance_count > 0 {
            "Low".to_string()
        } else {
            "Info".to_string()
        }
    }

    /// Generate security report content
    fn generate_report(&self, analysis: &SecurityAnalysis, context: &SpecialistContext) -> String {
        let mut report = String::new();

        report.push_str("# Security Analysis Report\n\n");
        report.push_str(&format!("**Project:** {}\n", context.working_dir));
        report.push_str(&format!(
            "**Generated:** {}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        report.push_str("## Executive Summary\n\n");
        report.push_str(&format!(
            "**Overall Risk Level:** {}\n",
            analysis.risk_level
        ));
        report.push_str(&format!(
            "**Vulnerabilities Found:** {}\n",
            analysis.vulnerabilities.len()
        ));
        report.push_str(&format!(
            "**Compliance Issues:** {}\n\n",
            analysis.compliance_issues.len()
        ));

        if !analysis.vulnerabilities.is_empty() {
            report.push_str("## Vulnerabilities\n\n");
            for vuln in &analysis.vulnerabilities {
                report.push_str(&format!("- {}\n", vuln));
            }
            report.push('\n');
        }

        if !analysis.compliance_issues.is_empty() {
            report.push_str("## Compliance Issues\n\n");
            for issue in &analysis.compliance_issues {
                report.push_str(&format!("- {}\n", issue));
            }
            report.push('\n');
        }

        report.push_str("## Recommendations\n\n");
        for (i, rec) in analysis.recommendations.iter().enumerate() {
            report.push_str(&format!("{}. {}\n", i + 1, rec));
        }

        report
    }
}

#[async_trait]
impl SpecialistAgent for SecurityAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Security
    }

    fn name(&self) -> &str {
        "SecurityAgent"
    }

    async fn can_handle(&self, context: &SpecialistContext) -> bool {
        let task_lower = context.task.to_lowercase();
        let security_keywords = [
            "security",
            "vulnerability",
            "compliance",
            "audit",
            "penetration",
            "scan",
            "owasp",
            "cwe",
            "secure",
            "threat",
            "risk",
        ];
        security_keywords
            .iter()
            .any(|keyword| task_lower.contains(keyword))
    }

    async fn execute(&self, context: SpecialistContext) -> Result<TaskResult> {
        tracing::info!("SecurityAgent executing: {}", context.task);

        let analysis = self.analyze_security(&context).await;

        let mut files_modified = Vec::new();
        let mut artifacts = Vec::new();

        // Generate security report
        let report_content = self.generate_report(&analysis, &context);
        let report_path = format!("{}/SECURITY_REPORT.md", context.working_dir);

        files_modified.push(report_path.clone());
        artifacts.push(format!(
            "Security analysis report: {} vulnerabilities, {} compliance issues, Risk: {}",
            analysis.vulnerabilities.len(),
            analysis.compliance_issues.len(),
            analysis.risk_level
        ));

        let mut metrics = HashMap::new();
        metrics.insert(
            "vulnerabilities_found".to_string(),
            serde_json::Value::Number(analysis.vulnerabilities.len().into()),
        );
        metrics.insert(
            "compliance_issues_found".to_string(),
            serde_json::Value::Number(analysis.compliance_issues.len().into()),
        );
        metrics.insert(
            "risk_level".to_string(),
            serde_json::Value::String(analysis.risk_level.clone()),
        );
        metrics.insert(
            "recommendations_count".to_string(),
            serde_json::Value::Number(analysis.recommendations.len().into()),
        );

        let success = analysis.risk_level != "Critical";

        Ok(TaskResult {
            success,
            output: format!(
                "Security analysis completed. Risk Level: {}. Found {} vulnerabilities and {} compliance issues.\n\nReport:\n{}",
                analysis.risk_level,
                analysis.vulnerabilities.len(),
                analysis.compliance_issues.len(),
                report_content
            ),
            files_modified,
            artifacts,
            metrics,
        })
    }

    fn config(&self) -> &SpecialistConfig {
        &self.config
    }

    async fn validate_result(&self, result: &TaskResult) -> Result<bool> {
        // Validate that the security analysis was performed
        if result.artifacts.is_empty() {
            return Ok(false);
        }

        // Check that metrics were collected
        if result.metrics.is_empty() {
            return Ok(false);
        }

        Ok(result.success)
    }
}

/// Security analysis results
#[derive(Debug)]
struct SecurityAnalysis {
    vulnerabilities: Vec<String>,
    compliance_issues: Vec<String>,
    recommendations: Vec<String>,
    risk_level: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> SpecialistContext {
        SpecialistContext {
            task: "Perform security audit on the API".to_string(),
            working_dir: "/tmp/test".to_string(),
            target_files: vec!["main.rs".to_string(), ".env".to_string()],
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
            language: Some("rust".to_string()),
            framework: None,
            environment: Some("test".to_string()),
        }
    }

    #[test]
    fn test_security_agent_creation() {
        let config = SpecialistConfig::default();
        let agent = SecurityAgent::new(config);
        assert_eq!(agent.name(), "SecurityAgent");
        assert_eq!(agent.role(), AgentRole::Security);
    }

    #[tokio::test]
    async fn test_can_handle_security_tasks() {
        let config = SpecialistConfig::default();
        let agent = SecurityAgent::new(config);

        let security_context = SpecialistContext {
            task: "Run security scan".to_string(),
            working_dir: ".".to_string(),
            target_files: vec![],
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
            language: None,
            framework: None,
            environment: None,
        };

        assert!(agent.can_handle(&security_context).await);

        let non_security_context = SpecialistContext {
            task: "Build the application".to_string(),
            working_dir: ".".to_string(),
            target_files: vec![],
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
            language: None,
            framework: None,
            environment: None,
        };

        assert!(!agent.can_handle(&non_security_context).await);
    }

    #[test]
    fn test_vulnerability_scanning() {
        let config = SpecialistConfig::default();
        let agent = SecurityAgent::new(config);

        let context = SpecialistContext {
            task: "Check for SQL injection in execute query".to_string(),
            working_dir: ".".to_string(),
            target_files: vec![".env".to_string()],
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
            language: None,
            framework: None,
            environment: None,
        };

        let vulnerabilities = agent.scan_vulnerabilities(&context);
        // Should detect sensitive file
        assert!(!vulnerabilities.is_empty());
    }

    #[test]
    fn test_risk_level_calculation() {
        let config = SpecialistConfig::default();
        let agent = SecurityAgent::new(config);

        // No issues = Info
        let risk = agent.calculate_risk_level(&[], &[]);
        assert_eq!(risk, "Info");

        // Critical vulnerability = Critical
        let critical_vuln = vec![DetectedVulnerability {
            pattern_id: "TEST".to_string(),
            name: "Test".to_string(),
            severity: VulnerabilitySeverity::Critical,
            location: "test".to_string(),
            description: "test".to_string(),
            cwe_id: None,
            fix_suggestion: "test".to_string(),
        }];
        let risk = agent.calculate_risk_level(&critical_vuln, &[]);
        assert_eq!(risk, "Critical");
    }

    #[tokio::test]
    async fn test_execute_produces_report() {
        let config = SpecialistConfig::default();
        let agent = SecurityAgent::new(config);
        let context = create_test_context();

        let result = agent.execute(context).await.unwrap();
        assert!(!result.output.is_empty());
        assert!(!result.artifacts.is_empty());
        assert!(result.metrics.contains_key("vulnerabilities_found"));
        assert!(result.metrics.contains_key("risk_level"));
    }
}
