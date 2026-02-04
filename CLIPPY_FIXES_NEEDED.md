# Clippy Fixes Needed for CI

**Status:** CI failing with 33 clippy errors
**Issue:** Clippy warnings treated as errors with `-D warnings` flag

---

## Issues to Fix

### 1. Large Enum Variant (slash_commands.rs:154)
```rust
// Current:
pub enum ParsedCommand {
    Recipe {
        path: PathBuf,
        recipe: Recipe,  // 544 bytes - too large
        args: Vec<String>,
    },
    // ...
}

// Fix: Box the large Recipe
pub enum ParsedCommand {
    Recipe {
        path: PathBuf,
        recipe: Box<Recipe>,  // ✅ Box it
        args: Vec<String>,
    },
    // ...
}
```

### 2. String Indexing (slash_commands.rs:218)
```rust
// Current:
let parts: Vec<&str> = trimmed[1..].split_whitespace().collect();

// Fix: Use strip_prefix
let parts: Vec<&str> = trimmed
    .strip_prefix('/')
    .unwrap_or(trimmed)
    .split_whitespace()
    .collect();
```

### 3. Map-Flatten (tasks/persistence.rs:161,166)
```rust
// Current:
.map(|o| serde_json::to_string(o).ok())
.flatten();

// Fix: Use and_then
.and_then(|o| serde_json::to_string(o).ok())
```

### 4. PathBuf Reference (validators/security.rs:134)
```rust
// Current:
fn scan_content(&self, content: &str, file_path: &PathBuf) -> Vec<ValidationIssue>

// Fix: Use &Path
fn scan_content(&self, content: &str, file_path: &Path) -> Vec<ValidationIssue>
// And change usages:
file: Some(file_path.to_path_buf())
```

### 5. Noop Clone (providers/routing/router.rs:123)
```rust
// Current:
let provider_config = ProviderConfig::new(provider.clone(), endpoint_id, model.clone());

// Fix: Remove .clone() on &str
let provider_config = ProviderConfig::new(provider, endpoint_id, model);
```

---

## Quick Fix Command

```bash
cd C:\Users\Admin\Downloads\projects\goose
cargo clippy --fix --lib --allow-dirty --allow-staged
```

This will automatically fix most issues.

---

## Manual Fixes Required

After running `cargo clippy --fix`, you may need to manually:

1. Box the Recipe field in ParsedCommand enum
2. Update all usages of ParsedCommand::Recipe to use Box::new()

---

## Verification

After fixes:
```bash
cargo clippy --lib -- -D warnings
cargo test --lib memory --features memory
git add -A
git commit -m "fix: Resolve clippy warnings for CI

- Box large Recipe field in ParsedCommand enum
- Use strip_prefix instead of string indexing
- Replace map().flatten() with and_then()
- Use &Path instead of &PathBuf in security validator
- Remove noop clone() calls on &str

All clippy warnings resolved. CI should pass now."
git push fork main
```
