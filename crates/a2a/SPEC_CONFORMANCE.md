# A2A Spec Conformance Report

Comparison of `crates/a2a` Rust implementation against:
- **Proto spec**: `A2A/specification/a2a.proto` (v1.0, authoritative)
- **JS SDK**: `@anthropic-ai/a2a-sdk` v0.3.10

## Status: ✅ Functional, with known deviations

### What's correct ✅

| Feature | Status |
|---------|--------|
| All 11 JSON-RPC methods | ✅ Covered |
| All 9 TaskState values | ✅ ProtoJSON SCREAMING_SNAKE_CASE + legacy lowercase compat |
| Role enum (User, Agent) | ✅ Dual-format deser |
| `"unknown"` TaskState (JS SDK compat) | ✅ Maps to Unspecified |
| All 12 error codes (-32700 to -32009) | ✅ Including 2 extras |
| Message (all 8 fields) | ✅ No `kind` field (correct per proto) |
| Task (all 6 fields) | ✅ No `kind` field (correct per proto) |
| TaskStatus | ✅ All fields match |
| Artifact | ✅ All fields match |
| AgentSkill | ✅ All fields match |
| SecurityScheme (5 variants) | ✅ Tagged enum |
| AgentInterface | ✅ All fields |
| AgentCard | ✅ All proto fields present |
| AgentProvider | ✅ |
| SSE streaming | ✅ Via axum |
| JSON-RPC 2.0 framing | ✅ |
| PushNotificationConfig CRUD | ✅ All 4 RPCs |

### Known deviations (tracked for future PRs)

#### Part discriminator: `type` field vs proto `oneof`

Proto uses `oneof content { text, raw, url, data }` — in ProtoJSON this means
mutually exclusive fields with no discriminator. Rust uses `#[serde(tag = "type")]`
adding a `"type": "text"` discriminator. JS SDK uses `kind: "text"`.

**Impact**: Rust↔Rust works. Rust↔JS needs adapter. Neither matches pure ProtoJSON.
**Decision**: Keep `type` tag for ergonomic Rust deser. Document deviation.

#### Part missing `filename` and `media_type`

Proto has `string filename = 6` and `string media_type = 7` on Part.
Both Rust and JS SDK omit these (v0.3.0 legacy).

**Fix**: Add `Option<String>` fields with `#[serde(skip_serializing_if)]`.

#### AgentCapabilities.extensions type

Proto: `repeated AgentExtension extensions = 3`
Rust: `extensions: bool`

**Impact**: Rust can't declare supported extensions to JS clients.
**Fix**: Change to `Vec<AgentExtension>` (breaking change, needs test updates).

#### AgentCard required vs optional fields

Proto3 fields are implicitly optional but conceptually required per spec docs.
Rust uses `Option<String>` for `version`, `protocol_version`, `capabilities`.
Proto marks these as required in the spec documentation.

**Decision**: Keep `Option` for lenient deserialization of incomplete agent cards
discovered in the wild. Validate completeness at a higher level.

#### SecurityScheme/SecurityRequirement typing

`security_schemes` is `serde_json::Value` instead of `HashMap<String, SecurityScheme>`.
`security` is `Vec<Value>` instead of `Vec<SecurityRequirement>`.

**Fix**: Type these properly (non-breaking if using `#[serde(default)]`).

### Not in scope

- REST transport (HTTP+JSON without JSON-RPC): Not planned
- gRPC transport: Not planned (JSON-RPC + SSE sufficient for goose)
- `.well-known/agent-card.json` discovery: Implemented in server routes

### JS SDK differences (not bugs)

The JS SDK v0.3.10 has several v0.3.0 holdovers that don't match the v1.0 proto:
- `kind: "message"` / `kind: "task"` discriminators (not in proto)
- `url` and `preferredTransport` on AgentCard (moved to AgentInterface in v1.0)
- `stateTransitionHistory` on AgentCapabilities (removed in v1.0)
- Lowercase-only enum values (proto mandates SCREAMING_SNAKE_CASE per ADR-001)

The Rust crate correctly follows the v1.0 proto spec while maintaining backward
compatibility with JS SDK via legacy deserialization aliases.
