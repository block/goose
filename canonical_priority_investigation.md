# Investigation: Priority Order for Canonical Model Data

## Current Problem
Code duplication in 3 places:
1. Every provider's `from_env()` does canonical lookup
2. ACP agent does canonical lookup
3. They all recreate ModelConfig instead of using what was passed

## Current Priority Order (what we designed)
1. **Environment variables** (`GOOSE_CONTEXT_LIMIT`, `GOOSE_MAX_TOKENS`)
2. **Canonical model data** (if provider/model match)
3. **Hardcoded limits** (`MODEL_SPECIFIC_LIMITS`)
4. **Global defaults** (128k input, 4k output)

## Proposed Solution
Add `provider_name` to `ModelConfig`, do canonical lookup automatically in `ModelConfig::new()`.

## Key Question: Where do explicitly-passed values fit?

### Option A: Passed values OVERRIDE canonical (Recommended)
```rust
// Priority: Env vars > Passed values > Canonical > Hardcoded > Defaults
let config = ModelConfig::new("gpt-4")
    .with_provider("openai")
    .with_context_limit(Some(50_000));  // This overrides canonical
```

**Pros:**
- Consistent with existing `with_*` methods (they always override)
- Gives users control to override canonical if it's wrong
- Intuitive: explicit > implicit
- Matches env var pattern: user knows better than system

**Cons:**
- Canonical data could be more accurate than user's guess

### Option B: Passed values are BACKUPS (fallback if no canonical)
```rust
// Priority: Env vars > Canonical > Passed values > Hardcoded > Defaults
let config = ModelConfig::new("gpt-4")
    .with_provider("openai")
    .with_context_limit(Some(50_000));  // Only used if canonical not found
```

**Pros:**
- Canonical data is "source of truth" when available
- Prevents users from accidentally overriding correct values

**Cons:**
- Inconsistent with existing `with_*` methods
- Less control for users
- If canonical is wrong, can't override without env vars

## Evidence from Codebase

1. **Existing `with_*` methods always override:**
```rust
pub fn with_context_limit(mut self, limit: Option<usize>) -> Self {
    if limit.is_some() {
        self.context_limit = limit;  // Unconditional override
    }
    self
}
```

2. **Current usage pattern suggests values are overrides:**
```rust
let model = ModelConfig::from_canonical(&model.model_name, canonical_id)?
    .with_fast(ANTHROPIC_DEFAULT_FAST_MODEL.to_string());
    // with_fast() is an override, not a backup
```

## Recommendation: **Option A** (Passed values override canonical)

**Reasoning:**
1. Consistent with existing API patterns
2. Users need ability to override if canonical is wrong
3. More intuitive: `with_context_limit()` sounds like "set this value"
4. Environment variables already work this way

**Implementation would be:**
```rust
impl ModelConfig {
    pub fn new(model_name: &str) -> Result<Self, ConfigError> {
        // 1. Parse env vars first (highest priority)
        let context_limit = Self::parse_context_limit(...)?;
        let max_tokens = Self::parse_max_tokens()?;

        // 2. If no env vars, try canonical (if provider is known)
        // 3. If no canonical, try hardcoded limits
        // 4. Fall back to global defaults

        Ok(Self { ... })
    }

    pub fn with_provider(mut self, provider: &str) -> Self {
        // Re-check canonical with provider name if limits not already set
        if self.context_limit.is_none() && !had_env_var {
            // Try canonical lookup
        }
        self.provider_name = Some(provider.to_string());
        self
    }
}
```

## Alternative: Two Constructors

Could have both patterns:
- `ModelConfig::new()` - canonical overrides defaults
- `ModelConfig::with_canonical_fallback()` - canonical is backup

But this adds complexity for little benefit.
