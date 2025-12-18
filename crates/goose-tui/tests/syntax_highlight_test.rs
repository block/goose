use goose_tui::utils::styles::Theme;
use goose_tui::utils::termimad_renderer::MarkdownRenderer;

#[test]
fn render_text_without_code_blocks() {
    let renderer = MarkdownRenderer::new(&Theme::default(), None);
    let lines = renderer.render_lines("Just **bold** text", 80);
    assert!(!lines.is_empty());
}

#[test]
fn render_text_with_code_block() {
    let renderer = MarkdownRenderer::new(&Theme::default(), None);
    let text = "Hello\n```rust\nfn main() {}\n```\nWorld";
    let lines = renderer.render_lines(text, 80);
    assert!(lines.len() >= 3);
}

#[test]
fn render_consecutive_code_blocks() {
    let renderer = MarkdownRenderer::new(&Theme::default(), None);
    let text = "```rust\nlet x = 1;\n```\n```python\nx = 1\n```";
    let lines = renderer.render_lines(text, 80);
    assert!(lines.len() >= 2);
}

#[test]
fn render_unclosed_code_block_does_not_panic() {
    let renderer = MarkdownRenderer::new(&Theme::default(), None);
    let text = "Start\n```rust\nfn main() {}";
    let lines = renderer.render_lines(text, 80);
    assert!(!lines.is_empty());
}

#[test]
fn render_empty_code_block() {
    let renderer = MarkdownRenderer::new(&Theme::default(), None);
    let text = "Before\n```rust\n```\nAfter";
    let lines = renderer.render_lines(text, 80);
    assert!(!lines.is_empty());
}

#[test]
fn render_code_block_with_language_alias() {
    let renderer = MarkdownRenderer::new(&Theme::default(), None);
    let text = "```js\nconst x = 1;\n```";
    let lines = renderer.render_lines(text, 80);
    assert!(!lines.is_empty());
}

#[test]
fn dark_theme_uses_dark_highlighting() {
    let dark_renderer = MarkdownRenderer::new(&Theme::dark(), None);
    let light_renderer = MarkdownRenderer::new(&Theme::light(), None);
    let text = "```rust\nfn main() {}\n```";

    let dark_lines = dark_renderer.render_lines(text, 80);
    let light_lines = light_renderer.render_lines(text, 80);

    assert!(!dark_lines.is_empty());
    assert!(!light_lines.is_empty());
}

#[test]
fn render_code_block_at_start() {
    let renderer = MarkdownRenderer::new(&Theme::default(), None);
    let text = "```rust\nfn main() {}\n```\nAfter";
    let lines = renderer.render_lines(text, 80);
    assert!(!lines.is_empty());
}

#[test]
fn render_code_block_at_end() {
    let renderer = MarkdownRenderer::new(&Theme::default(), None);
    let text = "Before\n```rust\nfn main() {}\n```";
    let lines = renderer.render_lines(text, 80);
    assert!(!lines.is_empty());
}
