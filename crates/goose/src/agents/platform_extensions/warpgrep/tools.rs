use crate::subprocess::SubprocessExt;

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use tokio::process::Command;

const RG_MAX_COUNT: &str = "50";
const OUTPUT_CHAR_LIMIT: usize = 20_000;
const RG_TIMEOUT_SECS: u64 = 30;

pub async fn execute_tool(name: &str, arguments: &serde_json::Value, working_dir: &Path) -> String {
    match name {
        "ripgrep" => {
            let pattern = arguments
                .get("pattern")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let path = arguments
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or(".");
            let glob = arguments.get("glob").and_then(|v| v.as_str());
            ripgrep(pattern, path, glob, working_dir).await
        }
        "read" => {
            let path = arguments.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let lines = arguments.get("lines").and_then(|v| v.as_str());
            read_file(path, lines, working_dir)
        }
        "list_directory" => {
            let path = arguments
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or(".");
            list_directory(path, working_dir)
        }
        // "finish" is handled directly in client.rs via resolve_finish()
        "finish" => "Finish handled by client".to_string(),
        _ => format!("Unknown tool: {name}"),
    }
}

pub fn resolve_path(path: &str, working_dir: &Path) -> PathBuf {
    let target = PathBuf::from(path);
    if target.is_absolute() {
        target
    } else {
        working_dir.join(target)
    }
}

async fn ripgrep(pattern: &str, path: &str, glob: Option<&str>, working_dir: &Path) -> String {
    let resolved = resolve_path(path, working_dir);

    let mut cmd = Command::new("rg");
    cmd.set_no_window();
    cmd.arg("--heading")
        .arg("--line-number")
        .arg("--max-count")
        .arg(RG_MAX_COUNT);

    if let Some(glob_pattern) = glob {
        cmd.arg("--glob").arg(glob_pattern);
    }

    cmd.arg(pattern).arg(&resolved);

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(RG_TIMEOUT_SECS),
        cmd.output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if !stderr.is_empty() && stdout.is_empty() {
                return truncate_output(&stderr);
            }

            if stdout.is_empty() {
                return "No matches found.".to_string();
            }

            truncate_output(&stdout)
        }
        Ok(Err(err)) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                "Error: ripgrep (rg) is not installed. Install it with: brew install ripgrep"
                    .to_string()
            } else {
                format!("Error running ripgrep: {err}")
            }
        }
        Err(_) => "Error: ripgrep timed out.".to_string(),
    }
}

fn read_file(path: &str, lines: Option<&str>, working_dir: &Path) -> String {
    let resolved = resolve_path(path, working_dir);
    let content = match std::fs::read_to_string(&resolved) {
        Ok(text) => text,
        Err(err) => return format!("Error reading file {}: {err}", resolved.display()),
    };

    let all_lines: Vec<&str> = content.lines().collect();

    let output = match lines {
        Some(range_str) => {
            let mut buf = String::new();
            for range in range_str.split(',') {
                let range = range.trim();
                if let Some((start_str, end_str)) = range.split_once('-') {
                    let start: usize = start_str.trim().parse().unwrap_or(1);
                    let end: usize = end_str.trim().parse().unwrap_or(all_lines.len());
                    let start = start.saturating_sub(1);
                    let end = end.min(all_lines.len());
                    for (i, line) in all_lines[start..end].iter().enumerate() {
                        buf.push_str(&format!("{:>4} | {}\n", start + i + 1, line));
                    }
                }
            }
            if buf.is_empty() {
                for (i, line) in all_lines.iter().enumerate() {
                    buf.push_str(&format!("{:>4} | {}\n", i + 1, line));
                }
            }
            buf
        }
        None => {
            let mut buf = String::new();
            for (i, line) in all_lines.iter().enumerate() {
                buf.push_str(&format!("{:>4} | {}\n", i + 1, line));
            }
            buf
        }
    };

    truncate_output(&output)
}

fn list_directory(path: &str, working_dir: &Path) -> String {
    let resolved = resolve_path(path, working_dir);
    if !resolved.is_dir() {
        return format!("Error: not a directory: {}", resolved.display());
    }

    let mut builder = WalkBuilder::new(&resolved);
    builder.max_depth(Some(1));
    builder.git_ignore(true);
    builder.git_exclude(true);
    builder.git_global(true);
    builder.require_git(false);
    builder.ignore(true);
    builder.hidden(true);

    let mut entries: Vec<String> = builder
        .build()
        .flatten()
        .filter_map(|entry| {
            let entry_path = entry.path();
            if entry_path == resolved {
                return None;
            }
            let name = entry_path
                .file_name()
                .map(|os_name| os_name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                Some(format!("{name}/"))
            } else {
                Some(name)
            }
        })
        .collect();

    entries.sort();
    entries.join("\n")
}

fn truncate_output(text: &str) -> String {
    if text.len() <= OUTPUT_CHAR_LIMIT {
        return text.to_string();
    }

    // Collect chars up to the byte limit to avoid panicking on UTF-8 boundaries
    let mut truncated = String::with_capacity(OUTPUT_CHAR_LIMIT + 30);
    for ch in text.chars() {
        if truncated.len() + ch.len_utf8() > OUTPUT_CHAR_LIMIT {
            break;
        }
        truncated.push(ch);
    }
    truncated.push_str("\n... (output truncated)");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn read_file_with_line_numbers() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("test.txt"), "alpha\nbeta\ngamma\ndelta\n").unwrap();

        let result = read_file("test.txt", None, dir.path());
        assert!(result.contains("1 | alpha"));
        assert!(result.contains("2 | beta"));
        assert!(result.contains("4 | delta"));
    }

    #[test]
    fn read_file_with_line_ranges() {
        let dir = tempdir().unwrap();
        let content: String = (1..=20).map(|i| format!("line {i}\n")).collect();
        fs::write(dir.path().join("test.txt"), &content).unwrap();

        let result = read_file("test.txt", Some("3-5,10-12"), dir.path());
        assert!(result.contains("3 | line 3"));
        assert!(result.contains("5 | line 5"));
        assert!(result.contains("10 | line 10"));
        assert!(result.contains("12 | line 12"));
        assert!(!result.contains("6 | line 6"));
    }

    #[test]
    fn list_directory_shows_entries() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn lib() {}").unwrap();

        let result = list_directory(".", dir.path());
        assert!(result.contains("src/"));
        assert!(result.contains("main.rs"));
        assert!(result.contains("lib.rs"));
    }

    #[test]
    fn truncate_output_long_text() {
        let long = "x".repeat(OUTPUT_CHAR_LIMIT + 1000);
        let result = truncate_output(&long);
        assert!(result.len() < long.len());
        assert!(result.contains("... (output truncated)"));
    }

    #[test]
    fn truncate_output_short_text() {
        let short = "hello world";
        assert_eq!(truncate_output(short), short);
    }

    #[test]
    fn resolve_path_absolute_is_unchanged() {
        let dir = tempdir().unwrap();
        let abs = "/some/absolute/path";
        assert_eq!(resolve_path(abs, dir.path()), PathBuf::from(abs));
    }

    #[test]
    fn resolve_path_relative_joins_working_dir() {
        let dir = tempdir().unwrap();
        let result = resolve_path("src/main.rs", dir.path());
        assert_eq!(result, dir.path().join("src/main.rs"));
    }

    #[tokio::test]
    async fn execute_tool_dispatches_correctly() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("hello.txt"), "hello world\n").unwrap();

        let args = serde_json::json!({"path": "hello.txt"});
        let result = execute_tool("read", &args, dir.path()).await;
        assert!(result.contains("hello world"));
    }
}
