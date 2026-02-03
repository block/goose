# Agentic Guardrails Integration Plan for Goose Enterprise Platform

## Executive Summary

This document outlines the professional integration strategy for incorporating the **Aegis Guardrails Framework** from [agentic-guardrails](https://github.com/FareedKhan-dev/agentic-guardrails) into the Goose Enterprise Platform. The integration will enhance Goose's existing safety infrastructure with a comprehensive 3-layer defense-in-depth approach.

---

## 1. Source Repository Analysis

### 1.1 Repository Structure

```
agentic-guardrails/
├── code.ipynb          # Full implementation notebook (~105KB)
├── README.md           # Comprehensive documentation (~108KB)
├── requirements.txt    # Dependencies: openai, langgraph, sec-edgar-downloader, pandas, pygraphviz
└── LICENSE             # MIT License
```

### 1.2 Core Architecture: Aegis 3-Layer Framework

The agentic-guardrails repository implements a **Defense-in-Depth** guardrail system with three independent layers:

| Layer | Name | Purpose | Goose Mapping |
|-------|------|---------|---------------|
| **Layer 1** | Input Guardrails | Perimeter defense - filters malicious/irrelevant prompts | `approval/` + new module |
| **Layer 2** | Action Plan Guardrails | Intent validation - blocks risky plans before execution | `shell_guard.rs` enhancement |
| **Layer 3** | Output Guardrails | Response sanitization - verifies accuracy/compliance | New `output_guard.rs` |

### 1.3 Key Components to Integrate

#### Layer 1: Asynchronous Input Guardrails
1. **Topical Guardrail** - Domain relevance classifier
2. **Sensitive Data Guardrail** - PII/MNPI detection with regex + redaction
3. **Threat & Compliance Guardrail** - LLM safety model integration (Llama Guard)
4. **Parallel Execution** - asyncio-based concurrent checks

#### Layer 2: Action Plan Guardrails
1. **Plan Generation Node** - Forces agent to output structured action plans
2. **Groundedness Check** - Validates reasoning is grounded in context
3. **AI-Powered Policy Enforcement** - Dynamic guardrail code generation from policy documents
4. **Human-in-the-Loop Trigger** - Escalation for high-risk actions

#### Layer 3: Output Guardrails
1. **Hallucination Detection** - LLM-as-judge for factual verification
2. **Compliance Guardrail** - Regulatory requirement checking
3. **Citation Verification** - Source attribution validation

---

## 2. Integration Strategy

### 2.1 Phase 1: Enhance Existing Approval Module (Layer 1)

**Location:** `crates/goose/src/approval/`

**Current State:** Goose has an approval system with `ApprovalPreset` (Safe, Paranoid, Autopilot).

**Integration Plan:**

```rust
// New file: crates/goose/src/approval/input_guardrails.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use regex::Regex;

/// Aegis Layer 1: Input Guardrails
pub struct InputGuardrails {
    topical_guard: TopicalGuard,
    sensitive_data_guard: SensitiveDataGuard,
    threat_guard: ThreatGuard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputGuardResult {
    pub topic_check: TopicResult,
    pub sensitive_data_check: SensitiveDataResult,
    pub threat_check: ThreatResult,
    pub verdict: GuardVerdict,
    pub redacted_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GuardVerdict {
    Allowed,
    Blocked { reasons: Vec<String> },
}

impl InputGuardrails {
    /// Run all input guardrails in parallel
    pub async fn check(&self, prompt: &str) -> Result<InputGuardResult> {
        let (topic, sensitive, threat) = tokio::join!(
            self.topical_guard.check(prompt),
            self.sensitive_data_guard.scan(prompt),
            self.threat_guard.check(prompt),
        );

        // Aggregate results and determine verdict
        Self::aggregate_results(topic?, sensitive?, threat?)
    }
}
```

**Key Additions:**
- `TopicalGuard` - LLM-based topic classifier
- `SensitiveDataGuard` - PII regex patterns (account numbers, SSN, emails, etc.)
- `ThreatGuard` - Integration with safety models (LlamaGuard, Anthropic safety)

### 2.2 Phase 2: Enhance ShellGuard with Plan Validation (Layer 2)

**Location:** `crates/goose/src/agents/shell_guard.rs`

**Current State:** ShellGuard checks individual commands against approval policies.

**Integration Plan:**

```rust
// Enhanced shell_guard.rs

/// Aegis Layer 2: Action Plan Guardrails
pub struct ActionPlanGuard {
    shell_guard: ShellGuard,
    groundedness_checker: GroundednessChecker,
    policy_enforcer: PolicyEnforcer,
    hitl_trigger: HITLTrigger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub steps: Vec<PlannedAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedAction {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub reasoning: String,
    pub verdict: Option<ActionVerdict>,
}

#[derive(Debug, Clone)]
pub enum ActionVerdict {
    Allowed,
    Blocked { reason: String },
    RequiresApproval { reason: String },
}

impl ActionPlanGuard {
    /// Validate an entire action plan before execution
    pub async fn validate_plan(
        &self,
        plan: &ActionPlan,
        context: &ExecutionContext,
    ) -> Result<Vec<ActionVerdict>> {
        let mut verdicts = Vec::new();

        for action in &plan.steps {
            // 1. Check groundedness
            let grounded = self.groundedness_checker.check(&action.reasoning, context).await?;
            if !grounded.is_grounded {
                verdicts.push(ActionVerdict::Blocked {
                    reason: "Reasoning not grounded in context".to_string()
                });
                continue;
            }

            // 2. Check policy compliance
            let policy_result = self.policy_enforcer.check(action).await?;
            if !policy_result.is_valid {
                verdicts.push(ActionVerdict::Blocked {
                    reason: policy_result.reason
                });
                continue;
            }

            // 3. Check if HITL required
            if self.hitl_trigger.requires_approval(action)? {
                verdicts.push(ActionVerdict::RequiresApproval {
                    reason: "High-risk action requires human approval".to_string()
                });
                continue;
            }

            verdicts.push(ActionVerdict::Allowed);
        }

        Ok(verdicts)
    }
}
```

### 2.3 Phase 3: Add Output Guardrails (Layer 3)

**Location:** `crates/goose/src/agents/output_guard.rs` (new file)

```rust
// New file: crates/goose/src/agents/output_guard.rs

/// Aegis Layer 3: Output Guardrails
pub struct OutputGuard {
    hallucination_detector: HallucinationDetector,
    compliance_checker: ComplianceChecker,
    citation_verifier: CitationVerifier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputGuardResult {
    pub hallucination_check: HallucinationResult,
    pub compliance_check: ComplianceResult,
    pub citation_check: CitationResult,
    pub verdict: OutputVerdict,
    pub sanitized_response: Option<String>,
}

impl OutputGuard {
    /// Validate agent output before returning to user
    pub async fn validate(
        &self,
        response: &str,
        context: &OutputContext,
    ) -> Result<OutputGuardResult> {
        // 1. Check for hallucinations (is response grounded in context/sources?)
        let hallucination = self.hallucination_detector.check(response, &context.sources).await?;

        // 2. Check compliance (no financial advice, no PII, etc.)
        let compliance = self.compliance_checker.check(response, &context.policies).await?;

        // 3. Verify citations (are sources valid and correctly attributed?)
        let citations = self.citation_verifier.verify(response, &context.sources).await?;

        Self::aggregate_results(hallucination, compliance, citations)
    }
}
```

---

## 3. Implementation Roadmap

### Phase 1: Foundation (Week 1-2)

| Task | Priority | Files Affected |
|------|----------|----------------|
| Create `input_guardrails.rs` module | High | `crates/goose/src/approval/` |
| Add PII regex patterns | High | `input_guardrails.rs` |
| Add topical classification | Medium | `input_guardrails.rs` |
| Integrate with existing approval flow | High | `approval/mod.rs` |
| Add tests | High | `tests/input_guardrails_test.rs` |

### Phase 2: Action Plan Validation (Week 2-3)

| Task | Priority | Files Affected |
|------|----------|----------------|
| Add `ActionPlan` types | High | `agents/types.rs` |
| Create `GroundednessChecker` | High | `agents/groundedness.rs` |
| Create `PolicyEnforcer` | High | `agents/policy_enforcer.rs` |
| Enhance `ShellGuard` with plan validation | High | `agents/shell_guard.rs` |
| Add HITL trigger logic | Medium | `agents/hitl.rs` |
| Integration tests | High | `tests/action_plan_test.rs` |

### Phase 3: Output Guardrails (Week 3-4)

| Task | Priority | Files Affected |
|------|----------|----------------|
| Create `output_guard.rs` | High | `agents/output_guard.rs` |
| Add `HallucinationDetector` | High | `agents/hallucination.rs` |
| Add `ComplianceChecker` | Medium | `agents/compliance.rs` |
| Add `CitationVerifier` | Medium | `agents/citation.rs` |
| End-to-end integration | High | `agents/agent.rs` |
| Reality gate tests | High | `tests/reality_gates_e2e_test.rs` |

### Phase 4: Production Hardening (Week 4-5)

| Task | Priority | Files Affected |
|------|----------|----------------|
| Performance optimization | Medium | All guardrail modules |
| Async parallel execution | High | All guardrail modules |
| Metrics/observability | Medium | `agents/observability.rs` |
| Configuration system | Medium | `config/guardrails.rs` |
| Documentation | High | `docs/` |

---

## 4. Mapping to Existing Goose Components

| Agentic Guardrails | Goose Equivalent | Integration Strategy |
|-------------------|------------------|---------------------|
| `check_topic()` | New | Add to `approval/input_guardrails.rs` |
| `scan_for_sensitive_data()` | Partial (PII) | Enhance with more patterns |
| `check_threats()` (Llama Guard) | `ShellGuard` | Extend with LLM safety check |
| `generate_action_plan()` | `PlanManager` | Enhance to output structured plans |
| `check_plan_groundedness()` | New | Add `GroundednessChecker` |
| `validate_trade_action()` | `ApprovalManager` | Add dynamic policy generation |
| `human_in_the_loop_trigger()` | `RequiresApproval` | Enhance existing approval flow |
| `is_response_grounded()` | New | Add `HallucinationDetector` |
| `check_compliance()` | New | Add `ComplianceChecker` |

---

## 5. Configuration Schema

```toml
# goose-config.toml

[guardrails]
enabled = true

[guardrails.layer1]
topical_check = true
allowed_topics = ["software_development", "code_generation", "debugging"]
pii_detection = true
pii_patterns = ["ssn", "credit_card", "account_number", "email", "phone"]
threat_check = true
threat_model = "anthropic_safety"  # or "llama_guard"

[guardrails.layer2]
plan_validation = true
groundedness_check = true
policy_enforcement = true
policy_file = "policies/enterprise_policy.txt"
hitl_threshold = "high_risk"

[guardrails.layer3]
output_validation = true
hallucination_check = true
compliance_check = true
citation_verification = false
```

---

## 6. Testing Strategy

### Unit Tests
- Each guardrail component tested independently
- Mock LLM responses for deterministic testing
- Edge cases: empty prompts, unicode, injection attempts

### Integration Tests
- Full pipeline tests with all three layers
- Parallel execution verification
- Latency benchmarks

### Reality Gate Tests (Already Implemented)
- Gate 5: ShellGuard blocks dangerous commands
- New Gate 7: Input guardrails block malicious prompts
- New Gate 8: Output guardrails catch hallucinations

### Red Team Testing
- Prompt injection attempts
- Jailbreak attempts
- PII leakage attempts
- Policy bypass attempts

---

## 7. Security Considerations

1. **LLM Safety Model Selection**: Support multiple safety models (Llama Guard, Anthropic safety, custom)
2. **Regex Pattern Security**: Ensure patterns don't cause ReDoS vulnerabilities
3. **Policy Document Security**: Validate and sandbox dynamically generated guardrail code
4. **Data Handling**: Ensure redacted data is not logged or transmitted
5. **Audit Trail**: Log all guardrail decisions for compliance

---

## 8. Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Layer 1 latency | < 500ms | Parallel execution |
| Layer 2 latency | < 1s | Per action validation |
| Layer 3 latency | < 500ms | Output validation |
| Total overhead | < 2s | Full pipeline |
| Memory overhead | < 50MB | Guardrail state |

---

## 9. Success Criteria

- [ ] All existing 672 tests pass
- [ ] 6 Reality Gates pass
- [ ] New guardrail tests pass (Layer 1, 2, 3)
- [ ] Clippy zero warnings
- [ ] Documentation complete
- [ ] Performance targets met
- [ ] Red team testing passed

---

## 10. References

- **Source Repository**: https://github.com/FareedKhan-dev/agentic-guardrails
- **LangGraph Documentation**: https://langchain-ai.github.io/langgraph/
- **Llama Guard Paper**: https://ai.meta.com/research/publications/llama-guard/
- **Goose Approval Module**: `crates/goose/src/approval/`
- **Goose ShellGuard**: `crates/goose/src/agents/shell_guard.rs`

---

## Appendix A: PII Regex Patterns

```rust
pub const PII_PATTERNS: &[(&str, &str)] = &[
    ("account_number", r"\b(ACCT|ACCOUNT)[- ]?(\d{3}[- ]?){2}\d{4}\b"),
    ("ssn", r"\b\d{3}[-]?\d{2}[-]?\d{4}\b"),
    ("credit_card", r"\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b"),
    ("email", r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b"),
    ("phone", r"\b(\+1)?[-.\s]?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b"),
    ("ip_address", r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b"),
    ("api_key", r"\b(sk|pk|api)[_-][A-Za-z0-9]{20,}\b"),
];
```

## Appendix B: MNPI Keywords

```rust
pub const MNPI_KEYWORDS: &[&str] = &[
    "insider info",
    "upcoming merger",
    "unannounced earnings",
    "confidential partnership",
    "material non-public",
    "before announcement",
    "leaked document",
    "internal memo",
];
```
