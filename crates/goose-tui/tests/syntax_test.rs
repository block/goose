use goose_tui::utils::syntax::{highlight_code, CodeBlockIterator, TextSegment};

#[test]
fn highlight_code_produces_styled_spans() {
    let lines = highlight_code("fn main() {}", "rust", true);
    assert!(!lines.is_empty());
    assert!(lines[0].spans.len() > 1);
}

#[test]
fn highlight_code_unknown_language_fallback() {
    let lines = highlight_code("random text", "nonexistent_lang_xyz", true);
    assert_eq!(lines.len(), 1);
}

#[test]
fn highlight_code_empty_input() {
    let lines = highlight_code("", "rust", true);
    assert!(lines.is_empty());
}

#[test]
fn highlight_code_multiline() {
    let code = "fn main() {\n    println!(\"hello\");\n}";
    let lines = highlight_code(code, "rust", true);
    assert_eq!(lines.len(), 3);
}

#[test]
fn highlight_code_light_mode() {
    let lines = highlight_code("fn main() {}", "rust", false);
    assert!(!lines.is_empty());
    assert!(lines[0].spans.len() > 1);
}

#[test]
fn highlight_code_language_alias_js() {
    let lines = highlight_code("const x = 1;", "js", true);
    assert!(!lines.is_empty());
}

#[test]
fn highlight_code_language_alias_py() {
    let lines = highlight_code("def foo(): pass", "py", true);
    assert!(!lines.is_empty());
}

#[test]
fn highlight_code_language_alias_ts() {
    let lines = highlight_code("const x: number = 1;", "ts", true);
    assert!(!lines.is_empty());
}

#[test]
fn highlight_code_language_alias_sh() {
    let lines = highlight_code("echo hello", "sh", true);
    assert!(!lines.is_empty());
}

#[test]
fn highlight_code_language_alias_yml() {
    let lines = highlight_code("key: value", "yml", true);
    assert!(!lines.is_empty());
}

#[test]
fn code_block_iterator_no_blocks() {
    let text = "Just plain text";
    let segments: Vec<_> = CodeBlockIterator::new(text).collect();
    assert_eq!(segments.len(), 1);
    assert!(matches!(segments[0], TextSegment::Text("Just plain text")));
}

#[test]
fn code_block_iterator_single_block() {
    let text = "```rust\nfn main() {}\n```";
    let segments: Vec<_> = CodeBlockIterator::new(text).collect();
    assert_eq!(segments.len(), 1);
    match &segments[0] {
        TextSegment::CodeBlock { lang, code } => {
            assert_eq!(*lang, "rust");
            assert_eq!(*code, "fn main() {}\n");
        }
        _ => panic!("Expected CodeBlock"),
    }
}

#[test]
fn code_block_iterator_mixed_content() {
    let text = "Before\n```python\ndef foo(): pass\n```\nAfter";
    let segments: Vec<_> = CodeBlockIterator::new(text).collect();
    assert_eq!(segments.len(), 3);
    assert!(matches!(segments[0], TextSegment::Text("Before\n")));
    assert!(matches!(
        segments[1],
        TextSegment::CodeBlock { lang: "python", .. }
    ));
    assert!(matches!(segments[2], TextSegment::Text("\nAfter")));
}

#[test]
fn code_block_iterator_unclosed_block() {
    let text = "Start\n```rust\nfn main() {}";
    let segments: Vec<_> = CodeBlockIterator::new(text).collect();
    assert_eq!(segments.len(), 2);
    assert!(matches!(segments[0], TextSegment::Text("Start\n")));
    assert!(matches!(
        segments[1],
        TextSegment::Text("```rust\nfn main() {}")
    ));
}

#[test]
fn code_block_iterator_consecutive_blocks() {
    let text = "```rust\na\n```\n```python\nb\n```";
    let segments: Vec<_> = CodeBlockIterator::new(text).collect();
    assert_eq!(segments.len(), 3);
    assert!(matches!(
        segments[0],
        TextSegment::CodeBlock { lang: "rust", .. }
    ));
    assert!(matches!(segments[1], TextSegment::Text("\n")));
    assert!(matches!(
        segments[2],
        TextSegment::CodeBlock { lang: "python", .. }
    ));
}

#[test]
fn code_block_iterator_empty_input() {
    let segments: Vec<_> = CodeBlockIterator::new("").collect();
    assert!(segments.is_empty());
}

#[test]
fn code_block_iterator_empty_code_block() {
    let text = "```rust\n```";
    let segments: Vec<_> = CodeBlockIterator::new(text).collect();
    assert_eq!(segments.len(), 1);
    match &segments[0] {
        TextSegment::CodeBlock { lang, code } => {
            assert_eq!(*lang, "rust");
            assert_eq!(*code, "");
        }
        _ => panic!("Expected CodeBlock"),
    }
}

#[test]
fn code_block_iterator_no_language() {
    let text = "```\nsome code\n```";
    let segments: Vec<_> = CodeBlockIterator::new(text).collect();
    assert_eq!(segments.len(), 1);
    match &segments[0] {
        TextSegment::CodeBlock { lang, code } => {
            assert_eq!(*lang, "");
            assert_eq!(*code, "some code\n");
        }
        _ => panic!("Expected CodeBlock"),
    }
}

#[test]
fn code_block_iterator_block_at_start() {
    let text = "```rust\ncode\n```\nafter";
    let segments: Vec<_> = CodeBlockIterator::new(text).collect();
    assert_eq!(segments.len(), 2);
    assert!(matches!(
        segments[0],
        TextSegment::CodeBlock { lang: "rust", .. }
    ));
    assert!(matches!(segments[1], TextSegment::Text("\nafter")));
}

#[test]
fn code_block_iterator_block_at_end() {
    let text = "before\n```rust\ncode\n```";
    let segments: Vec<_> = CodeBlockIterator::new(text).collect();
    assert_eq!(segments.len(), 2);
    assert!(matches!(segments[0], TextSegment::Text("before\n")));
    assert!(matches!(
        segments[1],
        TextSegment::CodeBlock { lang: "rust", .. }
    ));
}

#[test]
fn code_block_iterator_language_with_whitespace() {
    let text = "```  rust  \ncode\n```";
    let segments: Vec<_> = CodeBlockIterator::new(text).collect();
    assert_eq!(segments.len(), 1);
    match &segments[0] {
        TextSegment::CodeBlock { lang, code } => {
            assert_eq!(*lang, "rust");
            assert_eq!(*code, "code\n");
        }
        _ => panic!("Expected CodeBlock"),
    }
}
