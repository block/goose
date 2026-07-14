use crate::prompt_template::render_template;
use goose_providers::json::safely_parse_json;
use serde::{Deserialize, Serialize};

/// Structured output of the compaction LLM call.
///
/// Every list is ordered most-important-first so downstream consumers (the
/// summary render template, experiments that truncate sections) can cut from
/// the tail. Fields default to empty so a response that omits a section still
/// parses; a response whose fields have the wrong shape fails to parse
/// entirely, in which case callers fall back to the raw response text.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredSummary {
    #[serde(default)]
    pub user_intent: Vec<String>,
    #[serde(default)]
    pub technical_concepts: Vec<String>,
    #[serde(default)]
    pub files: Vec<FileActivity>,
    #[serde(default)]
    pub errors_and_fixes: Vec<String>,
    #[serde(default)]
    pub problem_solving: Vec<String>,
    #[serde(default)]
    pub user_messages: Vec<String>,
    #[serde(default)]
    pub pending_tasks: Vec<String>,
    #[serde(default)]
    pub current_work: Option<String>,
    #[serde(default)]
    pub next_step: Option<String>,
    /// Unknown top-level fields, kept so a user-customized compaction prompt
    /// that adds fields can still reach them from a customized render
    /// template. Not counted when deciding whether a summary is empty.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileActivity {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub key_code: Option<String>,
}

impl StructuredSummary {
    /// Parse the compaction model's response text into a structured summary.
    ///
    /// Returns `None` when no usable JSON document is found so the caller can
    /// keep the raw response text - the lossless fallback.
    pub fn parse(response_text: &str) -> Option<Self> {
        json_candidates(response_text).into_iter().find_map(|c| {
            let value = safely_parse_json(c).ok()?;
            let mut summary: Self = serde_json::from_value(value).ok()?;
            summary.normalize();
            (!summary.is_empty()).then_some(summary)
        })
    }

    /// Render the summary into the markdown that becomes the agent-visible
    /// context after compaction, via the user-overridable template.
    pub fn render(&self) -> Result<String, minijinja::Error> {
        render_template("compaction_summary.md", self)
    }

    /// Drop entries with no visible content, so a response of blank strings
    /// counts as empty (falling back to the raw text) rather than replacing
    /// the conversation with a summary that renders nothing.
    fn normalize(&mut self) {
        fn blank(s: &str) -> bool {
            s.trim().is_empty()
        }
        for list in [
            &mut self.user_intent,
            &mut self.technical_concepts,
            &mut self.errors_and_fixes,
            &mut self.problem_solving,
            &mut self.user_messages,
            &mut self.pending_tasks,
        ] {
            list.retain(|s| !blank(s));
        }
        for file in &mut self.files {
            if file.key_code.as_deref().is_some_and(blank) {
                file.key_code = None;
            }
        }
        self.files
            .retain(|f| !blank(&f.path) || !blank(&f.summary) || f.key_code.is_some());
        if self.current_work.as_deref().is_some_and(blank) {
            self.current_work = None;
        }
        if self.next_step.as_deref().is_some_and(blank) {
            self.next_step = None;
        }
    }

    fn is_empty(&self) -> bool {
        self.user_intent.is_empty()
            && self.technical_concepts.is_empty()
            && self.files.is_empty()
            && self.errors_and_fixes.is_empty()
            && self.problem_solving.is_empty()
            && self.user_messages.is_empty()
            && self.pending_tasks.is_empty()
            && self.current_work.is_none()
            && self.next_step.is_none()
    }
}

/// Locate candidate JSON documents within the model's response text, tried in
/// order until one parses.
///
/// Preference order: a brace-balanced object after the last ```json fence
/// (the prompt asks for exactly one, after an `<analysis>` scratchpad), then
/// one after the analysis block, then one anywhere in the text. Extraction is
/// brace-balanced rather than fence-delimited because JSON string values may
/// legally contain ``` (pasted snippets, diffs of markdown). An unterminated
/// object (output cut off mid-JSON) yields no candidate: repairing truncated
/// JSON would silently drop the late, continuation-critical sections while
/// the raw-text fallback preserves them via the analysis scratchpad.
#[allow(clippy::string_slice)] // All markers are ASCII; indices are byte offsets of ASCII matches.
fn json_candidates(text: &str) -> Vec<&str> {
    let after_analysis = text
        .rfind("</analysis>")
        .map(|idx| &text[idx + "</analysis>".len()..])
        .unwrap_or(text);

    let mut candidates: Vec<&str> = [
        last_fenced_json_block(text),
        balanced_object(after_analysis),
        balanced_object(text),
    ]
    .into_iter()
    .flatten()
    .collect();
    candidates.dedup();
    candidates
}

#[allow(clippy::string_slice)] // All markers are ASCII; indices are byte offsets of ASCII matches.
fn last_fenced_json_block(text: &str) -> Option<&str> {
    let start = text.rfind("```json")? + "```json".len();
    balanced_object(&text[start..])
}

#[allow(clippy::string_slice)] // Indices come from find()/char_indices(); slicing is safe.
fn balanced_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let body = &text[start..];

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in body.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&body[..=idx]);
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_RESPONSE: &str = r#"<analysis>
The user asked to fix a bug in parser.rs. I traced it to an off-by-one
in {brace handling} and patched it.
</analysis>

```json
{
  "user_intent": ["Fix the parser bug", "Add a regression test"],
  "technical_concepts": ["off-by-one", "tokenizer"],
  "files": [
    {"path": "src/parser.rs", "summary": "Fixed off-by-one in scan loop", "key_code": "fn scan(&mut self) { .. }"}
  ],
  "errors_and_fixes": ["Panic on empty input, fixed with early return"],
  "problem_solving": ["Root-caused via failing unit test"],
  "user_messages": ["fix the parser bug", "add a test"],
  "pending_tasks": ["Add a regression test"],
  "current_work": "Writing the regression test in tests/parser.rs",
  "next_step": "Finish the regression test"
}
```"#;

    #[test]
    fn parses_fenced_json_after_analysis() {
        let summary = StructuredSummary::parse(FULL_RESPONSE).expect("should parse");
        assert_eq!(
            summary.user_intent,
            vec!["Fix the parser bug", "Add a regression test"]
        );
        assert_eq!(summary.files.len(), 1);
        assert_eq!(summary.files[0].path, "src/parser.rs");
        assert_eq!(
            summary.current_work.as_deref(),
            Some("Writing the regression test in tests/parser.rs")
        );
    }

    #[test]
    fn truncated_json_falls_back_to_raw() {
        let text = r#"```json
{"user_intent": ["Fix the bug"], "pending_tasks": ["Write tests", "Update docs"#;
        assert!(StructuredSummary::parse(text).is_none());
    }

    #[test]
    fn embedded_fences_in_string_values_do_not_truncate() {
        let text = "```json\n{\"user_intent\": [\"Document the build\"], \"files\": [{\"path\": \"README.md\", \"summary\": \"Added build docs\", \"key_code\": \"```bash\\ncargo build\\n```\"}], \"pending_tasks\": [\"Publish the docs\"], \"current_work\": \"Writing docs\"}\n```";
        let summary = StructuredSummary::parse(text).expect("should parse");
        assert_eq!(
            summary.files[0].key_code.as_deref(),
            Some("```bash\ncargo build\n```")
        );
        assert_eq!(summary.pending_tasks, vec!["Publish the docs"]);
        assert_eq!(summary.current_work.as_deref(), Some("Writing docs"));
    }

    #[test]
    fn retries_next_candidate_when_fenced_extraction_fails() {
        let text = "<analysis>the model was told to emit ```json with {braces</analysis>\n{\"user_intent\": [\"Real goal\"]}";
        let summary = StructuredSummary::parse(text).expect("should parse");
        assert_eq!(summary.user_intent, vec!["Real goal"]);
    }

    #[test]
    fn rejects_unusable_or_empty_json() {
        for text in [
            "Here is a summary of the conversation. The user asked about compaction.",
            r#"{"user_intent": "not a list", "pending_tasks": ["task"]}"#,
            "{}",
            r#"{"notes": "unknown fields alone are not a summary"}"#,
            r#"{"current_work": ""}"#,
            r#"{"files": [{}], "user_intent": [" "]}"#,
        ] {
            assert!(
                StructuredSummary::parse(text).is_none(),
                "should fall back to raw text for: {text}"
            );
        }
    }

    #[test]
    fn drops_blank_entries_but_keeps_content() {
        let text = r#"{"user_intent": ["", "Fix the bug"], "files": [{"path": "a.rs", "summary": "Patched", "key_code": "  "}], "next_step": " "}"#;
        let summary = StructuredSummary::parse(text).expect("should parse");
        assert_eq!(summary.user_intent, vec!["Fix the bug"]);
        assert_eq!(summary.files[0].key_code, None);
        assert_eq!(summary.next_step, None);
    }

    #[test]
    fn renders_markdown_sections() {
        let summary = StructuredSummary::parse(FULL_RESPONSE).unwrap();
        let rendered = summary.render().expect("should render");
        assert!(rendered.contains("## User Intent"));
        assert!(rendered.contains("- Fix the parser bug"));
        assert!(rendered.contains("### src/parser.rs"));
        assert!(rendered.contains("fn scan(&mut self) { .. }"));
        assert!(rendered.contains("## Next Step"));
    }

    #[test]
    fn render_fences_exceed_backtick_runs_in_key_code() {
        let summary = StructuredSummary {
            files: vec![FileActivity {
                path: "docs/build.md".to_string(),
                summary: "Documented the build".to_string(),
                key_code: Some(
                    "```bash\ncargo build\n```\n````\nnested fence docs\n````".to_string(),
                ),
            }],
            errors_and_fixes: vec!["None".to_string()],
            ..Default::default()
        };
        let rendered = summary.render().expect("should render");
        // key_code's longest backtick run is four, so the fence must be five
        assert_eq!(rendered.matches("\n`````\n").count(), 2);
        let errors_heading = rendered.find("## Errors + Fixes").unwrap();
        let closing_fence = rendered.rfind("\n`````\n").unwrap();
        assert!(errors_heading > closing_fence);
    }
}
