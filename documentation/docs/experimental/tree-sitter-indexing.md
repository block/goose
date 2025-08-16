
# Goose CLI Tree-sitter Code Indexing (Experimental)

## Overview

Goose CLI provides experimental code indexing using Tree-sitter for a wide range of programming languages. This enables fast, accurate parsing and mapping of source code repositories for search, navigation, and analysis features. The indexer extracts classes/types, functions/methods, parent relationships, docstrings/comments, and (for most languages) function call relationships, with richer extraction for Go, Python, and C++.

## Tree-sitter Version Compatibility



**Important:** Goose CLI currently uses `tree-sitter` 0.20.x and compatible grammar crate versions. Many language grammar crates on crates.io (notably JavaScript, TypeScript, Go, C#, etc.) do not yet support `tree-sitter` 0.23.x or newer. Attempting to use newer versions results in dependency conflicts due to incompatible requirements for the `cc` crate and the native `tree-sitter` library.

- The following grammar crates are included for maximum language support (as of this release):
  - tree-sitter = "0.20.10"
  - tree-sitter-c-sharp = "0.20.0"
  - tree-sitter-cpp = "0.20.0"
  - tree-sitter-go = "0.20.0"
  - tree-sitter-java = "0.20.0"
  - tree-sitter-javascript = "0.20.0"
  - tree-sitter-python = "0.20.0"
  - tree-sitter-rust = "0.20.0"
  - tree-sitter-swift = "0.2.0"
  - tree-sitter-typescript = "0.20.0"

When newer grammar crates are published with 0.23.x+ support, these can be updated for improved features and bugfixes.

## Upgrading Guidance

- Check crates.io for new releases of grammar crates.
- All grammar crates and the core `tree-sitter` crate must be compatible with each other and with the same version of the `cc` crate.
- If you encounter build errors related to `cc` or native library conflicts, review the version requirements for all grammar crates.

## Status

This feature is experimental and subject to change as the Rust Tree-sitter ecosystem evolves.


## Supported Languages and Extracted Entities

The indexer supports the following languages and extracts the listed entities and relationships:

### JavaScript and TypeScript
- **Classes**: Name, parent (if any), location
- **Functions**: Top-level and class methods, with name, signature, parent (if any), location
- **Function calls**: All function call relationships within functions/methods
- **Doc comments**: Leading JSDoc or TypeScript doc comments (where available)

### Python
- **Classes**: Name, location, docstring
- **Functions**: Name, signature, parent (if any), location, docstring
- **Decorators**: For classes and functions
- **Function calls**: All function call relationships within functions/methods

### Rust
- **Structs, Enums, Traits**: Name, location, doc comments
- **Functions**: Name, signature, parent (struct/enum/trait/impl), location, doc comments
- **Function calls**: All function call relationships within functions/methods

### C++
- **Classes**: Name, location
- **Templates**: Template declarations
- **Functions**: Name, signature, parent (if any), location
- **Function calls**: All function call relationships within functions/methods

### Go
- **Types**: Structs, interfaces, enums, generics
- **Fields**: For structs/interfaces/enums
- **Functions**: Name, signature, parent (if any), location, generics
- **Variables**: Top-level variables
- **Imports**: Import paths
- **Function calls**: All function call relationships within functions/methods

### Java, C#, Swift
- **Classes/Structs/Protocols**: Name, location
- **Functions/Methods**: Name, signature, parent (if any), location
- **Function calls**: All function call relationships within functions/methods

For all supported languages, the output for each entity includes fields such as `file`, `language`, `type`, `name`, `signature`, `startingLineNumber`, `endingLineNumber`, `parent`, `doc`, and `calls` (where applicable). This enables downstream tools to provide code navigation, search, and analysis features for a wide range of projects.

> **Note:** The entity extraction logic is designed to be extensible. If you encounter edge cases or want to extract additional metadata (such as variable declarations, imports, or more relationships), contributions and suggestions are welcome.

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
The index is suitable for a solid LLM-based coding assistant for navigation and entity-level understanding, but not yet for advanced reasoning or deep semantic analysis. Expanding entity coverage and extracting more relationships (such as cross-file references, call graphs, and additional entity types) will further improve LLM capabilities.
