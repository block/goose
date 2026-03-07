use std::path::{Component, Path, PathBuf};

pub fn git_branch() -> Option<String> {
    let mut dir = std::env::current_dir().ok()?;
    // Limit traversal to avoid runaway on unusual filesystems.
    for _ in 0..50 {
        let git_path = dir.join(".git");
        if git_path.is_file() {
            let content = std::fs::read_to_string(&git_path).ok()?;
            let gitdir_raw = content.strip_prefix("gitdir: ")?.trim();
            // Resolve relative gitdir paths against the directory containing .git
            let gitdir = if Path::new(gitdir_raw).is_relative() {
                dir.join(gitdir_raw)
            } else {
                PathBuf::from(gitdir_raw)
            };
            let head = std::fs::read_to_string(gitdir.join("HEAD")).ok()?;
            return parse_head(&head);
        }
        if git_path.is_dir() {
            let head = std::fs::read_to_string(git_path.join("HEAD")).ok()?;
            return parse_head(&head);
        }
        if !dir.pop() {
            return None;
        }
    }
    None
}

fn parse_head(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if let Some(ref_path) = trimmed.strip_prefix("ref: ") {
        // Preserve full branch name after refs/heads/ (e.g. "feature/foo" not just "foo")
        Some(
            ref_path
                .strip_prefix("refs/heads/")
                .unwrap_or(ref_path)
                .to_string(),
        )
    } else {
        // Detached HEAD — show short hash (need at least 8 chars)
        Some(trimmed.get(..8)?.to_string())
    }
}

pub fn format_cwd() -> String {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!("format_cwd: current_dir failed: {e}");
            return "?".to_string();
        }
    };
    let home = dirs::home_dir();
    let display = match home {
        Some(ref home) => match cwd.strip_prefix(home) {
            Ok(rel) if rel.as_os_str().is_empty() => "~".to_string(),
            Ok(rel) => format!("~/{}", rel.display()),
            Err(_) => cwd.display().to_string(),
        },
        None => cwd.display().to_string(),
    };
    truncate_path(&display, 30)
}

fn truncate_path(path: &str, max: usize) -> String {
    if path.chars().count() <= max {
        return path.to_string();
    }
    // "…/parent/leaf" form keeps the most contextually useful components.
    let p = std::path::Path::new(path);
    let comps: Vec<&str> = p
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            Component::RootDir => Some("/"),
            _ => None,
        })
        .collect();
    if let [.., parent, leaf] = comps.as_slice() {
        format!("…/{parent}/{leaf}")
    } else {
        // Fallback for single-component paths: truncate from the right.
        let tail: String = path
            .chars()
            .rev()
            .take(max - 1)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("…{tail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_head_branch() {
        assert_eq!(
            parse_head("ref: refs/heads/main\n"),
            Some("main".to_string())
        );
    }

    #[test]
    fn parse_head_detached() {
        assert_eq!(parse_head("abc123def456\n"), Some("abc123de".to_string()));
    }

    #[test]
    fn parse_head_slash_branch() {
        assert_eq!(
            parse_head("ref: refs/heads/feature/foo\n"),
            Some("feature/foo".to_string())
        );
    }

    #[test]
    fn parse_head_tag_ref() {
        // Tags aren't under refs/heads/ — return full ref path
        assert_eq!(
            parse_head("ref: refs/tags/v1.0\n"),
            Some("refs/tags/v1.0".to_string())
        );
    }

    #[test]
    fn parse_head_short_hash() {
        assert_eq!(parse_head("abc"), None);
    }

    #[test]
    fn truncate_path_long() {
        let long = "~/very/deeply/nested/project/directory/src";
        let result = truncate_path(long, 30);
        assert!(result.starts_with('…'));
        assert!(result.len() <= 40);
    }

    #[test]
    fn truncate_path_unicode() {
        // 日本語 is 3 chars but 9 bytes — byte-length check would differ
        let path = "~/projects/日本語";
        assert_eq!(truncate_path(path, 30), path); // 16 chars, fits in 30
    }
}
