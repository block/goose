use goose_tui::headless::prepend_context;
use goose_tui::hidden_blocks::CWD_ANALYSIS_TAG;

#[test]
fn prepend_context_wraps_with_tags() {
    let result = prepend_context("my prompt", Some("analysis data"));

    let expected =
        format!("<{CWD_ANALYSIS_TAG}>\nanalysis data\n</{CWD_ANALYSIS_TAG}>\n\nmy prompt");
    assert_eq!(result, expected);
}

#[test]
fn prepend_context_returns_prompt_when_none() {
    assert_eq!(prepend_context("my prompt", None), "my prompt");
}

#[test]
fn prepend_context_preserves_multiline_content() {
    let prompt = "Complex prompt with\nmultiple lines";
    let analysis = "Line 1\nLine 2\nLine 3";

    let result = prepend_context(prompt, Some(analysis));

    assert!(result.contains("Line 1\nLine 2\nLine 3"));
    assert!(result.ends_with(prompt));
}
