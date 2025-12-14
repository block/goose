use crate::hidden_blocks::ATTACHED_FILES_TAG;
use std::path::{Path, PathBuf};

const MAX_LINES: usize = 2000;
pub const PATH_TERMINATORS: &str = ",:;!?()[]{}\"'";

#[derive(Debug, Clone)]
pub struct AttachedFile {
    pub path: PathBuf,
    pub content: String,
    pub line_count: usize,
    pub truncated: bool,
}

pub struct ProcessResult {
    pub augmented_text: String,
    pub attachments: Vec<AttachedFile>,
    pub errors: Vec<(String, String)>,
}

pub fn process(input: &str, working_dir: &Path) -> ProcessResult {
    let mentions = extract_mentions(input);

    if mentions.is_empty() {
        return ProcessResult {
            augmented_text: input.to_string(),
            attachments: vec![],
            errors: vec![],
        };
    }

    let attachments: Vec<_> = mentions
        .iter()
        .filter_map(|m| read_file(m, working_dir).ok())
        .collect();

    ProcessResult {
        augmented_text: build_augmented_message(input, &attachments, working_dir),
        attachments,
        errors: vec![],
    }
}

fn extract_mentions(input: &str) -> Vec<String> {
    let mut mentions = Vec::new();
    let mut chars = input.chars().peekable();
    let mut prev_char: Option<char> = None;

    while let Some(c) = chars.next() {
        if c == '@' && prev_char.is_none_or(|p| p.is_whitespace() || "([{<".contains(p)) {
            let path = consume_path(&mut chars);
            if !path.is_empty() && looks_like_path(&path) {
                mentions.push(path);
            }
        }
        prev_char = Some(c);
    }

    mentions
}

fn looks_like_path(s: &str) -> bool {
    s.contains('/') || s.contains('.') || s.starts_with('~')
}

fn consume_path(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut path = String::new();
    let mut escaped = false;

    while let Some(&c) = chars.peek() {
        if escaped {
            path.push(c);
            escaped = false;
            chars.next();
        } else if c == '\\' {
            escaped = true;
            chars.next();
        } else if c.is_whitespace() || PATH_TERMINATORS.contains(c) {
            break;
        } else {
            path.push(c);
            chars.next();
        }
    }

    path
}

fn read_file(mention: &str, working_dir: &Path) -> Result<AttachedFile, String> {
    let path = resolve_path(mention, working_dir)?;
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let line_count = content.lines().count();
    let (content, truncated) = truncate_if_needed(content);

    Ok(AttachedFile {
        path,
        content,
        line_count,
        truncated,
    })
}

fn resolve_path(mention: &str, working_dir: &Path) -> Result<PathBuf, String> {
    let path = if let Some(home_relative) = mention.strip_prefix("~/") {
        dirs::home_dir()
            .ok_or("Cannot resolve home directory")?
            .join(home_relative)
    } else if Path::new(mention).is_absolute() {
        PathBuf::from(mention)
    } else {
        working_dir.join(mention)
    };

    if !path.exists() {
        return Err(format!("File not found: {mention}"));
    }
    if path.is_dir() {
        return Err(format!("Directories not supported: {mention}"));
    }

    Ok(path)
}

fn truncate_if_needed(content: String) -> (String, bool) {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= MAX_LINES {
        (content, false)
    } else {
        (lines[..MAX_LINES].join("\n"), true)
    }
}

fn build_augmented_message(
    input: &str,
    attachments: &[AttachedFile],
    working_dir: &Path,
) -> String {
    if attachments.is_empty() {
        return input.to_string();
    }

    let files_xml: String = attachments
        .iter()
        .map(|file| {
            let relative = file.path.strip_prefix(working_dir).unwrap_or(&file.path);
            let content = if file.content.ends_with('\n') {
                &file.content
            } else {
                &format!("{}\n", file.content)
            };
            format!(
                "<file path=\"{}\">\n{}</file>\n",
                relative.display(),
                content
            )
        })
        .collect();

    format!("<{ATTACHED_FILES_TAG}>\n<!-- Contents provided inline. Do not use tools to re-read these files. -->\n{files_xml}</{ATTACHED_FILES_TAG}>\n\n{input}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn end_to_end_file_attachment() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.rs"), "fn main() {}").unwrap();

        let result = process("Review @test.rs please", dir.path());

        assert!(result.errors.is_empty());
        assert_eq!(result.attachments.len(), 1);
        assert_eq!(result.attachments[0].content, "fn main() {}");
        assert!(result.augmented_text.contains("<attached_files_goose_tui>"));
        assert!(result.augmented_text.ends_with("Review @test.rs please"));

        let missing = process("Check @nonexistent.txt", dir.path());
        assert!(missing.errors.is_empty());
        assert!(missing.attachments.is_empty());
        assert_eq!(missing.augmented_text, "Check @nonexistent.txt");

        let plain = process("No mentions here", dir.path());
        assert_eq!(plain.augmented_text, "No mentions here");
        assert!(plain.attachments.is_empty());
    }
}
