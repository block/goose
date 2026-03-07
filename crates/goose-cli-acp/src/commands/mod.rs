use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct CommandDef {
    /// e.g. `commit-push-pr` or `subdir/name`
    pub name: String,
    pub description: Option<String>,
    /// If set, the body is passed as an argument to this MCP tool name.
    pub tool: Option<String>,
    /// Defaults to `"command"`.
    pub argument_name: Option<String>,
    #[allow(dead_code)] // Claude Code compat — parsed from frontmatter, not yet surfaced
    pub argument_hint: Option<String>,
    pub body: String,
    #[allow(dead_code)] // Retained for future diagnostics (e.g. /command info)
    pub source: PathBuf,
}

#[derive(Deserialize, Default)]
struct CommandFrontmatter {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default, rename = "argument-name")]
    argument_name: Option<String>,
    #[serde(default, rename = "argument-hint")]
    argument_hint: Option<String>,
}

fn parse_frontmatter(content: &str) -> (CommandFrontmatter, String) {
    // Normalize CRLF to LF so delimiter search works on Windows.
    let content = &content.replace("\r\n", "\n");

    if let Some(rest) = content.strip_prefix("---") {
        // Match closing delimiter: "\n---\n" or "\n---" at end of string.
        let end = rest
            .find("\n---\n")
            .or_else(|| rest.strip_suffix("\n---").map(|s| s.len()));
        if let Some(end) = end {
            let yaml = rest.get(..end).unwrap_or("").trim();
            let body = rest
                .get(end..)
                .unwrap_or("")
                .trim_start_matches('\n')
                .strip_prefix("---")
                .unwrap_or("")
                .trim_start_matches('\n')
                .to_string();
            let fm: CommandFrontmatter = serde_yaml::from_str(yaml).unwrap_or_else(|e| {
                tracing::debug!("parse_frontmatter: YAML parse error: {e}");
                CommandFrontmatter::default()
            });
            return (fm, body);
        }
    }
    (CommandFrontmatter::default(), content.to_string())
}

const MAX_SCAN_DEPTH: usize = 4;

fn scan_dir(dir: &Path, base: &Path, commands: &mut Vec<CommandDef>, depth: usize) {
    if depth >= MAX_SCAN_DEPTH {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::debug!("scan_dir: cannot read {:?}: {e}", dir);
            return;
        }
    };

    let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();

    for path in paths {
        if path.is_symlink() {
            continue;
        }
        if path.is_dir() {
            scan_dir(&path, base, commands, depth + 1);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            load_command_file(&path, base, commands);
        }
    }
}

fn load_command_file(path: &Path, base: &Path, commands: &mut Vec<CommandDef>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!("cannot read command file {:?}: {e}", path);
            return;
        }
    };

    // Names may contain '/' for subdirectory commands (e.g. "subdir/name").
    // is_valid_command_name rejects '/' — that validator is for user-created commands only.
    let rel = path.strip_prefix(base).unwrap_or(path);
    let name = rel
        .with_extension("")
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");

    if name.is_empty() {
        return;
    }

    let (fm, body) = parse_frontmatter(&content);

    // Later dirs win — drop any earlier definition with the same name.
    commands.retain(|c| c.name != name);

    commands.push(CommandDef {
        name,
        description: fm.description,
        tool: fm.tool,
        argument_name: fm.argument_name,
        argument_hint: fm.argument_hint,
        body,
        source: path.to_path_buf(),
    });
}

/// Load user-defined slash commands from standard search directories.
///
/// Priority order (lowest → highest, later overrides earlier):
/// 1. `~/.claude/commands/` (Claude Code compat, user global)
/// 2. `.claude/commands/` (Claude Code compat, project-local)
/// 3. `~/.config/goose/commands/` (Goose user global)
/// 4. `.goose/commands/` (Goose project-local)
pub fn load_commands() -> Vec<CommandDef> {
    let mut commands = Vec::new();
    let cwd = std::env::current_dir().unwrap_or_else(|e| {
        tracing::debug!("load_commands: current_dir failed: {e}");
        PathBuf::new()
    });

    let dirs: [Option<PathBuf>; 4] = [
        dirs::home_dir().map(|d| d.join(".claude/commands")),
        Some(cwd.join(".claude/commands")),
        dirs::config_dir().map(|d| d.join("goose/commands")),
        Some(cwd.join(".goose/commands")),
    ];

    for dir in dirs.into_iter().flatten() {
        scan_dir(&dir, &dir, &mut commands, 0);
    }

    commands
}

pub fn substitute_args(body: &str, args: &str) -> String {
    body.replace("$ARGUMENTS", args)
}

fn is_valid_command_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn create_command(name: &str, body: &str) -> std::io::Result<()> {
    if !is_valid_command_name(name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Command name must contain only alphanumeric characters, hyphens, and underscores",
        ));
    }
    let dir = std::env::current_dir()?.join(".goose/commands");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{name}.md"));
    let content = format!("---\ndescription: User alias\n---\n{body}\n");
    std::fs::write(path, content)
}

pub fn remove_command(name: &str) -> std::io::Result<bool> {
    if !is_valid_command_name(name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid command name: {name}"),
        ));
    }
    let dir = std::env::current_dir()?.join(".goose/commands");
    let path = dir.join(format!("{name}.md"));
    if path.exists() {
        std::fs::remove_file(path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_args() {
        assert_eq!(
            substitute_args("do $ARGUMENTS now", "something"),
            "do something now"
        );
        assert_eq!(
            substitute_args("no placeholder", "ignored"),
            "no placeholder"
        );
    }

    #[test]
    fn test_parse_frontmatter_full() {
        let content =
            "---\ndescription: My command\ntool: shell__run\n---\nDo the thing $ARGUMENTS\n";
        let (fm, body) = parse_frontmatter(content);
        assert_eq!(fm.description.as_deref(), Some("My command"));
        assert_eq!(fm.tool.as_deref(), Some("shell__run"));
        assert_eq!(body.trim(), "Do the thing $ARGUMENTS");
    }

    #[test]
    fn test_parse_frontmatter_no_trailing_newline() {
        let content = "---\ndescription: Works\n---\nbody here";
        let (fm, body) = parse_frontmatter(content);
        assert_eq!(fm.description.as_deref(), Some("Works"));
        assert_eq!(body, "body here");
    }

    #[test]
    fn test_parse_frontmatter_none() {
        let content = "Just a body, no frontmatter";
        let (fm, body) = parse_frontmatter(content);
        assert!(fm.description.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn test_parse_frontmatter_crlf() {
        let content =
            "---\r\ndescription: Windows file\r\ntool: shell__run\r\n---\r\nBody with CRLF\r\n";
        let (fm, body) = parse_frontmatter(content);
        assert_eq!(fm.description.as_deref(), Some("Windows file"));
        assert_eq!(fm.tool.as_deref(), Some("shell__run"));
        assert_eq!(body.trim(), "Body with CRLF");
    }

    #[test]
    fn test_is_valid_command_name() {
        assert!(is_valid_command_name("deploy"));
        assert!(is_valid_command_name("my-command"));
        assert!(is_valid_command_name("my_command"));
        assert!(is_valid_command_name("test123"));
        assert!(is_valid_command_name("Test-Command_123"));

        assert!(!is_valid_command_name(""));
        assert!(!is_valid_command_name("has space"));
        assert!(!is_valid_command_name("has.dot"));
        assert!(!is_valid_command_name("has/slash"));
        assert!(!is_valid_command_name("../escape"));
    }

    #[test]
    fn test_create_command_invalid_name() {
        // These should fail validation without touching the filesystem
        assert!(create_command("has space", "body").is_err());
        assert!(create_command("has/slash", "body").is_err());
        assert!(create_command("has.dot", "body").is_err());
        assert!(create_command("", "body").is_err());
    }
}
