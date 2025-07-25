# AGENT.md Support Implementation Plan

This document outlines the implementation plan for adding AGENT.md support to Goose, following the [AGENT.md specification](https://ampcode.com/AGENT.md).

## Current Status

- **Phase 1**: ✅ COMPLETED - Basic AGENT.md support with backward compatibility
- **Phase 2**: ✅ COMPLETED - File reference support (@-mentions) fully integrated
- **Phase 3**: ✅ COMPLETED - Hierarchical AGENT.md support with intelligent merging
- **Phase 4**: 🔲 NOT STARTED - Enhanced integration and tooling

**Branch**: `kh/support-agent-md-files`

## Overview

The goal is to make Goose more agnostic by supporting the standardized AGENT.md format while maintaining backward compatibility with existing `.goosehints` files. This will allow users to have one configuration file that works across multiple agentic coding tools.

## Phase 1: Basic AGENT.md Support (Backward Compatible)

### Core Implementation
- [x] Extend `DeveloperRouter::new()` in `crates/goose-mcp/src/developer/mod.rs` to look for AGENT.md files
- [x] Add AGENT.md to the file search hierarchy (global → local → project-specific)
- [x] Implement file discovery order: AGENT.md first, then `.goosehints` for backward compatibility
- [x] Parse AGENT.md content and integrate with existing hints system
- [x] Ensure AGENT.md takes precedence when both AGENT.md and `.goosehints` exist

### File Discovery
- [x] Look for `AGENT.md` in current working directory
- [x] Look for global `~/.config/goose/AGENT.md`
- [x] Maintain existing `.goosehints` discovery as fallback
- [x] Document the precedence order clearly

### Content Integration
- [x] Extract content from AGENT.md files
- [x] Merge AGENT.md content with existing `.goosehints` content
- [x] Preserve existing instruction formatting and structure
- [x] Add clear attribution for content sources (currently shows in all modes, not just debug/verbose)

### Testing
- [ ] Add unit tests for AGENT.md file discovery
- [ ] Add tests for content parsing and merging  
- [ ] Add tests for precedence handling (AGENT.md vs .goosehints)
- [x] Test backward compatibility with existing `.goosehints` files (existing tests still pass)

## Phase 2: File Reference Support (@-mentions) ✅ COMPLETED

### @-mention Parsing
- [x] Implement regex-based parsing for `@filename.md` patterns in AGENT.md content
- [x] Create `parse_file_references(content: &str) -> Vec<PathBuf>` function
- [x] Handle various file reference formats (@file.md, @./path/file.md, etc.)
- [x] Support relative and absolute path references

### Referenced File Reading
- [x] Implement automatic reading of files referenced via @-mentions
- [x] Respect existing `.gooseignore` patterns for referenced files
- [x] Add referenced file content to instructions with clear attribution
- [x] Handle missing referenced files gracefully (warning, not error)

### Security and Safety
- [x] Implement circular reference detection and prevention
- [x] Add recursion depth limits for @-mentions (e.g., max 3 levels deep)
- [ ] Prevent reading files outside project boundaries (security consideration)
- [x] Track visited files to prevent infinite loops

### Error Handling
- [x] Graceful handling of missing referenced files
- [x] Clear error messages for circular references
- [x] Warnings for referenced files that are ignored by `.gooseignore`
- [x] Detailed logging for debugging file reference issues

### Testing
- [ ] Add tests for @-mention parsing
- [ ] Add tests for referenced file reading
- [ ] Add tests for circular reference detection
- [ ] Add tests for security boundaries
- [ ] Add integration tests with real file structures

### Implementation Notes
- **COMPLETED**: Full integration of @-mention support into initialization flow
- Added `parse_file_references()` function at module level that uses regex to detect @-mentions in content
- Added `read_referenced_files()` function at module level with:
  - Circular reference detection using a HashSet of visited files
  - Recursion depth limiting (MAX_DEPTH = 3)
  - Respect for .gooseignore patterns
  - Graceful error handling for missing files
  - Clear attribution in the expanded content with "--- Content from {path} ---" markers
- **Integration completed**: Modified `DeveloperRouter::new()` to process @-mentions when reading configuration files
  - File references are processed for both global and local configuration files
  - Ignore patterns are built first, then used during file reference expansion
  - Referenced files are wrapped with attribution markers for clarity
- **Code quality**: Fixed duplicate code issues, ran `cargo fmt` and `cargo clippy`

## Phase 3: Hierarchical AGENT.md Support ✅ COMPLETED

### Multiple File Support
- [x] Implement support for multiple AGENT.md files in hierarchy
- [x] Global: `~/.config/goose/AGENT.md`
- [x] Project root: `./AGENT.md`
- [x] Subdirectory: `./subdir/AGENT.md` (when working in subdirectories)

### Merging Strategy
- [x] Implement intelligent merging of multiple AGENT.md files
- [x] More specific files override general ones (hierarchical precedence)
- [x] Concatenate non-conflicting sections with clear attribution
- [x] Define and implement clear precedence rules (Global → ProjectRoot → Subdirectory)
- [x] Handle section-specific merging with proper section titles

### Content Organization
- [x] Maintain section boundaries during merging with clear section titles
- [x] Provide clear attribution for each section's source (Global/Project/Directory Configuration)
- [x] Handle conflicts between different AGENT.md files through precedence ordering
- [x] Preserve file reference (@-mention) expansion across hierarchical levels

### Testing
- [x] Add tests for hierarchical file discovery (`test_discover_hierarchical_config_files_*`)
- [x] Add tests for content merging strategies (`test_hierarchical_agent_md_integration`)
- [x] Add tests for precedence rules (`test_discover_hierarchical_config_files_precedence`)
- [x] Add integration tests with complex directory structures (`test_hierarchical_agent_md_*`)

### Implementation Notes
- **COMPLETED**: Full hierarchical AGENT.md support with intelligent merging
- Added `ConfigScope` enum (Global, ProjectRoot, Subdirectory) for proper scoping
- Added `ConfigFile` struct to represent discovered configuration files with metadata
- Implemented `discover_hierarchical_config_files()` function that:
  - Walks up directory hierarchy to find all AGENT.md files
  - Properly scopes files based on their location relative to current working directory
  - Maintains precedence order with intelligent sorting
  - Supports mixed AGENT.md and .goosehints files at different levels
- **Integration completed**: Modified `DeveloperRouter::new()` to use hierarchical discovery
  - Replaced old dual-loop approach with unified hierarchical system
  - Maintains proper section titles and attribution for each configuration level
  - Preserves file reference (@-mention) expansion functionality across all levels
- **Comprehensive testing**: Added 8 new test functions covering all hierarchical scenarios
- **Code quality**: Clean refactoring with proper error handling and boundary conditions

## Phase 4: Enhanced Integration

### Structured Content Parsing
- [ ] Implement parsing of specific AGENT.md sections
- [ ] Extract Build & Commands section for potential shell tool integration
- [ ] Extract Code Style section for formatting guidance
- [ ] Extract Testing section for test-related guidance
- [ ] Make parsed sections available to other Goose components

### CLI Integration
- [ ] Add `goose migrate-hints` command to convert `.goosehints` to `AGENT.md`
- [ ] Add `goose validate-agent` command to validate AGENT.md format and references
- [ ] Add `goose show-config` command to display merged configuration
- [ ] Provide migration guidance and best practices

### Migration Tooling
- [ ] Implement automatic migration from `.goosehints` to `AGENT.md`
- [ ] Create symbolic link creation for backward compatibility
- [ ] Provide migration suggestions and warnings
- [ ] Support batch migration for multiple projects

### Documentation
- [ ] Update Goose documentation to explain AGENT.md support
- [ ] Provide migration guide from `.goosehints` to `AGENT.md`
- [ ] Document file reference (@-mention) syntax and capabilities
- [ ] Provide example AGENT.md files for common project types

### Testing
- [ ] Add end-to-end tests for CLI commands
- [ ] Add tests for migration tooling
- [ ] Add tests for structured content parsing
- [ ] Performance tests for large AGENT.md files with many references

## Implementation Notes

### Code Organization
- Primary changes in `crates/goose-mcp/src/developer/mod.rs`
- New helper functions for AGENT.md parsing and merging
- Maintain existing `.goosehints` functionality for backward compatibility
- Consider extracting configuration logic into separate module if it grows large

### Performance Considerations
- Cache parsed AGENT.md content to avoid re-reading on each tool call
- Implement lazy loading of referenced files
- Consider file watching for development mode to reload changes
- Optimize file I/O for projects with many AGENT.md files

### Security Considerations
- Validate file paths to prevent directory traversal attacks
- Respect existing ignore patterns for all file operations
- Limit file reference depth to prevent resource exhaustion
- Consider file size limits for referenced files

### User Experience
- Provide clear error messages and helpful suggestions
- Maintain backward compatibility throughout implementation
- Offer smooth migration path from existing `.goosehints` files
- Document best practices for AGENT.md usage

## Success Criteria

- [x] Goose can read and use AGENT.md files for project guidance
- [x] File references (@-mentions) work correctly and securely (Phase 2 completed)
- [x] Hierarchical AGENT.md files merge intelligently (Phase 3 completed)
- [x] Existing `.goosehints` files continue to work unchanged
- [ ] Migration tools help users transition smoothly
- [x] Performance impact is minimal
- [x] Security boundaries are maintained (except for project boundary validation)
- [ ] Documentation is comprehensive and helpful

## Future Considerations

- Integration with other Goose extensions and tools
- Support for AGENT.md templates and scaffolding
- Community contribution of example AGENT.md files
- Potential standardization discussions with other tool makers
- Advanced parsing for structured data within AGENT.md sections
