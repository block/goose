use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Convert markdown into Telegram-compatible HTML.
///
/// Telegram supports only a minimal HTML subset with no attributes beyond
/// `href` on `<a>`. Unsupported tags or attributes (e.g. `class`) cause the
/// API to reject the message outright.
pub fn markdown_to_telegram_html(markdown: &str) -> String {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(markdown, options);

    let mut output = String::with_capacity(markdown.len());
    let mut list_number: Option<u64> = None;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                Tag::Heading { .. } => output.push_str("<b>"),
                Tag::Strong => output.push_str("<b>"),
                Tag::Emphasis => output.push_str("<i>"),
                Tag::Strikethrough => output.push_str("<s>"),
                Tag::CodeBlock(_) => output.push_str("<pre><code>"),
                Tag::Link { dest_url, .. } => {
                    output.push_str(&format!("<a href=\"{}\">", escape_html(&dest_url)));
                }
                Tag::List(start) => {
                    list_number = start;
                }
                Tag::Item => {
                    if let Some(n) = list_number.as_mut() {
                        output.push_str(&format!("{}. ", n));
                        *n += 1;
                    } else {
                        output.push_str("• ");
                    }
                }
                Tag::BlockQuote(_) => output.push_str("<blockquote>"),
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph => output.push('\n'),
                TagEnd::Heading(_) => output.push_str("</b>\n"),
                TagEnd::Strong => output.push_str("</b>"),
                TagEnd::Emphasis => output.push_str("</i>"),
                TagEnd::Strikethrough => output.push_str("</s>"),
                TagEnd::CodeBlock => output.push_str("</code></pre>\n"),
                TagEnd::Link => output.push_str("</a>"),
                TagEnd::List(_) => {
                    list_number = None;
                }
                TagEnd::Item => output.push('\n'),
                TagEnd::BlockQuote(_) => output.push_str("</blockquote>\n"),
                _ => {}
            },
            Event::Text(text) => output.push_str(&escape_html(&text)),
            Event::Code(code) => {
                output.push_str("<code>");
                output.push_str(&escape_html(&code));
                output.push_str("</code>");
            }
            Event::SoftBreak | Event::HardBreak => output.push('\n'),
            Event::Rule => output.push_str("———\n"),
            _ => {}
        }
    }

    let collapsed = collapse_newlines(&output);
    collapsed.trim().to_string()
}

/// Collapse runs of 3+ newlines down to 2, preserving whitespace inside `<pre>` blocks.
fn collapse_newlines(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_pre = false;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if !in_pre && ch == '<' {
            let rest: String = chars.clone().take(4).collect();
            if rest.starts_with("pre>") || rest.starts_with("pre ") {
                in_pre = true;
            }
            if rest.starts_with("/pre") {
                in_pre = false;
            }
            result.push(ch);
        } else if !in_pre && ch == '\n' {
            let mut count = 1;
            while chars.peek() == Some(&'\n') {
                chars.next();
                count += 1;
            }
            for _ in 0..count.min(2) {
                result.push('\n');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_unchanged() {
        assert_eq!(markdown_to_telegram_html("Hello world"), "Hello world");
    }

    #[test]
    fn bold_and_italic() {
        assert_eq!(
            markdown_to_telegram_html("This is **bold** and *italic*"),
            "This is <b>bold</b> and <i>italic</i>"
        );
    }

    #[test]
    fn inline_code() {
        assert_eq!(
            markdown_to_telegram_html("Use `cargo build` to compile"),
            "Use <code>cargo build</code> to compile"
        );
    }

    #[test]
    fn code_block_no_class_attribute() {
        let html = markdown_to_telegram_html("```rust\nfn main() {}\n```");
        assert!(
            !html.contains("class="),
            "Telegram rejects class attributes: {html}"
        );
        assert!(html.contains("<pre><code>"));
        assert!(html.contains("fn main() {}"));
        assert!(html.contains("</code></pre>"));
    }

    #[test]
    fn code_block_no_language() {
        let html = markdown_to_telegram_html("```\nhello\n```");
        assert!(html.contains("<pre><code>"));
        assert!(html.contains("hello"));
    }

    #[test]
    fn heading() {
        assert_eq!(markdown_to_telegram_html("# Title"), "<b>Title</b>");
    }

    #[test]
    fn unordered_list() {
        let html = markdown_to_telegram_html("- one\n- two\n- three");
        assert!(html.contains("• one"));
        assert!(html.contains("• two"));
        assert!(html.contains("• three"));
    }

    #[test]
    fn ordered_list() {
        let html = markdown_to_telegram_html("1. first\n2. second\n3. third");
        assert!(html.contains("1. first"));
        assert!(html.contains("2. second"));
        assert!(html.contains("3. third"));
    }

    #[test]
    fn link() {
        let html = markdown_to_telegram_html("Visit [Rust](https://rust-lang.org) docs");
        assert!(html.contains("<a href=\"https://rust-lang.org\">Rust</a>"));
    }

    #[test]
    fn html_entities_escaped() {
        assert_eq!(
            markdown_to_telegram_html("1 < 2 & 3 > 0"),
            "1 &lt; 2 &amp; 3 &gt; 0"
        );
    }

    #[test]
    fn strikethrough() {
        assert_eq!(markdown_to_telegram_html("~~deleted~~"), "<s>deleted</s>");
    }

    #[test]
    fn blockquote() {
        let html = markdown_to_telegram_html("> This is a quote");
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("This is a quote"));
        assert!(html.contains("</blockquote>"));
    }

    #[test]
    fn horizontal_rule() {
        let html = markdown_to_telegram_html("above\n\n---\n\nbelow");
        assert!(html.contains("———"));
    }

    #[test]
    fn complex_llm_response() {
        let md = r#"# Summary

Here's what I found:

1. **First item** - this is important
2. **Second item** - also relevant

```python
print("hello")
```

For more info, visit [docs](https://example.com).

> Note: this is a blockquote

That's all!"#;
        let html = markdown_to_telegram_html(md);
        assert!(!html.contains("class="), "no class attributes: {html}");
        assert!(html.contains("<b>Summary</b>"));
        assert!(html.contains("<b>First item</b>"));
        assert!(html.contains("1. "));
        assert!(html.contains("2. "));
        assert!(html.contains("<pre><code>"));
        assert!(html.contains("print(\"hello\")"));
        assert!(html.contains("<a href="));
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("That's all!"));
    }

    #[test]
    fn no_trailing_whitespace() {
        let html = markdown_to_telegram_html("Hello\n\n");
        assert!(!html.ends_with('\n'));
        assert!(!html.ends_with(' '));
    }

    #[test]
    fn no_excessive_newlines() {
        let md = "Paragraph one.\n\n\n\nParagraph two.\n\n\n\n\nParagraph three.";
        let html = markdown_to_telegram_html(md);
        assert!(!html.contains("\n\n\n"));
    }

    #[test]
    fn list_to_paragraph_spacing() {
        let md = "- one\n- two\n\nNext paragraph.";
        let html = markdown_to_telegram_html(md);
        assert!(!html.contains("\n\n\n"));
    }

    #[test]
    fn code_block_preserves_internal_newlines() {
        let md = "```\nline1\n\n\nline2\n```";
        let html = markdown_to_telegram_html(md);
        assert!(html.contains("line1\n\n\nline2"));
    }

    #[test]
    fn compact_output() {
        let md = "Sure! Here's a quick summary:\n\n## Key Points\n\n- Point one\n- Point two\n\nThat's it!";
        let html = markdown_to_telegram_html(md);
        let newline_count = html.chars().filter(|c| *c == '\n').count();
        assert!(
            newline_count <= 7,
            "too many newlines ({newline_count}): {html}"
        );
    }
}
