---
date: 2025-11-16T15:27:56+0000
researcher: jtennant
git_commit: 5066a09c77a79ddc9b30113adb1c7a9e162f0a0f
branch: main
repository: goose
topic: "How does goose load the agents.md file and subdirectory behavior"
tags: [research, codebase, agents-md, hints, configuration, path-resolution]
status: complete
last_updated: 2025-11-16
last_updated_by: jtennant
---

# Research: How does goose load the agents.md file and subdirectory behavior

**Date**: 2025-11-16T15:27:56+0000
**Researcher**: jtennant
**Git Commit**: 5066a09c77a79ddc9b30113adb1c7a9e162f0a0f
**Branch**: main
**Repository**: goose

## Research Question

1. How does goose load the agents.md file?
2. If I run goose from the repo root and then goose accesses a file in a subdirectory, will it read the subdirectory/agents.md file?

## Summary

**Answer to Question 1:** Goose loads agents.md files through a hierarchical search mechanism that walks **UP** from the current working directory to the git repository root. It concatenates all found hint files (agents.md, .goosehints, or custom configured filenames) into a single hints string that gets injected into the agent's system prompt.

**Answer to Question 2:** **NO**, goose will NOT read the subdirectory/agents.md file in this scenario. The working directory is captured once at session creation time (when you run the `goose` command), and this determines which agents.md files are loaded. Goose searches UP from this working directory to the git root, not DOWN into subdirectories. If you start goose from the repo root, only the root-level agents.md will be loaded, even if goose later accesses files in subdirectories during the session.

### Key Behavioral Points

**With Git Repository (`.git` present):**
- Loads hints from all directories from git root → current working directory
- Example: Starting in `/repo/src/utils/` loads hints from `/repo/`, `/repo/src/`, and `/repo/src/utils/`
- All files are concatenated in order (root first, current directory last)

**Without Git Repository:**
- Only loads hints from the current working directory
- Parent and ancestor hint files are completely ignored

**Working Directory is Fixed at Session Creation:**
- CLI mode captures `std::env::current_dir()` when `goose` command is invoked
- Server mode uses the `working_dir` provided in the request payload
- This working directory persists for the entire session
- Accessing files in other directories during the session does NOT change which hints are loaded

## Detailed Findings

### 1. Entry Point and Configuration

**File**: crates/goose/src/agents/prompt_manager.rs:92-101

The `SystemPromptBuilder::with_hints()` method is called during agent initialization:

```rust
let hints_filenames = config
    .get_param::<Vec<String>>("CONTEXT_FILE_NAMES")
    .unwrap_or_else(|_| {
        vec![
            GOOSE_HINTS_FILENAME.to_string(),    // ".goosehints"
            AGENTS_MD_FILENAME.to_string(),      // "AGENTS.md"
        ]
    });
```

- Default filenames: `[".goosehints", "AGENTS.md"]`
- Customizable via `CONTEXT_FILE_NAMES` environment variable (JSON array)
- Example: `export CONTEXT_FILE_NAMES='["CLAUDE.md", ".goosehints", "AGENTS.md"]'`

### 2. Working Directory Capture

**CLI Mode** (goose-cli/src/session/builder.rs:170, 331):
- Uses `std::env::current_dir()` at the moment the `goose` command is executed
- This is the directory where the user types `goose` in their terminal

**Server Mode** (goose-server/src/routes/agent.rs:61-104):
- Client provides `working_dir` in the `StartAgentRequest` payload
- Stored in the Session object

**Session Storage** (session/session_manager.rs:68-71):
- Session struct contains `pub working_dir: PathBuf`
- This value is persistent throughout the session lifecycle
- All hint loading operations use this stored value

### 3. Git Root Discovery

**File**: crates/goose/src/hints/load_hints.rs:13-28

```rust
fn find_git_root(start_dir: &Path) -> Option<&Path> {
    let mut check_dir = start_dir;

    loop {
        if check_dir.join(".git").exists() {
            return Some(check_dir);
        }
        if let Some(parent) = check_dir.parent() {
            check_dir = parent;
        } else {
            break;
        }
    }

    None
}
```

- Walks UP from the working directory checking for `.git` directory
- Returns the first parent directory containing `.git`
- Returns `None` if no git repository is found

### 4. Directory Search Path Construction

**File**: crates/goose/src/hints/load_hints.rs:30-52

The `get_local_directories()` function determines which directories to search:

**With Git Root:**
- Creates a vector of all directories from git root to current working directory
- Example: If cwd is `/repo/src/utils` and git_root is `/repo`:
  - Returns: `["/repo", "/repo/src", "/repo/src/utils"]`
- This is why parent hints are loaded

**Without Git Root:**
- Returns only the current working directory: `vec![cwd.to_path_buf()]`
- No parent directory traversal occurs

### 5. File Loading Order and Precedence

**File**: crates/goose/src/hints/load_hints.rs:54-120

Loading happens in two phases:

**Phase 1: Global Hints** (lines 62-78):
- Searches in Goose's config directory (e.g., `~/.config/goose/`)
- Looks for each filename from `CONTEXT_FILE_NAMES`
- Labeled as "Global Hints" in the output

**Phase 2: Local/Project Hints** (lines 79-101):
- Iterates through directories (from git root → cwd)
- For each directory, searches for each configured filename
- Labeled as "Project Hints" in the output

**Precedence Order:**
1. Global hints from Goose config directory
2. Git root level files
3. Intermediate directory files (ordered from root toward cwd)
4. Current working directory files
5. All files are concatenated together (additive, no overriding)

### 6. Import Boundary Enforcement

**File**: crates/goose/src/hints/load_hints.rs:82

```rust
let import_boundary = git_root.unwrap_or(cwd);
```

Hint files can use `@filename` syntax to import other files. The import boundary restricts what can be imported:

**With Git Root:**
- Can import any file within the git repository
- Files outside the git root are blocked

**Without Git Root:**
- Can only import files in current directory or subdirectories
- Parent directory imports are blocked

**File**: crates/goose/src/hints/import_files.rs:15-49

The `sanitize_reference_path()` function enforces these restrictions:
- Absolute paths are rejected
- Path traversal outside the boundary is blocked
- If blocked, the reference is left as-is in the content (e.g., `@../../../forbidden.md`)

### 7. Integration into System Prompt

**File**: crates/goose/src/agents/prompt_manager.rs:172-175, 189-197

The assembled hints string is added to the system prompt:

```rust
if let Some(hints) = self.hints {
    system_prompt_extras.push(hints);
}

format!(
    "{}\n\n# Additional Instructions:\n\n{}",
    base_prompt,
    sanitized_system_prompt_extras.join("\n\n")
)
```

All hints (global + project) are concatenated and appended to the base system prompt.

## Code References

- `crates/goose/src/hints/load_hints.rs:10-11` - Filename constants (`AGENTS_MD_FILENAME`, `GOOSE_HINTS_FILENAME`)
- `crates/goose/src/hints/load_hints.rs:13-28` - `find_git_root()` function
- `crates/goose/src/hints/load_hints.rs:30-52` - `get_local_directories()` function
- `crates/goose/src/hints/load_hints.rs:54-120` - `load_hint_files()` main loading function
- `crates/goose/src/hints/import_files.rs:136-181` - `read_referenced_files()` for @import handling
- `crates/goose/src/agents/prompt_manager.rs:92-117` - `with_hints()` integration point
- `goose-cli/src/session/builder.rs:170` - CLI working directory capture
- `session/session_manager.rs:68-71` - Session working directory storage

## Architecture Documentation

### Data Flow

1. User runs `goose` command from a directory (e.g., `/repo/src/utils/`)
2. CLI captures `std::env::current_dir()` → stored in Session as `working_dir`
3. Agent initialization calls `SystemPromptBuilder::with_hints(working_dir)`
4. Configuration reads `CONTEXT_FILE_NAMES` or uses defaults
5. Git root discovered by walking up from `working_dir` (e.g., finds `/repo/.git`)
6. Directory search path built: `["/repo", "/repo/src", "/repo/src/utils"]`
7. Global hints loaded from `~/.config/goose/`
8. Local hints loaded from each directory in search path, in order
9. All hints assembled with section headers ("Global Hints" / "Project Hints")
10. Hints added to system prompt extras
11. Final system prompt built with hints appended

### Search Behavior

**Searches UP, not DOWN:**
- Only searches parent directories (up to git root)
- Does NOT search subdirectories of the working directory
- Does NOT search sibling directories

**Example Scenario (answering the user's question):**

Repository structure:
```
/repo/
  agents.md         (contains root instructions)
  .git/
  src/
    utils/
      agents.md     (contains utils-specific instructions)
```

**Scenario A: Start goose from `/repo/`**
- Loaded: `/repo/agents.md` only
- NOT loaded: `/repo/src/utils/agents.md`
- Even if goose accesses files in `/repo/src/utils/`, it will NOT load that agents.md

**Scenario B: Start goose from `/repo/src/utils/`**
- Loaded: `/repo/agents.md` AND `/repo/src/utils/agents.md`
- Both files are concatenated into the hints

### Multiple Filename Support

When `CONTEXT_FILE_NAMES='["CLAUDE.md", ".goosehints", "AGENTS.md"]'`:
- At each directory level, searches for all three filenames
- Within each directory, processes in order: CLAUDE.md → .goosehints → AGENTS.md
- All found files are concatenated together

### Git Repository Detection

**With `.git` directory:**
- Hierarchical loading enabled
- Import boundary = git root
- Example: 3 directories searched from `/repo/` to `/repo/src/utils/`

**Without `.git` directory:**
- Only current directory searched
- Import boundary = current directory
- No parent hints loaded

## Test Documentation

### Key Tests Demonstrating Behavior

**File**: crates/goose/src/hints/load_hints.rs

1. **`test_nested_goosehints_with_git_root`** - Demonstrates hierarchical loading with git root
   - Creates structure: root/.git, root/.goosehints, root/subdir/.goosehints, root/subdir/current/.goosehints
   - Verifies all three files are loaded and concatenated

2. **`test_nested_goosehints_without_git_root`** - Demonstrates single-directory loading
   - Creates same structure but no .git directory
   - Verifies ONLY current directory .goosehints is loaded
   - Parent and grandparent files are ignored

3. **`test_hints_with_git_import_boundary`** - Demonstrates import boundary enforcement
   - Files outside git root cannot be imported
   - References to forbidden files are left as-is

### Documentation

**File**: documentation/docs/guides/using-goosehints.md

- Section "Nested `.goosehints` Files" (line 113) explains hierarchical loading
- Example showing 3-level hierarchy with detailed explanation
- Notes that hierarchical loading only works in git repositories

### Example Files in Repository

- `/AGENTS.md` - Root-level instructions for working on Goose
- `/documentation/AGENTS.md` - Documentation-specific instructions (demonstrates subdirectory hints)
- `/.goosehints` - Root level hints
- `/documentation/.goosehints` - Documentation subdirectory hints
- `/ui/desktop/.goosehints` - Desktop UI subdirectory hints

## Related Research

No prior research documents exist in this repository.

## Open Questions

None - the research comprehensively answers both questions posed by the user.
