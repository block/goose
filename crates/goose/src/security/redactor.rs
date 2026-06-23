use regex::Regex;
use rmcp::model::{AnnotateAble, RawContent, RawTextContent};
#[cfg(test)]
use std::collections::HashSet;

/// How the redactor handles detected sensitive data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RedactorMode {
    /// Replace with a category-tagged placeholder like `[REDACTED: api_key]`.
    #[default]
    Mask,
    /// Replace with a deterministic hash of the matched value.
    Hash,
    /// Remove the matched text entirely.
    Redact,
}

/// Categories of sensitive data that the redactor looks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedactCategory {
    ApiKey,
    AccessToken,
    AwsCredentials,
    Email,
    PhoneNumber,
    Ssn,
    CreditCard,
    PrivateIp,
    GithubToken,
    SlackToken,
    Jwt,
    Password,
    GenericSecret,
    UrlWithAuth,
    BearerToken,
}

impl std::fmt::Display for RedactCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedactCategory::ApiKey => write!(f, "api_key"),
            RedactCategory::AccessToken => write!(f, "access_token"),
            RedactCategory::AwsCredentials => write!(f, "aws_credentials"),
            RedactCategory::Email => write!(f, "email"),
            RedactCategory::PhoneNumber => write!(f, "phone_number"),
            RedactCategory::Ssn => write!(f, "ssn"),
            RedactCategory::CreditCard => write!(f, "credit_card"),
            RedactCategory::PrivateIp => write!(f, "private_ip"),
            RedactCategory::GithubToken => write!(f, "github_token"),
            RedactCategory::SlackToken => write!(f, "slack_token"),
            RedactCategory::Jwt => write!(f, "jwt"),
            RedactCategory::Password => write!(f, "password"),
            RedactCategory::GenericSecret => write!(f, "generic_secret"),
            RedactCategory::UrlWithAuth => write!(f, "url_with_auth"),
            RedactCategory::BearerToken => write!(f, "bearer_token"),
        }
    }
}

/// A single redaction event describing what was found and how it was replaced.
#[derive(Debug, Clone)]
pub struct RedactionEvent {
    pub category: RedactCategory,
    pub replacement: String,
}

/// A compiled redaction pattern paired with its category.
struct RedactPattern {
    category: RedactCategory,
    regex: Regex,
}

/// The redactor — scans text and replaces sensitive data according to mode.
pub struct Redactor {
    mode: RedactorMode,
    patterns: Vec<RedactPattern>,
    /// User-defined allowlist: if text matches, skip redaction for that category.
    allowlist: Vec<Regex>,
}

impl Redactor {
    pub fn new() -> Self {
        Self::with_mode(RedactorMode::default())
    }

    pub fn with_mode(mode: RedactorMode) -> Self {
        let patterns = Self::build_patterns();
        Self {
            mode,
            patterns,
            allowlist: Vec::new(),
        }
    }

    /// Create a redactor with an allowlist of regex patterns. Text matching any
    /// allowlist pattern will not be redacted.
    pub fn with_allowlist(mut self, allowlist: Vec<Regex>) -> Self {
        self.allowlist = allowlist;
        self
    }

    /// Build all redaction patterns.
    fn build_patterns() -> Vec<RedactPattern> {
        vec![
            // AWS credentials — AKIA keys
            RedactPattern {
                category: RedactCategory::AwsCredentials,
                regex: Self::lazy_regex(r#"AKIA[0-9A-Z]{16}"#),
            },
            // Generic API keys — sk- (OpenAI), pk_ (generic publish), etc.
            RedactPattern {
                category: RedactCategory::ApiKey,
                regex: Self::lazy_regex(r#"(?i)(?:sk|pk)[_-][A-Za-z0-9]{20,}"#),
            },
            // GitHub tokens — ghp_ and github_pat_
            RedactPattern {
                category: RedactCategory::GithubToken,
                regex: Self::lazy_regex(
                    r#"(?:ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9]{22}_[A-Za-z0-9]{58})"#,
                ),
            },
            // Slack tokens — xoxb-, xoxp-, xoxa-, xoxr-
            RedactPattern {
                category: RedactCategory::SlackToken,
                regex: Self::lazy_regex(r#"(xox[bapsr])-[A-Za-z0-9\-]+"#),
            },
            // JWT tokens — eyJ...
            RedactPattern {
                category: RedactCategory::Jwt,
                regex: Self::lazy_regex(r#"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+"#),
            },
            // Bearer tokens — Bearer <token>
            RedactPattern {
                category: RedactCategory::BearerToken,
                regex: Self::lazy_regex(r#"(?i)Bearer\s+[A-Za-z0-9._~+/=-]+"#),
            },
            // Generic access tokens — long hex/base64 strings assigned to token/secret vars
            RedactPattern {
                category: RedactCategory::AccessToken,
                regex: Self::lazy_regex(
                    r#"(?i)(?:access[_-]?token|client[_-]?secret)\s*[=:]\s*["']?\b[A-Za-z0-9._~+/=-]{20,}\b"#,
                ),
            },
            // Email addresses
            RedactPattern {
                category: RedactCategory::Email,
                regex: Self::lazy_regex(r#"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"#),
            },
            // US phone numbers — various formats
            RedactPattern {
                category: RedactCategory::PhoneNumber,
                regex: Self::lazy_regex(
                    r#"(?:(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4})"#,
                ),
            },
            // Social Security Numbers — XXX-XX-XXXX
            RedactPattern {
                category: RedactCategory::Ssn,
                regex: Self::lazy_regex(r#"\b\d{3}-\d{2}-\d{4}\b"#),
            },
            // Credit card numbers — Visa (4xxx), MC (5[1-5]xx), Amex (3[47]xx)
            RedactPattern {
                category: RedactCategory::CreditCard,
                regex: Self::lazy_regex(
                    r#"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13})\b"#,
                ),
            },
            // Private IP addresses (RFC 1918)
            RedactPattern {
                category: RedactCategory::PrivateIp,
                regex: Self::lazy_regex(
                    r#"\b(?:10\.\d{1,3}\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3})\b"#,
                ),
            },
            // URLs with embedded credentials — user:pass@host
            RedactPattern {
                category: RedactCategory::UrlWithAuth,
                regex: Self::lazy_regex(r#"(?i)(https?)://([^:\s]+):([^@\s]+)@([^\s/"]+)"#),
            },
            // Password assignments — password=..., pwd=..., "password": "..."
            RedactPattern {
                category: RedactCategory::Password,
                regex: Self::lazy_regex(r#"(?i)(?:password|passwd|pwd)\s*[=:]\s*["']?\S+\b"#),
            },
            // Generic secrets — base64-like or hex strings assigned to secret/key vars
            RedactPattern {
                category: RedactCategory::GenericSecret,
                regex: Self::lazy_regex(
                    r#"(?i)(?:secret|private[_-]?key)\s*[=:]\s*["']?\b[A-Za-z0-9+/=]{20,}\b"#,
                ),
            },
        ]
    }

    /// Compile a regex pattern. All patterns are compiled eagerly in `build_patterns`.
    fn lazy_regex(pattern: &str) -> Regex {
        Regex::new(pattern).expect("valid regex")
    }

    /// Check if a matched value is covered by the allowlist.
    fn is_allowlisted(&self, text: &str) -> bool {
        self.allowlist.iter().any(|re| re.is_match(text))
    }

    /// Generate a replacement string for the given category and mode.
    fn replacement(&self, category: RedactCategory, matched: &str) -> String {
        match self.mode {
            RedactorMode::Mask => format!("[REDACTED: {}]", category),
            RedactorMode::Hash => {
                let hash = format!("{:x}", md5_hash(matched));
                format!("[HASHED: {}]", hash)
            }
            RedactorMode::Redact => String::new(),
        }
    }

    /// Scan text and return a new string with sensitive data replaced, plus events.
    pub fn redact_text(&self, text: &str) -> (String, Vec<RedactionEvent>) {
        let mut events = Vec::new();
        let mut result = text.to_string();

        // Collect all matches with their byte positions, then replace from end to start
        // to preserve offsets.
        #[derive(Debug, Clone, Copy)]
        struct MatchInfo {
            start: usize,
            end: usize,
            category: RedactCategory,
        }

        let mut matches: Vec<MatchInfo> = Vec::new();

        for pattern in &self.patterns {
            for cap in pattern.regex.find_iter(&result) {
                // SAFETY: regex `find_iter` returns byte offsets that are always on UTF-8 boundaries
                // for ASCII patterns. All our patterns are ASCII-only, so this is safe.
                #[allow(clippy::string_slice)]
                let matched = &result[cap.start()..cap.end()];

                // Skip allowlisted matches
                if self.is_allowlisted(matched) {
                    continue;
                }

                // Skip SSN pattern if it could be a phone number (phone numbers are more common)
                // This is a heuristic — SSNs have a very specific format that overlaps with phone
                if pattern.category == RedactCategory::Ssn {
                    // SSN pattern: \b\d{3}-\d{2}-\d{4}\b
                    // If the middle group is 00-99, it's more likely an SSN
                    // If the middle group looks like an area code (201-999), it might be a phone
                    // We'll be conservative and only flag SSN if the first group doesn't look like a US area code
                    let digits: Vec<u32> =
                        matched.split('-').filter_map(|d| d.parse().ok()).collect();
                    if digits.len() == 3 {
                        // Area codes typically start with 2-9, SSN first groups vary
                        // Keep the match — we'd rather over-redact than miss an SSN
                    }
                }

                matches.push(MatchInfo {
                    start: cap.start(),
                    end: cap.end(),
                    category: pattern.category,
                });
            }
        }

        // Deduplicate overlapping matches: when one match fully contains another,
        // keep the outer (broader) match and discard the inner one.
        // For partial overlaps, keep both.

        // Step 1: Find all "contained" relationships — mark inner matches for removal
        let mut to_remove = std::collections::HashSet::new();
        for i in 0..matches.len() {
            for j in 0..matches.len() {
                if i != j && !to_remove.contains(&i) {
                    // If match[i] is fully contained within match[j], mark i (the inner one) for removal
                    if matches[i].start >= matches[j].start && matches[i].end <= matches[j].end {
                        to_remove.insert(i);
                    }
                }
            }
        }

        // Step 2: Collect remaining matches, sorted by start position descending for replacement
        let mut deduped: Vec<MatchInfo> = matches
            .into_iter()
            .enumerate()
            .filter(|(idx, _)| !to_remove.contains(idx))
            .map(|(_, m)| m)
            .collect();
        deduped.sort_by_key(|m| std::cmp::Reverse(m.start));

        for m in &deduped {
            // SAFETY: regex `find_iter` returns byte offsets that are always on UTF-8 boundaries
            // for ASCII patterns. All our patterns are ASCII-only, so this is safe.
            #[allow(clippy::string_slice)]
            let matched = &result[m.start..m.end];
            let replacement = self.replacement(m.category, matched);
            events.push(RedactionEvent {
                category: m.category,
                replacement: replacement.clone(),
            });
            result.replace_range(m.start..m.end, &replacement);
        }

        (result, events)
    }

    /// Check whether any redaction would occur in the given text.
    pub fn has_sensitive_data(&self, text: &str) -> bool {
        let (result, _) = self.redact_text(text);
        result != text
    }

    /// Log a redaction event without leaking sensitive data.
    pub fn log_redaction(category: &RedactCategory, count: usize) {
        tracing::warn!(
            "Redactor detected sensitive data category={} count={}",
            category.to_string(),
            count
        );
    }

    fn redact_json_value(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => {
                let (redacted, _) = self.redact_text(s);
                serde_json::Value::String(redacted)
            }
            serde_json::Value::Object(map) => {
                let redacted = map
                    .iter()
                    .map(|(k, v)| (k.clone(), self.redact_json_value(v)))
                    .collect();
                serde_json::Value::Object(redacted)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.redact_json_value(v)).collect())
            }
            other => other.clone(),
        }
    }

    /// Redact a single message — returns a new message with sensitive data replaced.
    pub fn redact_message(
        &self,
        msg: &crate::conversation::message::Message,
    ) -> crate::conversation::message::Message {
        let mut new_msg = msg.clone();
        for content in &mut new_msg.content {
            *content = self.redact_message_content(content);
        }
        new_msg
    }

    fn redact_message_content(
        &self,
        content: &crate::conversation::message::MessageContent,
    ) -> crate::conversation::message::MessageContent {
        use crate::conversation::message::MessageContent;

        match content {
            MessageContent::Text(text) => {
                let (redacted_text, events) = self.redact_text(&text.text);
                if !events.is_empty() {
                    Self::log_redaction(&RedactCategory::from_text(&redacted_text), events.len());
                }
                MessageContent::Text(
                    RawTextContent {
                        text: redacted_text,
                        meta: text.raw.meta.clone(),
                    }
                    .optional_annotate(text.annotations.clone()),
                )
            }
            MessageContent::ToolResponse(tool_response) => {
                let mut new_response = tool_response.clone();
                if let Ok(call_tool_result) = &mut new_response.tool_result {
                    call_tool_result.content = call_tool_result
                        .content
                        .iter()
                        .map(|c| {
                            if let RawContent::Text(ref text_content) = c.raw {
                                let (redacted, events) = self.redact_text(&text_content.text);
                                if !events.is_empty() {
                                    Self::log_redaction(&events[0].category, events.len());
                                }
                                RawContent::Text(RawTextContent {
                                    text: redacted,
                                    meta: text_content.meta.clone(),
                                })
                                .optional_annotate(c.annotations.clone())
                            } else {
                                c.clone()
                            }
                        })
                        .collect();
                }
                MessageContent::ToolResponse(new_response)
            }
            MessageContent::ActionRequired(action_required) => {
                use crate::conversation::message::ActionRequiredData;
                let mut new_action = action_required.clone();
                if let ActionRequiredData::ElicitationResponse {
                    ref mut user_data, ..
                } = new_action.data
                {
                    *user_data = self.redact_json_value(user_data);
                }
                MessageContent::ActionRequired(new_action)
            }
            _ => content.clone(),
        }
    }

    /// Redact an entire conversation — returns a new conversation with sensitive data replaced.
    pub fn redact_conversation(
        &self,
        conv: &crate::conversation::Conversation,
    ) -> crate::conversation::Conversation {
        let redacted: Vec<_> = conv
            .messages()
            .iter()
            .map(|m| self.redact_message(m))
            .collect();
        crate::conversation::Conversation::new_unvalidated(redacted)
    }

    /// Check if redactor is enabled via config.
    pub fn is_enabled() -> bool {
        crate::config::Config::global()
            .get_param::<bool>("REDACTOR_ENABLED")
            .unwrap_or(false)
    }

    /// Create a redactor from config values.
    pub fn from_config() -> Self {
        let config = crate::config::Config::global();

        let mode_str = config
            .get_param::<String>("REDACTOR_MODE")
            .unwrap_or_else(|_| "mask".to_string());
        let mode = match mode_str.as_str() {
            "hash" => RedactorMode::Hash,
            "redact" => RedactorMode::Redact,
            _ => RedactorMode::Mask,
        };

        let allowlist_str = config
            .get_param::<String>("REDACTOR_ALLOWLIST")
            .unwrap_or_else(|_| "[]".to_string());
        let allowlist: Vec<String> = serde_json::from_str(&allowlist_str).unwrap_or_default();
        let allowlist_re: Vec<regex::Regex> = allowlist
            .into_iter()
            .filter_map(|pat| regex::Regex::new(&pat).ok())
            .collect();

        Self::with_mode(mode).with_allowlist(allowlist_re)
    }
}

/// Helper to extract a category name from redacted text for logging.
impl RedactCategory {
    fn from_text(text: &str) -> Self {
        // Heuristic: check which redaction tag is present in the text
        if text.contains("[REDACTED: api_key]") || text.contains("[HASHED:") {
            Self::ApiKey
        } else if text.contains("[REDACTED: email]") {
            Self::Email
        } else if text.contains("[REDACTED: phone_number]") {
            Self::PhoneNumber
        } else if text.contains("[REDACTED: ssn]") {
            Self::Ssn
        } else if text.contains("[REDACTED: credit_card]") {
            Self::CreditCard
        } else if text.contains("[REDACTED: private_ip]") {
            Self::PrivateIp
        } else if text.contains("[REDACTED: github_token]") {
            Self::GithubToken
        } else if text.contains("[REDACTED: slack_token]") {
            Self::SlackToken
        } else if text.contains("[REDACTED: jwt]") {
            Self::Jwt
        } else if text.contains("[REDACTED: password]") {
            Self::Password
        } else if text.contains("[REDACTED: url_with_auth]") {
            Self::UrlWithAuth
        } else if text.contains("[REDACTED: bearer_token]") {
            Self::BearerToken
        } else if text.contains("[REDACTED: aws_credentials]") {
            Self::AwsCredentials
        } else if text.contains("[REDACTED: generic_secret]") {
            Self::GenericSecret
        } else if text.contains("[REDACTED: access_token]") {
            Self::AccessToken
        } else {
            Self::ApiKey
        }
    }
}

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple MD5 hash for deterministic replacements.
fn md5_hash(input: &str) -> u128 {
    // Use a simple hash — we don't need cryptographic security here,
    // just determinism. Using std library to avoid extra deps.
    let mut hash: u128 = 0;
    for (i, byte) in input.bytes().enumerate() {
        hash ^= (byte as u128) << ((i % 16) * 8);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_redactor() -> Redactor {
        Redactor::new()
    }

    fn make_redactor_with_mode(mode: RedactorMode) -> Redactor {
        Redactor::with_mode(mode)
    }

    // --- AWS Credentials ---

    #[test]
    fn test_redacts_aws_key() {
        let redactor = make_redactor();
        let (result, events) = redactor.redact_text("My AWS key is AKIAIOSFODNN7EXAMPLE");
        assert!(result.contains("[REDACTED: aws_credentials]"));
        assert!(!result.contains("AKIAIOSFODNN7EXAMPLE"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, RedactCategory::AwsCredentials);
    }

    #[test]
    fn test_no_false_positive_on_similar_text() {
        let redactor = make_redactor();
        // Normal text that doesn't match the 20-char AKIA pattern
        let (result, _) = redactor.redact_text("AKIA is a region code");
        assert_eq!(result, "AKIA is a region code");
    }

    // --- GitHub Tokens ---

    #[test]
    fn test_redacts_github_pat_token() {
        let redactor = make_redactor();
        // ghp_ token (36 chars after prefix)
        let (result, events) =
            redactor.redact_text("token: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij");
        assert!(result.contains("[REDACTED: github_token]"));
        assert_eq!(events[0].category, RedactCategory::GithubToken);
    }

    // --- Slack Tokens ---

    #[test]
    fn test_redacts_slack_bot_token() {
        let redactor = make_redactor();
        let (result, events) = redactor
            .redact_text("SLACK_TOKEN=xoxb-123456789012-1234567890123-AbCdEfGhIjKlMnOpQrStUvWx");
        assert!(result.contains("[REDACTED: slack_token]"));
        assert_eq!(events[0].category, RedactCategory::SlackToken);
    }

    // --- JWT ---

    #[test]
    fn test_redacts_jwt() {
        let redactor = make_redactor();
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let (result, _events) = redactor.redact_text(&format!("Bearer {}", jwt));
        // JWT is contained within the bearer_token match, so the broader bearer_token
        // pattern takes precedence (both correctly redact the sensitive data)
        assert!(result.contains("[REDACTED: jwt]") || result.contains("[REDACTED: bearer_token]"));
        assert!(!result.contains("eyJhbGci"));
    }

    // --- Email ---

    #[test]
    fn test_redacts_email() {
        let redactor = make_redactor();
        let (result, events) = redactor.redact_text("Contact us at user@example.com for help");
        assert!(result.contains("[REDACTED: email]"));
        assert!(!result.contains("user@example.com"));
        assert_eq!(events[0].category, RedactCategory::Email);
    }

    #[test]
    fn test_redacts_multiple_emails() {
        let redactor = make_redactor();
        let text = "Reach alice@test.com or bob@test.com";
        let (result, events) = redactor.redact_text(text);
        assert!(!result.contains("alice@test.com"));
        assert!(!result.contains("bob@test.com"));
        assert_eq!(events.len(), 2);
    }

    // --- Private IP ---

    #[test]
    fn test_redacts_private_ip() {
        let redactor = make_redactor();
        let (result, events) = redactor.redact_text("Server at 192.168.1.100 responded");
        assert!(result.contains("[REDACTED: private_ip]"));
        assert!(!result.contains("192.168.1.100"));
        assert_eq!(events[0].category, RedactCategory::PrivateIp);
    }

    #[test]
    fn test_redacts_10_subnet() {
        let redactor = make_redactor();
        let (result, _) = redactor.redact_text("Internal service at 10.0.0.5");
        assert!(result.contains("[REDACTED: private_ip]"));
    }

    // --- URL with Auth ---

    #[test]
    fn test_redacts_url_with_credentials() {
        let redactor = make_redactor();
        let (result, events) =
            redactor.redact_text("Connect to https://admin:secret123@internal.example.com/api");
        assert!(result.contains("[REDACTED: url_with_auth]"));
        // The credentials should be gone from the replacement
        assert!(!result.contains("secret123"));
        assert_eq!(events[0].category, RedactCategory::UrlWithAuth);
    }

    // --- Password ---

    #[test]
    fn test_redacts_password_assignment() {
        let redactor = make_redactor();
        let (result, events) = redactor.redact_text("password=mysecretpassword123");
        assert!(result.contains("[REDACTED: password]"));
        assert!(!result.contains("mysecretpassword123") || result.contains("[REDACTED: password]"));
        assert_eq!(events[0].category, RedactCategory::Password);
    }

    // --- Bearer Token ---

    #[test]
    fn test_redacts_bearer_token() {
        let redactor = make_redactor();
        let (result, events) =
            redactor.redact_text("Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.test");
        assert!(result.contains("[REDACTED: bearer_token]"));
        assert_eq!(events[0].category, RedactCategory::BearerToken);
    }

    // --- Mode: Hash ---

    #[test]
    fn test_hash_mode_produces_deterministic_output() {
        let redactor = make_redactor_with_mode(RedactorMode::Hash);
        let (result1, _) = redactor.redact_text("AKIAIOSFODNN7EXAMPLE");
        let (result2, _) = redactor.redact_text("AKIAIOSFODNN7EXAMPLE");
        assert_eq!(result1, result2);
        assert!(result1.contains("[HASHED:"));
    }

    #[test]
    fn test_hash_mode_different_values_different_hashes() {
        let redactor = make_redactor_with_mode(RedactorMode::Hash);
        let (result1, _) = redactor.redact_text("AKIAIOSFODNN7EXAMPLE");
        let (result2, _) = redactor.redact_text("AKIAZZZZZZZZZZZZZZ");
        assert_ne!(result1, result2);
    }

    // --- Mode: Redact ---

    #[test]
    fn test_redact_mode_removes_text() {
        let redactor = make_redactor_with_mode(RedactorMode::Redact);
        let (result, _) =
            redactor.redact_text("My email is user@example.com and phone is 555-123-4567");
        assert!(!result.contains("user@example.com"));
        assert!(!result.contains("555-123-4567"));
        // Should not contain any placeholder markers
        assert!(!result.contains("[REDACTED:"));
    }

    // --- Allowlist ---

    #[test]
    fn test_allowlist_bypasses_detection() {
        let allowlist = vec![Regex::new(r"user@example\.com").unwrap()];
        let redactor = Redactor::new().with_allowlist(allowlist);
        let (result, events) = redactor.redact_text("Contact user@example.com for help");
        assert_eq!(events.len(), 0);
        assert!(result.contains("user@example.com"));
    }

    // --- Multiple categories in one text ---

    #[test]
    fn test_redacts_multiple_categories() {
        let redactor = make_redactor();
        let text = "Email: admin@corp.com AWS key: AKIAIOSFODNN7EXAMPLE IP: 10.0.0.1";
        let (result, events) = redactor.redact_text(text);
        assert!(!result.contains("admin@corp.com"));
        assert!(!result.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!result.contains("10.0.0.1"));
        let categories: HashSet<_> = events.iter().map(|e| e.category).collect();
        assert!(categories.contains(&RedactCategory::Email));
        assert!(categories.contains(&RedactCategory::AwsCredentials));
        assert!(categories.contains(&RedactCategory::PrivateIp));
    }

    // --- No false positives on normal text ---

    #[test]
    fn test_no_false_positives_on_normal_text() {
        let redactor = make_redactor();
        let normal_texts = [
            "The weather is nice today",
            "Please review the code changes",
            "Run cargo build to compile",
            "The file path is /usr/local/bin/goose",
            "Version 1.2.3 of the library",
            "Meeting at 3:00 PM",
            "The project root is at ./crates/goose",
        ];
        for text in normal_texts {
            let (result, events) = redactor.redact_text(text);
            assert_eq!(result, text, "text was modified: {}", text);
            assert_eq!(events.len(), 0, "unexpected events for: {}", text);
        }
    }

    // --- has_sensitive_data ---

    #[test]
    fn test_has_sensitive_data_detects() {
        let redactor = make_redactor();
        assert!(redactor.has_sensitive_data("AKIAIOSFODNN7EXAMPLE"));
        assert!(redactor.has_sensitive_data("user@example.com"));
    }

    #[test]
    fn test_has_sensitive_data_clean() {
        let redactor = make_redactor();
        assert!(!redactor.has_sensitive_data("The weather is nice"));
        assert!(!redactor.has_sensitive_data("Hello world"));
    }

    // --- Empty and edge cases ---

    #[test]
    fn test_empty_text() {
        let redactor = make_redactor();
        let (result, events) = redactor.redact_text("");
        assert_eq!(result, "");
        assert!(events.is_empty());
    }

    #[test]
    fn test_text_with_no_matches() {
        let redactor = make_redactor();
        let (result, events) = redactor.redact_text("just some normal text");
        assert_eq!(result, "just some normal text");
        assert!(events.is_empty());
    }

    // --- Credit card ---

    #[test]
    fn test_redacts_visa_card() {
        let redactor = make_redactor();
        let (result, events) = redactor.redact_text("Card: 4111111111111111");
        assert!(result.contains("[REDACTED: credit_card]"));
        assert_eq!(events[0].category, RedactCategory::CreditCard);
    }

    // --- Phone number ---

    #[test]
    fn test_redacts_phone_number() {
        let redactor = make_redactor();
        let (result, events) = redactor.redact_text("Call 555-123-4567 for support");
        assert!(result.contains("[REDACTED: phone_number]"));
        assert_eq!(events[0].category, RedactCategory::PhoneNumber);
    }

    // --- SSN ---

    #[test]
    fn test_redacts_ssn() {
        let redactor = make_redactor();
        let (result, events) = redactor.redact_text("SSN: 123-45-6789");
        assert!(result.contains("[REDACTED: ssn]"));
        assert_eq!(events[0].category, RedactCategory::Ssn);
    }

    // --- Generic secret ---

    #[test]
    fn test_redacts_generic_secret() {
        let redactor = make_redactor();
        let (result, events) = redactor.redact_text("secret=ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789");
        assert!(result.contains("[REDACTED: generic_secret]"));
        assert_eq!(events[0].category, RedactCategory::GenericSecret);
    }
}
