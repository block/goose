use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::Result;
use goose_providers::conversation::{message::Message, Conversation};
use regex::Regex;

use crate::{providers::base::Provider, utils::safe_truncate};

pub static MSG_COUNT_FOR_SESSION_NAME_GENERATION: usize = 3;

fn strip_xml_tags(text: &str) -> String {
    static BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)<([a-zA-Z][a-zA-Z0-9_]*)[^>]*>.*?</[a-zA-Z][a-zA-Z0-9_]*>").unwrap()
    });
    static TAG_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"</?[a-zA-Z][a-zA-Z0-9_]*[^>]*>").unwrap());
    let pass1 = BLOCK_RE.replace_all(text, "");
    TAG_RE.replace_all(&pass1, "").into_owned()
}

fn extract_short_title(text: &str) -> String {
    let word_count = text.split_whitespace().count();
    if word_count <= 8 {
        return text.to_string();
    }

    {
        let mut results = Vec::new();
        let mut quote_char: Option<char> = None;
        let mut current = String::new();
        let mut prev_char: Option<char> = None;

        for ch in text.chars() {
            match quote_char {
                None => {
                    if matches!(ch, '"' | '\'' | '`') {
                        let after_alnum = prev_char.map(|p| p.is_alphanumeric()).unwrap_or(false);
                        if !after_alnum {
                            quote_char = Some(ch);
                            current.clear();
                        }
                    }
                }
                Some(q) => {
                    if ch == q {
                        let trimmed = current.trim().to_string();
                        let wc = trimmed.split_whitespace().count();
                        if (2..=8).contains(&wc) {
                            results.push(trimmed);
                        }
                        quote_char = None;
                        current.clear();
                    } else {
                        current.push(ch);
                    }
                }
            }
            prev_char = Some(ch);
        }

        if let Some(title) = results.last() {
            return title.clone();
        }
    }

    if let Some(last) = text.lines().rev().find(|l| !l.trim().is_empty()) {
        return last.trim().to_string();
    }

    text.to_string()
}

/// Returns the first 3 user messages as strings for session naming,
/// filtering out assistant-only content (e.g. preprompt blocks).
fn get_initial_user_messages(messages: &Conversation) -> Vec<String> {
    messages
        .iter()
        .filter(|m| m.role == rmcp::model::Role::User && m.is_user_visible())
        .take(MSG_COUNT_FOR_SESSION_NAME_GENERATION)
        .map(|m| {
            m.content
                .iter()
                .filter_map(|c| c.filter_for_audience(rmcp::model::Role::User))
                .filter_map(|c| c.as_text().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect()
}

/// Extracts preprompt context (assistant-audience blocks) from the first user message.
/// These are content blocks visible to the assistant but not the user.
fn get_preprompt_context(messages: &Conversation) -> String {
    messages
        .iter()
        .filter(|m| m.role == rmcp::model::Role::User)
        .take(1)
        .flat_map(|m| m.content.iter())
        .filter_map(|c| {
            // If this block is NOT visible to the user, it's preprompt/assistant-only content
            if c.filter_for_audience(rmcp::model::Role::User).is_none() {
                c.as_text().map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Reads the current git branch for a working directory, if it is inside a
/// git checkout. Handles both regular checkouts (`.git` directory) and
/// worktrees (`.git` file containing a `gitdir:` pointer). Never fails —
/// returns None when anything is missing or unexpected.
///
/// Repos at or above `home` are ignored: a `.git` at the home directory or
/// higher is almost always a dotfiles repo unrelated to the conversation.
fn read_git_branch(working_dir: &Path, home: Option<&Path>) -> Option<String> {
    let git_root = crate::hints::find_git_root(working_dir)?;
    if let Some(home) = home {
        // Reject the dotfiles case: the repo root is the home directory, or
        // the working dir lives under home but the repo root does not
        // (i.e. the walk escaped home before finding .git).
        if git_root == home || (working_dir.starts_with(home) && !git_root.starts_with(home)) {
            return None;
        }
    }
    let git_path = git_root.join(".git");
    let dir = git_root.to_path_buf();

    let head_path: PathBuf = if git_path.is_file() {
        // Worktree: .git is a file like "gitdir: /path/to/repo/.git/worktrees/x"
        let contents = std::fs::read_to_string(&git_path).ok()?;
        let gitdir = contents.strip_prefix("gitdir:")?.trim();
        let gitdir_path = PathBuf::from(gitdir);
        let gitdir_path = if gitdir_path.is_relative() {
            dir.join(gitdir_path)
        } else {
            gitdir_path
        };
        gitdir_path.join("HEAD")
    } else {
        git_path.join("HEAD")
    };

    let head = std::fs::read_to_string(head_path).ok()?;
    let branch = head.trim().strip_prefix("ref: refs/heads/")?.trim();
    if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    }
}

/// Builds deterministic naming hints from the session's working directory:
/// the folder name and, when available, the git branch. These often carry
/// the distinguishing subject (ticket ID, feature name) even when the first
/// user messages are purely mechanical.
///
/// Sessions whose working directory is the home directory itself get no
/// hints: the folder name is just the username, and any git repo there is
/// typically a dotfiles checkout unrelated to the conversation.
fn deterministic_hints_with_home(working_dir: Option<&Path>, home: Option<&Path>) -> String {
    let Some(working_dir) = working_dir else {
        return String::new();
    };
    if home.is_some_and(|h| working_dir == h) {
        return String::new();
    }
    let mut hints = Vec::new();
    if let Some(folder) = working_dir.file_name().and_then(|f| f.to_str()) {
        hints.push(format!("working folder: {folder}"));
    }
    if let Some(branch) = read_git_branch(working_dir, home) {
        hints.push(format!("git branch: {branch}"));
    }
    hints.join("\n")
}

fn deterministic_hints(working_dir: Option<&Path>) -> String {
    deterministic_hints_with_home(working_dir, dirs::home_dir().as_deref())
}

/// Assembles the user-role text for the naming request: optional preprompt
/// background, optional deterministic hints, then the user messages wrapped
/// in the markers that CLI providers use to strip this scaffolding.
fn build_naming_user_text(context: &[String], preprompt_context: &str, hints: &str) -> String {
    use crate::providers::cli_common::{
        SESSION_NAME_BEGIN_MARKER, SESSION_NAME_END_MARKER, SESSION_NAME_SUFFIX,
    };

    let preprompt_section = if preprompt_context.is_empty() {
        String::new()
    } else {
        format!(
            "---BEGIN BACKGROUND CONTEXT (for understanding only; you may borrow a ticket ID, feature, or project name from here, but do NOT base the title's topic on this)---\n{preprompt_context}\n---END BACKGROUND CONTEXT---\n\n"
        )
    };

    let hints_section = if hints.is_empty() {
        String::new()
    } else {
        format!(
            "---BEGIN HINTS (optional signals like folder and git branch; use identifiers from here when they clarify the subject)---\n{hints}\n---END HINTS---\n\n"
        )
    };

    format!(
        "{}{}{}\n{}\n{}\n\n{}",
        preprompt_section,
        hints_section,
        SESSION_NAME_BEGIN_MARKER,
        context.join("\n"),
        SESSION_NAME_END_MARKER,
        SESSION_NAME_SUFFIX,
    )
}

/// Generate a session name/description based on the conversation history
/// Creates a prompt asking for a concise description in 4 words or less.
pub(crate) async fn generate_session_name(
    provider: &dyn Provider,
    model_config: &goose_providers::model::ModelConfig,
    session_id: &str,
    messages: &Conversation,
    working_dir: Option<&Path>,
) -> Result<String> {
    let context = get_initial_user_messages(messages);
    let preprompt_context = get_preprompt_context(messages);
    let system = crate::prompt_template::render_template(
        "session_name.md",
        &std::collections::HashMap::<String, String>::new(),
    )?;

    let hints = deterministic_hints(working_dir);
    let user_text = build_naming_user_text(&context, &preprompt_context, &hints);
    let message = Message::user().with_text(&user_text);
    let result = if provider.manages_own_context() {
        crate::providers::cli_common::generate_simple_session_description(
            provider.get_name(),
            &[message],
        )?
    } else {
        crate::model_config::complete_fast(
            provider,
            model_config,
            session_id,
            &system,
            &[message],
            &[],
        )
        .await?
    };

    let raw: String = result
        .0
        .content
        .iter()
        .filter_map(|c| c.as_text())
        .collect();
    let description = strip_xml_tags(&raw)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    Ok(safe_truncate(&extract_short_title(&description), 100))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_hints_no_working_dir() {
        assert_eq!(deterministic_hints(None), "");
    }

    #[test]
    fn test_naming_user_text_includes_hints_before_messages() {
        let context = vec!["set up a worktree".to_string()];
        let text = build_naming_user_text(
            &context,
            "",
            "working folder: goose\ngit branch: tulsi/bot-1565-session-titles",
        );
        let hints_idx = text
            .find("git branch: tulsi/bot-1565-session-titles")
            .expect("hints must appear in the naming prompt");
        let begin_idx = text
            .find(crate::providers::cli_common::SESSION_NAME_BEGIN_MARKER)
            .expect("user-message marker must appear");
        assert!(
            hints_idx < begin_idx,
            "hints must precede the user messages section"
        );
        assert!(text.contains("---BEGIN HINTS"));
        assert!(text.contains("set up a worktree"));
    }

    #[test]
    fn test_naming_user_text_omits_empty_sections() {
        let context = vec!["hello".to_string()];
        let text = build_naming_user_text(&context, "", "");
        assert!(!text.contains("---BEGIN HINTS"));
        assert!(!text.contains("---BEGIN BACKGROUND CONTEXT"));
        assert!(text.starts_with(crate::providers::cli_common::SESSION_NAME_BEGIN_MARKER));
    }

    #[test]
    fn test_naming_prompt_matches_cli_description_detection() {
        // CLI providers detect naming requests by string-matching the system
        // prompt (see is_session_description_request). If this fails after a
        // prompt edit, CLI-provider session naming silently breaks.
        let system = crate::prompt_template::render_template(
            "session_name.md",
            &std::collections::HashMap::<String, String>::new(),
        )
        .unwrap();
        assert!(
            crate::providers::cli_common::is_session_description_request(&system),
            "session_name.md must keep the phrase CLI providers match on"
        );
    }

    #[test]
    fn test_deterministic_hints_folder_only() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("acme-renewal");
        std::fs::create_dir(&project).unwrap();
        let hints = deterministic_hints_with_home(Some(&project), None);
        assert_eq!(hints, "working folder: acme-renewal");
    }

    #[test]
    fn test_deterministic_hints_with_git_branch() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("goose-internal");
        let git = repo.join(".git");
        std::fs::create_dir_all(&git).unwrap();
        std::fs::write(
            git.join("HEAD"),
            "ref: refs/heads/tulsi/bot-1565-session-titles\n",
        )
        .unwrap();
        let hints = deterministic_hints_with_home(Some(&repo), None);
        assert_eq!(
            hints,
            "working folder: goose-internal\ngit branch: tulsi/bot-1565-session-titles"
        );
    }

    #[test]
    fn test_deterministic_hints_from_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let nested = repo.join("crates").join("goose");
        let git = repo.join(".git");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(&git).unwrap();
        std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        let hints = deterministic_hints_with_home(Some(&nested), None);
        assert_eq!(hints, "working folder: goose\ngit branch: main");
    }

    #[test]
    fn test_deterministic_hints_suppressed_for_home_dir() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("tulsi");
        let git = home.join(".git");
        std::fs::create_dir_all(&git).unwrap();
        std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        // Even with a dotfiles repo at $HOME, sessions started in the home
        // directory get no hints — the folder name is just the username.
        assert_eq!(deterministic_hints_with_home(Some(&home), Some(&home)), "");
    }

    #[test]
    fn test_git_branch_walk_stops_at_home_dir() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("tulsi");
        let git = home.join(".git");
        std::fs::create_dir_all(&git).unwrap();
        std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        // A plain (non-repo) folder under home must not inherit the dotfiles
        // repo's branch from the ancestor walk.
        let project = home.join("Documents").join("notes");
        std::fs::create_dir_all(&project).unwrap();
        assert_eq!(read_git_branch(&project, Some(&home)), None);
        assert_eq!(
            deterministic_hints_with_home(Some(&project), Some(&home)),
            "working folder: notes"
        );
    }

    #[test]
    fn test_git_branch_found_below_home_dir() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("tulsi");
        let repo = home.join("code").join("payments");
        let git = repo.join(".git");
        std::fs::create_dir_all(&git).unwrap();
        std::fs::write(git.join("HEAD"), "ref: refs/heads/fix/refund-timeout\n").unwrap();
        assert_eq!(
            read_git_branch(&repo, Some(&home)),
            Some("fix/refund-timeout".to_string())
        );
    }

    #[test]
    fn test_read_git_branch_worktree_gitfile() {
        let dir = tempfile::tempdir().unwrap();
        let main_repo = dir.path().join("main");
        let worktree_gitdir = main_repo.join(".git").join("worktrees").join("wt");
        std::fs::create_dir_all(&worktree_gitdir).unwrap();
        std::fs::write(
            worktree_gitdir.join("HEAD"),
            "ref: refs/heads/feature/wt-branch\n",
        )
        .unwrap();

        let worktree = dir.path().join("wt");
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", worktree_gitdir.display()),
        )
        .unwrap();

        assert_eq!(
            read_git_branch(&worktree, None),
            Some("feature/wt-branch".to_string())
        );
    }

    #[test]
    fn test_read_git_branch_detached_head() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let git = repo.join(".git");
        std::fs::create_dir_all(&git).unwrap();
        std::fs::write(git.join("HEAD"), "1c1bd5299a243f309cb251d2bbe429c7f4\n").unwrap();
        assert_eq!(read_git_branch(&repo, None), None);
    }

    #[test]
    fn test_strip_xml_tags() {
        assert_eq!(strip_xml_tags("<think>reasoning</think>answer"), "answer");
        assert_eq!(strip_xml_tags("before<t>mid</t>after"), "beforeafter");
        assert_eq!(strip_xml_tags("<a>x</a><b>y</b>z"), "z");
        assert_eq!(strip_xml_tags("no tags here"), "no tags here");
        assert_eq!(strip_xml_tags("a < b > c"), "a < b > c");
        assert_eq!(strip_xml_tags("<think>über</think>ok"), "ok");
        assert_eq!(strip_xml_tags("<think>日本語</think>hello"), "hello");
        assert_eq!(strip_xml_tags(""), "");
        assert_eq!(strip_xml_tags("<>stuff</>"), "<>stuff</>");
        // attributes
        assert_eq!(
            strip_xml_tags(r#"<think class="deep">reasoning</think>answer"#),
            "answer"
        );
        // self-closing tags
        assert_eq!(strip_xml_tags("<br/>self closing"), "self closing");
        // orphan closing tags
        assert_eq!(strip_xml_tags("orphan </think> tag"), "orphan  tag");
        // multiline content
        assert_eq!(
            strip_xml_tags("<think>\nline1\nline2\n</think>result"),
            "result"
        );
    }

    #[test]
    fn test_extract_short_title() {
        assert_eq!(extract_short_title("List files"), "List files");
        assert_eq!(
            extract_short_title(
                r#"blah blah blah blah blah blah blah blah blah "List files in folder""#
            ),
            "List files in folder"
        );
        assert_eq!(
            extract_short_title(
                "blah blah blah blah blah blah blah blah blah `View current files`"
            ),
            "View current files"
        );
        assert_eq!(
            extract_short_title(
                r#"stuff stuff stuff stuff stuff stuff stuff stuff "Abc title" "Zzz title""#
            ),
            "Zzz title"
        );
        assert_eq!(
            extract_short_title(
                "long long long long long long long long long\nList files in folder"
            ),
            "List files in folder"
        );
        assert_eq!(
            extract_short_title(
                r#"lots of words here and there and more and more "single" final line here"#
            ),
            "lots of words here and there and more and more \"single\" final line here"
        );
        assert_eq!(extract_short_title("Hello world"), "Hello world");
        assert_eq!(
            extract_short_title(
                r#"1. Analyze the request. 2. The user's message says list files. 3. "List current folder files" fits perfectly. Result: List current folder files"#
            ),
            "List current folder files"
        );
        assert_eq!(
            extract_short_title(
                r#"the user's phrasing is about listing files and the user's intent is clear. "List folder files" is best"#
            ),
            "List folder files"
        );
        assert_eq!(
            extract_short_title(
                "lots of reasoning here about what to call it\nList current folder files"
            ),
            "List current folder files"
        );
    }
}
