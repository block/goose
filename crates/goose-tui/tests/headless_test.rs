use goose_tui::headless::prepend_context;
use goose_tui::hidden_blocks::CWD_ANALYSIS_TAG;

// ============================================================================
// prepend_context tests
// ============================================================================

#[test]
fn prepend_context_wraps_with_tags() {
    let result = prepend_context("my prompt", Some("analysis data"));

    assert!(result.starts_with(&format!("<{CWD_ANALYSIS_TAG}>")));
    assert!(result.contains("analysis data"));
    assert!(result.contains(&format!("</{CWD_ANALYSIS_TAG}>")));
    assert!(result.ends_with("my prompt"));
}

#[test]
fn prepend_context_none_returns_prompt() {
    let result = prepend_context("my prompt", None);

    assert_eq!(result, "my prompt");
}

#[test]
fn prepend_context_preserves_prompt_exactly() {
    let prompt = "Complex prompt with\nmultiple lines\nand special chars: @#$%";
    let result = prepend_context(prompt, Some("ctx"));

    assert!(result.ends_with(prompt));
}

#[test]
fn prepend_context_format_structure() {
    let result = prepend_context("prompt", Some("context"));

    // Should have format: <tag>\ncontext\n</tag>\n\nprompt
    let expected = format!("<{CWD_ANALYSIS_TAG}>\ncontext\n</{CWD_ANALYSIS_TAG}>\n\nprompt");
    assert_eq!(result, expected);
}

#[test]
fn prepend_context_multiline_analysis() {
    let analysis = "Line 1\nLine 2\nLine 3";
    let result = prepend_context("prompt", Some(analysis));

    assert!(result.contains("Line 1\nLine 2\nLine 3"));
}
