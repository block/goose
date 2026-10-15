use goose_agent::prompt::{append_instructions, render_string};
use serde_json::json;

#[test]
fn templates_keep_the_existing_rendering_and_code_fence_filter() {
    assert_eq!(
        render_string("  Hello {{ name }}  ", &json!({"name": "world"})).unwrap(),
        "Hello world"
    );
    assert_eq!(
        render_string(
            "{{ code | code_fence }}",
            &json!({"code": "```rust\ncode\n```\n"})
        )
        .unwrap(),
        "````\n```rust\ncode\n```\n````"
    );
    assert!(render_string("{% if %}", &json!({})).is_err());
}

#[test]
fn additional_instructions_preserve_order_and_remain_literal() {
    assert_eq!(append_instructions("base".into(), []), "base");
    assert_eq!(
        append_instructions(
            "base".into(),
            ["e\u{301}\u{E0061}".into(), "{{ literal }}".into()]
        ),
        "base\n\n# Additional Instructions:\n\né\n\n{{ literal }}"
    );
}
