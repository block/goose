//! map tool - Build a mental map of what exists and where.
//!
//! This tool provides directory tree views with line count annotations.

use std::fs;
use std::path::{Path, PathBuf};

use rmcp::model::{CallToolResult, Content};
use schemars::JsonSchema;
use serde::Deserialize;

use super::should_ignore;

// ============================================================================
// Tool Parameters
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MapParams {
    /// Absolute path to the directory to map.
    pub path: String,

    /// Maximum depth to traverse. Default is 2. Use 0 for unlimited (careful with large repos).
    #[serde(default = "default_depth")]
    pub depth: u32,
}

fn default_depth() -> u32 {
    2
}

// ============================================================================
// Tool Implementation
// ============================================================================

pub struct MapTool;

impl MapTool {
    pub fn new() -> Self {
        Self
    }

    pub fn map(&self, params: MapParams) -> CallToolResult {
        let path = PathBuf::from(&params.path);

        if !path.exists() {
            return CallToolResult::error(vec![Content::text(format!(
                "Path does not exist: {}",
                params.path
            ))]);
        }

        if !path.is_dir() {
            return CallToolResult::error(vec![Content::text(format!(
                "Path is not a directory: {}",
                params.path
            ))]);
        }

        let max_depth = if params.depth == 0 {
            usize::MAX
        } else {
            params.depth as usize
        };

        let mut output = String::new();
        build_tree(&path, 0, max_depth, &mut output);

        CallToolResult::success(vec![Content::text(output)])
    }
}

fn build_tree(dir: &Path, depth: usize, max_depth: usize, output: &mut String) {
    if depth > max_depth {
        return;
    }

    let indent = "  ".repeat(depth);

    let mut entries: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };

    // Sort: directories first, then files, alphabetically
    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if should_ignore(&path) {
            continue;
        }

        if path.is_dir() {
            let dir_lines = count_dir_lines(&path, max_depth - depth);
            let annotation = format_lines(dir_lines);
            output.push_str(&format!("{}{}/  {}\n", indent, name, annotation));

            if depth < max_depth {
                build_tree(&path, depth + 1, max_depth, output);
            }
        } else if path.is_file() {
            let lines = count_file_lines(&path);
            let annotation = format_lines(lines);
            output.push_str(&format!("{}{}  {}\n", indent, name, annotation));
        }
    }
}

fn count_dir_lines(dir: &Path, remaining_depth: usize) -> usize {
    let mut total = 0;

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();

        if should_ignore(&path) {
            continue;
        }

        if path.is_dir() && remaining_depth > 0 {
            total += count_dir_lines(&path, remaining_depth - 1);
        } else if path.is_file() {
            total += count_file_lines(&path);
        }
    }

    total
}

impl Default for MapTool {
    fn default() -> Self {
        Self::new()
    }
}

fn count_file_lines(path: &Path) -> usize {
    match fs::read_to_string(path) {
        Ok(content) => content.lines().count(),
        Err(_) => 0,
    }
}

fn format_lines(lines: usize) -> String {
    if lines >= 1000 {
        format!("[{}K]", lines / 1000)
    } else {
        format!("[{}]", lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::RawContent;
    use std::fs;
    use tempfile::TempDir;

    fn extract_text(result: &CallToolResult) -> &str {
        match &result.content[0].raw {
            RawContent::Text(t) => &t.text,
            _ => panic!("Expected text content"),
        }
    }

    fn setup_test_dir() -> TempDir {
        let dir = tempfile::tempdir().unwrap();

        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("src/main.rs"),
            "fn main() {\n    println!(\"Hello\");\n}\n",
        )
        .unwrap();

        fs::write(
            dir.path().join("src/lib.rs"),
            "pub struct Foo;\n\nimpl Foo {\n    pub fn new() -> Self { Self }\n}\n",
        )
        .unwrap();

        fs::create_dir_all(dir.path().join("tests")).unwrap();
        fs::write(
            dir.path().join("tests/test.rs"),
            "#[test]\nfn test_foo() {\n    assert!(true);\n}\n",
        )
        .unwrap();

        dir
    }

    #[test]
    fn test_map_basic() {
        let dir = setup_test_dir();
        let tool = MapTool::new();

        let result = tool.map(MapParams {
            path: dir.path().to_string_lossy().to_string(),
            depth: 2,
        });

        assert!(!result.is_error.unwrap_or(false));

        let content = extract_text(&result);

        assert!(content.contains("src/"));
        assert!(content.contains("tests/"));
        assert!(content.contains("main.rs"));
        assert!(content.contains("lib.rs"));
        assert!(content.contains("["));
    }

    #[test]
    fn test_map_nonexistent() {
        let tool = MapTool::new();

        let result = tool.map(MapParams {
            path: "/nonexistent/path".to_string(),
            depth: 2,
        });

        assert!(result.is_error.unwrap_or(false));
    }

    #[test]
    fn test_map_file_not_dir() {
        let dir = setup_test_dir();
        let tool = MapTool::new();

        let result = tool.map(MapParams {
            path: dir.path().join("src/main.rs").to_string_lossy().to_string(),
            depth: 2,
        });

        assert!(result.is_error.unwrap_or(false));
    }

    #[test]
    fn test_map_ignores_hidden() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".git/config"), "git config").unwrap();
        fs::write(dir.path().join("visible.rs"), "fn foo() {}").unwrap();

        let tool = MapTool::new();
        let result = tool.map(MapParams {
            path: dir.path().to_string_lossy().to_string(),
            depth: 2,
        });

        let content = extract_text(&result);

        assert!(content.contains("visible.rs"));
        assert!(!content.contains(".git"));
    }
}
