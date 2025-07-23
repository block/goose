# AGENT.md Support Implementation Plan

This document outlines the implementation plan for adding AGENT.md support to Goose, following the [AGENT.md specification](https://ampcode.com/AGENT.md).

## Overview

The goal is to make Goose more agnostic by supporting the standardized AGENT.md format while maintaining backward compatibility with existing `.goosehints` files. This will allow users to have one configuration file that works across multiple agentic coding tools.

## Phase 1: Basic AGENT.md Support (Backward Compatible)

### Core Implementation
- [ ] Extend `DeveloperRouter::new()` in `crates/goose-mcp/src/developer/mod.rs` to look for AGENT.md files
- [ ] Add AGENT.md to the file search hierarchy (global → local → project-specific)
- [ ] Implement file discovery order: AGENT.md first, then `.goosehints` for backward compatibility
- [ ] Parse AGENT.md content and integrate with existing hints system
- [ ] Ensure AGENT.md takes precedence when both AGENT.md and `.goosehints` exist

### File Discovery
- [ ] Look for `AGENT.md` in current working directory
- [ ] Look for global `~/.config/goose/AGENT.md`
- [ ] Maintain existing `.goosehints` discovery as fallback
- [ ] Document the precedence order clearly

### Content Integration
- [ ] Extract content from AGENT.md files
- [ ] Merge AGENT.md content with existing `.goosehints` content
- [ ] Preserve existing instruction formatting and structure
- [ ] Add clear attribution for content sources in debug/verbose modes

### Testing
- [ ] Add unit tests for AGENT.md file discovery
- [ ] Add tests for content parsing and merging
- [ ] Add tests for precedence handling (AGENT.md vs .goosehints)
- [ ] Test backward compatibility with existing `.goosehints` files

## Phase 2: File Reference Support (@-mentions)

### @-mention Parsing
- [ ] Implement regex-based parsing for `@filename.md` patterns in AGENT.md content
- [ ] Create `parse_file_references(content: &str) -> Vec<PathBuf>` function
- [ ] Handle various file reference formats (@file.md, @./path/file.md, etc.)
- [ ] Support relative and absolute path references

### Referenced File Reading
- [ ] Implement automatic reading of files referenced via @-mentions
- [ ] Respect existing `.gooseignore` patterns for referenced files
- [ ] Add referenced file content to instructions with clear attribution
- [ ] Handle missing referenced files gracefully (warning, not error)

### Security and Safety
- [ ] Implement circular reference detection and prevention
- [ ] Add recursion depth limits for @-mentions (e.g., max 3 levels deep)
- [ ] Prevent reading files outside project boundaries (security consideration)
- [ ] Track visited files to prevent infinite loops

### Error Handling
- [ ] Graceful handling of missing referenced files
- [ ] Clear error messages for circular references
- [ ] Warnings for referenced files that are ignored by `.gooseignore`
- [ ] Detailed logging for debugging file reference issues

### Testing
- [ ] Add tests for @-mention parsing
- [ ] Add tests for referenced file reading
- [ ] Add tests for circular reference detection
- [ ] Add tests for security boundaries
- [ ] Add integration tests with real file structures

## Phase 3: Hierarchical AGENT.md Support

### Multiple File Support
- [ ] Implement support for multiple AGENT.md files in hierarchy
- [ ] Global: `~/.config/goose/AGENT.md`
- [ ] Project root: `./AGENT.md`
- [ ] Subdirectory: `./subdir/AGENT.md` (when working in subdirectories)

### Merging Strategy
- [ ] Implement intelligent merging of multiple AGENT.md files
- [ ] More specific files override general ones
- [ ] Concatenate non-conflicting sections
- [ ] Define and implement clear precedence rules
- [ ] Handle section-specific merging (e.g., build commands vs code style)

### Content Organization
- [ ] Parse structured sections from AGENT.md (Build & Commands, Code Style, etc.)
- [ ] Maintain section boundaries during merging
- [ ] Provide clear attribution for each section's source
- [ ] Handle conflicts between different AGENT.md files

### Testing
- [ ] Add tests for hierarchical file discovery
- [ ] Add tests for content merging strategies
- [ ] Add tests for precedence rules
- [ ] Add integration tests with complex directory structures

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

- [ ] Goose can read and use AGENT.md files for project guidance
- [ ] File references (@-mentions) work correctly and securely
- [ ] Hierarchical AGENT.md files merge intelligently
- [ ] Existing `.goosehints` files continue to work unchanged
- [ ] Migration tools help users transition smoothly
- [ ] Performance impact is minimal
- [ ] Security boundaries are maintained
- [ ] Documentation is comprehensive and helpful

## Future Considerations

- Integration with other Goose extensions and tools
- Support for AGENT.md templates and scaffolding
- Community contribution of example AGENT.md files
- Potential standardization discussions with other tool makers
- Advanced parsing for structured data within AGENT.md sections
