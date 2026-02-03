# Goose Security Guardrails

## Overview

The Guardrails module provides comprehensive security scanning for AI interactions. It runs multiple detectors in parallel to identify and block potentially harmful content before it reaches the AI model or user.

## Features

- **6 Built-in Detectors**: Prompt injection, PII, jailbreak, topic, keyword, and secret detection
- **Async Parallel Execution**: All detectors run concurrently for minimal latency
- **Configurable Sensitivity**: Low, Medium, High, or Custom sensitivity levels
- **Extensible Architecture**: Easy to add custom detectors

## Quick Start

```rust
use goose::guardrails::{GuardrailsEngine, GuardrailsConfig, DetectionContext};

// Create engine with default configuration
let engine = GuardrailsEngine::new();

// Or with custom configuration
let config = GuardrailsConfig {
    enabled: true,
    fail_mode: FailMode::FailClosed,
    timeout_ms: 5000,
    ..Default::default()
};
let engine = GuardrailsEngine::with_config(config);

// Scan input
let context = DetectionContext::default();
let result = engine.scan("User input text", &context).await?;

if !result.passed {
    println!("Blocked: {:?}", result.blocked_reason);
}
```

## Detectors

### 1. Prompt Injection Detector

Detects attempts to manipulate AI behavior through crafted prompts.

**Patterns detected:**
- System prompt manipulation ("ignore previous instructions")
- Role hijacking ("you are now...")
- Instruction injection ("[INST]", "system:")
- Jailbreak indicators ("DAN mode", "developer mode")

```rust
use goose::guardrails::detectors::PromptInjectionDetector;

let detector = PromptInjectionDetector::new(DetectionSensitivity::Medium);
```

### 2. PII Detector

Identifies personally identifiable information in text.

**Supported PII types:**
- Email addresses
- Phone numbers
- Social Security Numbers (SSN)
- Credit card numbers (with Luhn validation)
- IP addresses
- Dates of birth

```rust
use goose::guardrails::detectors::{PiiDetector, PiiType};

let detector = PiiDetector::new()
    .with_allowed_type(PiiType::Email)  // Allow emails
    .with_redaction(true);               // Redact detected PII
```

### 3. Jailbreak Detector

Detects attempts to bypass AI safety measures.

**Patterns detected:**
- DAN (Do Anything Now) mode
- Developer mode exploits
- Character/roleplay exploits
- Token manipulation
- Bypass attempts

### 4. Topic Detector

Filters content based on allowed or banned topics.

**Built-in topics:**
- Violence
- Illegal activities
- Adult content
- Hate speech
- Self-harm
- Drugs/substances

```rust
use goose::guardrails::config::{TopicConfig, TopicMode};

let config = TopicConfig {
    enabled: true,
    mode: TopicMode::Blocklist,  // Block specific topics
    topics: vec!["violence".to_string(), "illegal_activities".to_string()],
    ..Default::default()
};
```

### 5. Keyword Detector

Detects custom keywords with multiple matching modes.

**Match modes:**
- Exact match
- Case-insensitive
- Fuzzy matching (Levenshtein distance)
- Phrase matching

```rust
use goose::guardrails::detectors::KeywordDetector;

let detector = KeywordDetector::new()
    .add_keyword("banned_word", MatchMode::CaseInsensitive)
    .add_phrase("forbidden phrase", MatchMode::Exact);
```

### 6. Secret Detector

Identifies accidentally exposed secrets and credentials.

**Detected secrets:**
- AWS access keys
- GitHub tokens
- OpenAI/Anthropic API keys
- Stripe keys
- Private keys (RSA, EC)
- Database URLs
- JWT tokens
- Generic API keys

## Configuration

### GuardrailsConfig

```rust
pub struct GuardrailsConfig {
    /// Global enable/disable
    pub enabled: bool,

    /// Fail-open or fail-closed on errors
    pub fail_mode: FailMode,

    /// Maximum execution time for all detectors (ms)
    pub timeout_ms: u64,

    /// Individual detector configurations
    pub prompt_injection: DetectorConfig,
    pub pii: PiiConfig,
    pub jailbreak: DetectorConfig,
    pub topics: TopicConfig,
    pub keywords: KeywordConfig,
    pub secrets: DetectorConfig,
}
```

### Detection Sensitivity

```rust
pub enum DetectionSensitivity {
    Low,      // Fewer false positives, may miss some attacks
    Medium,   // Balanced (default)
    High,     // More aggressive, may have false positives
    Custom(f64), // Custom threshold (0.0 - 1.0)
}
```

### Fail Modes

```rust
pub enum FailMode {
    FailClosed,  // Block on errors (safer, default)
    FailOpen,    // Allow through on errors (more permissive)
}
```

## Results

### GuardrailsResult

```rust
pub struct GuardrailsResult {
    /// Whether the scan passed (no threats detected)
    pub passed: bool,

    /// Results from each detector
    pub results: Vec<DetectionResult>,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,

    /// Reason for blocking (if blocked)
    pub blocked_reason: Option<String>,
}
```

### DetectionResult

```rust
pub struct DetectionResult {
    /// Detector that produced this result
    pub detector_name: String,

    /// Whether a threat was detected
    pub detected: bool,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,

    /// Evidence/details about detection
    pub evidence: Vec<String>,

    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}
```

## Performance

The guardrails module is optimized for low latency:

- **Parallel execution**: All detectors run concurrently
- **Regex caching**: Compiled patterns are cached
- **Lazy initialization**: Resources loaded on first use
- **Target latency**: < 50ms for typical inputs

## Testing

```bash
# Run guardrails unit tests
cargo test --package goose guardrails::

# Run integration tests
cargo test --package goose --test guardrails_integration_test
```

## Example: Full Integration

```rust
use goose::guardrails::{
    GuardrailsEngine, GuardrailsConfig, DetectionContext,
    config::{DetectorConfig, DetectionSensitivity, FailMode}
};

#[tokio::main]
async fn main() -> Result<()> {
    // Configure guardrails
    let config = GuardrailsConfig {
        enabled: true,
        fail_mode: FailMode::FailClosed,
        timeout_ms: 5000,
        prompt_injection: DetectorConfig {
            enabled: true,
            sensitivity: DetectionSensitivity::High,
            confidence_threshold: 0.7,
        },
        pii: PiiConfig {
            enabled: true,
            sensitivity: DetectionSensitivity::Medium,
            redact_on_detect: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let engine = GuardrailsEngine::with_config(config);

    // Create context
    let context = DetectionContext {
        session_id: "session-123".to_string(),
        user_id: Some("user-456".to_string()),
        ..Default::default()
    };

    // Scan user input
    let input = "Please process my request";
    let result = engine.scan(input, &context).await?;

    if result.passed {
        // Safe to proceed
        process_input(input).await?;
    } else {
        // Handle blocked input
        log::warn!("Input blocked: {:?}", result.blocked_reason);
        for detection in result.results.iter().filter(|r| r.detected) {
            log::info!("Detected by {}: {:?}", detection.detector_name, detection.evidence);
        }
    }

    Ok(())
}
```

## See Also

- [Enterprise Integration Action Plan](07_ENTERPRISE_INTEGRATION_ACTION_PLAN.md)
- [Comprehensive Audit Report](08_COMPREHENSIVE_AUDIT_REPORT.md)
