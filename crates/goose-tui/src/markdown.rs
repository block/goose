//! Render Markdown to plain text for the terminal.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

pub fn render(md: &str) -> String {
    let mut out = String::with_capacity(md.len());

    for event in Parser::new_ext(md, Options::ENABLE_STRIKETHROUGH) {
        match event {
            Event::Text(t) => out.push_str(&t),
            Event::Code(c) => {
                out.push('`');
                out.push_str(&c);
                out.push('`');
            }
            Event::Start(Tag::CodeBlock(_)) => out.push('\n'),
            Event::End(TagEnd::CodeBlock) => out.push('\n'),
            Event::Start(Tag::Heading { level, .. }) => {
                out.push_str(&"#".repeat(level as usize));
                out.push(' ');
            }
            Event::End(TagEnd::Heading(_)) => out.push('\n'),
            Event::Start(Tag::Item) => out.push_str("  • "),
            Event::End(TagEnd::Item) => out.push('\n'),
            Event::End(TagEnd::Paragraph) => out.push('\n'),
            Event::SoftBreak | Event::HardBreak => out.push('\n'),
            Event::Rule => out.push_str("────────────────────────────────\n"),
            _ => {}
        }
    }
    out
}
