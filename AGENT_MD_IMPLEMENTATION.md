# File Reference (@-mention) Support Implementation

This document outlines the implementation of @-mention file reference support for Goose's `.goosehints` configuration files.

## Current Status

- **Phase 1**: ✅ COMPLETED - File reference support (@-mentions) with security features
- **Phase 2**: ✅ COMPLETED - Security hardening against path traversal attacks
- **Phase 3**: 🔲 REMOVED - AGENT.md-specific functionality (existing context file system handles this)

**Branch**: `kh/support-agent-md-files`

## Overview

**Update**: After discovering that Goose already has existing functionality to read AGENT.md files through the `CONTEXT_FILE_NAMES` setting, we simplified this implementation to focus solely on the valuable @-mention file reference functionality.

The goal is to enhance Goose's existing `.goosehints` system with the ability to reference and include other files using `@filename.md` syntax, while maintaining strong security protections.

## Core Implementation

### File Reference System (@-mentions)
- [x] Parse `@filename.md` references in `.goosehints` files using secure regex
- [x] Expand referenced files inline with attribution markers
- [x] Support both global (`~/.config/goose/.goosehints`) and local (`./.goosehints`) files
- [x] Recursive file reference support with circular reference detection
- [x] Respect `.gooseignore` and `.gitignore` patterns for security

### Security Features
- [x] **Path Traversal Protection**: Reject absolute paths and `../` sequences
- [x] **ReDoS Protection**: Input size limits (1MB) to prevent regex DoS attacks
- [x] **Canonical Path Verification**: Ensure files stay within allowed directories
- [x] **Comprehensive Security Testing**: Full test suite for security edge cases

### File Discovery (Existing .goosehints support)
- [x] Global hints: `~/.config/goose/.goosehints`
- [x] Local project hints: `./.goosehints` 
- [x] Both files are processed if they exist
- [x] @-mentions work in both global and local files

### Security Layers

#### 1. Path Sanitization
```rust
/// Sanitize and resolve a file reference path safely
fn sanitize_reference_path(reference: &Path, base_path: &Path) -> Result<PathBuf, FileReferenceError>
```

**Protection against:**
- ❌ `@/etc/passwd` (absolute paths)
- ❌ `@../../../etc/passwd` (path traversal)
- ❌ `@~/secrets.txt` (tilde expansion attacks)
- ❌ Symlink-based escapes

#### 2. Performance Protection
```rust
const MAX_CONTENT_LENGTH: usize = 1_000_000; // 1MB limit
```

**Protection against:**
- ❌ ReDoS (Regular Expression Denial of Service)
- ❌ Memory exhaustion from large inputs
- ❌ Infinite processing loops

### Example Usage

**Global hints** (`~/.config/goose/.goosehints`):
```markdown
These are my global preferences.

@global-coding-standards.md
```

**Local hints** (`./.goosehints`):
```markdown
This is a Rust project with crates in the crate directory.

Project documentation: @README.md
Development setup: @docs/setup.md

Tips:
- Always run `cargo clippy -- -D warnings`
- Check git status before starting work
```

**Referenced file** (`docs/setup.md`):
```markdown
# Development Setup

1. Install Rust toolchain
2. Run `cargo build --all`
3. Install pre-commit hooks
```

**Expanded result in system prompt**:
```markdown
### Global Configuration
These are my global preferences.

--- Content from /home/user/.config/goose/global-coding-standards.md ---
[Content of global-coding-standards.md here]
--- End of /home/user/.config/goose/global-coding-standards.md ---

### Project Configuration
This is a Rust project with crates in the crate directory.

Project documentation: 
--- Content from /current/project/README.md ---
[Content of README.md here]  
--- End of /current/project/README.md ---

Development setup:
--- Content from /current/project/docs/setup.md ---
# Development Setup

1. Install Rust toolchain
2. Run `cargo build --all`
3. Install pre-commit hooks
--- End of /current/project/docs/setup.md ---

Tips:
- Always run `cargo clippy -- -D warnings`
- Check git status before starting work
```

### Testing

- [x] Basic file reference expansion tests
- [x] Security vulnerability tests (path traversal, ReDoS)
- [x] Edge case testing (circular references, missing files)
- [x] Integration tests with real DeveloperRouter
- [x] Performance tests with large inputs

## Removed Functionality

The following AGENT.md-specific features were removed as they duplicate existing Goose functionality:

- ~~AGENT.md file discovery and precedence handling~~
- ~~Hierarchical AGENT.md support (multiple directories)~~
- ~~AGENT.md taking precedence over .goosehints~~

## Security Audit Results

✅ **No Path Traversal Vulnerabilities**  
✅ **No ReDoS Attack Vectors**  
✅ **Proper Input Validation**  
✅ **Canonical Path Verification**  
✅ **Comprehensive Test Coverage**

## Implementation Files

- **Main implementation**: `crates/goose-mcp/src/developer/mod.rs`
  - `sanitize_reference_path()` - Security layer
  - `parse_file_references()` - Regex parsing with protection
  - `read_referenced_files()` - File expansion logic
  - Security error types and comprehensive testing

## Benefits

1. **Enhanced Documentation**: Link to detailed guides and documentation
2. **Modular Configuration**: Break large .goosehints into smaller, focused files
3. **Team Collaboration**: Share common configuration files across projects
4. **Security**: Robust protection against common file inclusion vulnerabilities
5. **Performance**: Optimized regex compilation and DoS protection
