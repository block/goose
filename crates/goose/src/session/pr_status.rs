//! Infer a session's lifecycle status from GitHub pull requests referenced in
//! its conversation. A session that produced a PR is pending while the PR is
//! open, completed when it merges, and rejected when it is closed unmerged.

use crate::conversation::message::MessageContent;
use crate::conversation::Conversation;
use regex::Regex;
use std::sync::LazyLock;

use super::session_manager::SessionStatus;

static PR_URL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)/pull/(\d+)").unwrap()
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubPrRef {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

impl GitHubPrRef {
    pub fn repo_slug(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

/// Map a GitHub PR state (as reported by `gh pr view --json state`) to a
/// session lifecycle status.
pub fn status_from_github_pr_state(state: &str) -> Option<SessionStatus> {
    match state.trim().to_ascii_uppercase().as_str() {
        "OPEN" => Some(SessionStatus::Pending),
        "MERGED" => Some(SessionStatus::Completed),
        "CLOSED" => Some(SessionStatus::Rejected),
        _ => None,
    }
}

pub fn extract_github_pr_refs(text: &str) -> Vec<GitHubPrRef> {
    let mut refs: Vec<GitHubPrRef> = Vec::new();
    for caps in PR_URL_PATTERN.captures_iter(text) {
        let pr = GitHubPrRef {
            owner: caps[1].to_string(),
            repo: caps[2].to_string(),
            number: caps[3].parse().unwrap_or(0),
        };
        if let Some(idx) = refs.iter().position(|r| r == &pr) {
            refs.remove(idx);
        }
        refs.push(pr);
    }
    refs
}

/// All PRs referenced by a conversation, in the order they first appear. The
/// last entry is the most recently mentioned PR and the best candidate for
/// status inference.
pub fn find_conversation_pr_refs(conversation: &Conversation) -> Vec<GitHubPrRef> {
    let mut refs: Vec<GitHubPrRef> = Vec::new();
    for message in conversation.messages() {
        for content in &message.content {
            let texts: Vec<String> = match content {
                MessageContent::Text(text) => vec![text.text.clone()],
                MessageContent::ToolResponse(response) => match &response.tool_result {
                    Ok(result) => result
                        .content
                        .iter()
                        .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                        .collect(),
                    Err(_) => Vec::new(),
                },
                _ => Vec::new(),
            };
            for text in texts {
                for pr in extract_github_pr_refs(&text) {
                    if let Some(idx) = refs.iter().position(|r| r == &pr) {
                        refs.remove(idx);
                    }
                    refs.push(pr);
                }
            }
        }
    }
    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use rmcp::model::{CallToolResult, Content};

    #[test]
    fn extracts_pr_refs_from_text() {
        let text = "Opened https://github.com/aaif-goose/goose/pull/10616 for review,                     supersedes github.com/block/goose/pull/10500.";
        let refs = extract_github_pr_refs(text);
        assert_eq!(
            refs,
            vec![
                GitHubPrRef {
                    owner: "aaif-goose".into(),
                    repo: "goose".into(),
                    number: 10616
                },
                GitHubPrRef {
                    owner: "block".into(),
                    repo: "goose".into(),
                    number: 10500
                },
            ]
        );
        assert_eq!(refs[0].repo_slug(), "aaif-goose/goose");
    }

    #[test]
    fn repeated_pr_ref_moves_to_end() {
        let text = "see github.com/a/b/pull/1 then github.com/a/b/pull/2 and back to github.com/a/b/pull/1";
        let refs = extract_github_pr_refs(text);
        assert_eq!(refs.len(), 2);
        assert_eq!(refs.last().unwrap().number, 1);
    }

    #[test]
    fn dedupes_and_ignores_non_pr_links() {
        let text = "https://github.com/a/b/pull/1 and https://github.com/a/b/pull/1                     plus https://github.com/a/b/issues/2 and https://example.com/a/b/pull/3";
        let refs = extract_github_pr_refs(text);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].number, 1);
    }

    #[test]
    fn maps_pr_states_to_status() {
        assert_eq!(
            status_from_github_pr_state("OPEN"),
            Some(SessionStatus::Pending)
        );
        assert_eq!(
            status_from_github_pr_state("MERGED"),
            Some(SessionStatus::Completed)
        );
        assert_eq!(
            status_from_github_pr_state("CLOSED"),
            Some(SessionStatus::Rejected)
        );
        assert_eq!(
            status_from_github_pr_state("open\n"),
            Some(SessionStatus::Pending)
        );
        assert_eq!(status_from_github_pr_state("DRAFT"), None);
        assert_eq!(status_from_github_pr_state(""), None);
    }

    #[test]
    fn finds_prs_in_messages_and_tool_responses() {
        let mut conversation = Conversation::new_unvalidated(Vec::new());
        conversation.push(Message::user().with_text("working on the fix"));
        conversation.push(
            Message::assistant()
                .with_text("created https://github.com/aaif-goose/goose/pull/10616"),
        );
        conversation.push(Message::user().with_tool_response(
            "call_1",
            Ok(CallToolResult::success(vec![Content::text(
                "https://github.com/block/goose/pull/10670",
            )])),
        ));
        let refs = find_conversation_pr_refs(&conversation);
        assert_eq!(refs.len(), 2);
        assert_eq!(refs.last().unwrap().number, 10670);
    }
}
