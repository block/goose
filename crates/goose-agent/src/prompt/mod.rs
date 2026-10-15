//! Prompt rendering and instruction loading without application configuration.

#[cfg(not(target_arch = "wasm32"))]
mod hints;
#[cfg(not(target_arch = "wasm32"))]
mod import_files;
#[cfg(not(target_arch = "wasm32"))]
pub use hints::{build_gitignore, load_hint_files, SubdirectoryHintTracker};

use minijinja::{Environment, Error as MiniJinjaError, Value as MJValue};
use serde::Serialize;
use unicode_normalization::UnicodeNormalization;

/// Wrap code in a markdown fence longer than any backtick run it contains,
/// so embedded fences cannot close the block early.
fn code_fence(code: String) -> String {
    let longest_run = code
        .chars()
        .fold((0usize, 0usize), |(max, run), c| {
            if c == '`' {
                (max.max(run + 1), run + 1)
            } else {
                (max, 0)
            }
        })
        .0;
    let fence = "`".repeat((longest_run + 1).max(3));
    format!("{fence}\n{}\n{fence}", code.trim_end_matches('\n'))
}

/// Renders a MiniJinja template with the `code_fence` filter.
pub fn render_string<T: Serialize>(
    template_str: &str,
    context: &T,
) -> Result<String, MiniJinjaError> {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);
    env.add_filter("code_fence", code_fence);
    env.add_template("template", template_str)?;
    let tmpl = env.get_template("template")?;
    let ctx = MJValue::from_serialize(context);
    let rendered = tmpl.render(ctx)?;
    Ok(rendered.trim().to_string())
}

/// Appends ordered, literal instructions to an already-rendered base prompt.
pub fn append_instructions(
    base_prompt: String,
    extras: impl IntoIterator<Item = String>,
) -> String {
    let extras: Vec<String> = extras
        .into_iter()
        .map(|extra| {
            extra
                .nfc()
                .filter(|ch| !matches!(ch, '\u{E0000}'..='\u{E007F}'))
                .collect()
        })
        .collect();
    if extras.is_empty() {
        base_prompt
    } else {
        format!(
            "{}\n\n# Additional Instructions:\n\n{}",
            base_prompt,
            extras.join("\n\n")
        )
    }
}
