# Goose CLI Tree-sitter Code Indexing (Experimental)

## Overview

Goose CLI supports experimental code indexing using Tree-sitter for a wide range of programming languages. This enables fast, accurate parsing and mapping of source code repositories for search, navigation, and analysis features.

## Tree-sitter Version Compatibility

**Important:** We are currently restricted to using `tree-sitter` 0.22.x and compatible grammar crate versions. This is because many language grammar crates on crates.io (notably JavaScript, TypeScript, Go, C#, etc.) do not yet support `tree-sitter` 0.23.x or newer. Attempting to use newer versions results in dependency conflicts due to incompatible requirements for the `cc` crate and the native `tree-sitter` library.

- The following grammar crates are included for maximum language support:
  - tree-sitter = "0.22.6"
  - tree-sitter-c-sharp = "0.21.3"
  - tree-sitter-cpp = "0.22.2"
  - tree-sitter-go = "0.21.0"
  - tree-sitter-java = "0.20.2"
  - tree-sitter-javascript = "0.20.2"
  - tree-sitter-python = "0.20.4"
  - tree-sitter-rust = "0.20.4"
  - tree-sitter-swift = "0.2.0"
  - tree-sitter-typescript = "0.20.1"

When newer grammar crates are published with 0.23.x+ support, these can be updated for improved features and bugfixes.

## Upgrading Guidance

- Check crates.io for new releases of grammar crates.
- All grammar crates and the core `tree-sitter` crate must be compatible with each other and with the same version of the `cc` crate.
- If you encounter build errors related to `cc` or native library conflicts, review the version requirements for all grammar crates.

## Status

This feature is experimental and subject to change as the Rust Tree-sitter ecosystem evolves.

## Supported Languages

### JavaScript and TypeScript

The indexer supports JavaScript (`.js`, `.jsx`) and TypeScript (`.ts`, `.tsx`) source files using the corresponding Tree-sitter grammar crates. For each file, the indexer extracts:

- **Classes**: Including class name, parent (if any), and location.
- **Functions**: Both top-level and class methods, with name, signature, parent (if any), and location.
- **Doc comments**: Where available, leading JSDoc or TypeScript doc comments are included in the metadata.

The output for each entity includes fields such as `file`, `language`, `type`, `name`, `signature`, `line`, `parent`, and `doc`. This enables downstream tools to provide code navigation, search, and analysis features for JavaScript and TypeScript projects.

> **Note:** The entity extraction logic is designed to be extensible. If you encounter edge cases or want to extract additional metadata (such as variable declarations, imports, or function calls), contributions and suggestions are welcome.

## Note on LLM Coding Assistant Usefulness

The current Tree-sitter-based indexer provides a strong foundation for LLM-based coding assistants, enabling entity-level navigation, search, and summarization for a wide range of languages. For each supported language, the index includes classes/types, functions/methods, and parent relationships, with richer extraction for Go (types, fields, variables, imports, generics), Python (decorators, docstrings), and C++ (templates).

**Limitations:**
- For most languages, only classes/types and functions/methods are extracted. Fields, variables, imports, enum variants, properties, and other constructs are not yet indexed (except in Go).
- Cross-file relationships (such as imports and module dependencies) are not extracted for most languages.
- The index does not include code usage, call graphs, or reference information.

**Implication:**
- For basic LLM code navigation, search, and summarization, the current index is sufficient and competitive with most open-source code indexers.
- For advanced code understanding (e.g., deep semantic analysis, refactoring, dependency analysis), further feature parity and relationship extraction will be needed.

**Summary:**
The index is suitable for a solid LLM-based coding assistant for navigation and entity-level understanding, but not yet for advanced reasoning or deep semantic analysis. Expanding entity coverage and extracting more relationships will further improve LLM capabilities.
