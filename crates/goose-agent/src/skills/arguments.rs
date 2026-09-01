use anyhow::{bail, Result};
use regex::{Captures, Regex};
use std::sync::LazyLock;

static PLACEHOLDER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$ARGUMENTS\[(?P<idx>\d+)\]|\$ARGUMENTS\b|\$(?P<pos>\d+)|\$(?P<name>[A-Za-z_][A-Za-z0-9_-]*)")
        .expect("skill argument regex should compile")
});

fn split_arguments(input: &str) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (None, '"') => quote = Some('"'),
            (None, '\'') if current.is_empty() => quote = Some('\''),
            (None, c) if c.is_whitespace() => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            }
            (_, '\\') if quote == Some('"') => {
                if let Some(next) = chars.peek().copied() {
                    if matches!(next, '"' | '\\') {
                        current.push(chars.next().expect("peeked character"));
                    } else {
                        current.push(ch);
                    }
                } else {
                    current.push(ch);
                }
            }
            (_, c) => current.push(c),
        }
    }
    if quote.is_some() {
        bail!("unmatched quote in skill arguments");
    }
    if !current.is_empty() {
        result.push(current);
    }
    Ok(result)
}

pub(super) fn apply(content: &str, raw: &str, names: &[String]) -> Result<String> {
    let resolvable = |caps: &Captures<'_>| {
        caps.name("name")
            .map(|name| names.iter().any(|candidate| candidate == name.as_str()))
            .unwrap_or(true)
    };
    if !PLACEHOLDER_RE
        .captures_iter(content)
        .any(|caps| resolvable(&caps))
    {
        return Ok(format!("{content}\n\nARGUMENTS: {raw}"));
    }
    let tokens = split_arguments(raw)?;
    let nth = |index: usize| tokens.get(index).cloned().unwrap_or_default();
    Ok(PLACEHOLDER_RE
        .replace_all(content, |caps: &Captures<'_>| {
            if let Some(index) = caps.name("idx") {
                return nth(index.as_str().parse().unwrap_or(usize::MAX));
            }
            if let Some(position) = caps.name("pos") {
                let position = position.as_str().parse::<usize>().unwrap_or_default();
                return position.checked_sub(1).map_or_else(String::new, nth);
            }
            if let Some(name) = caps.name("name") {
                return names
                    .iter()
                    .position(|candidate| candidate == name.as_str())
                    .map_or_else(|| caps[0].to_string(), nth);
            }
            raw.to_string()
        })
        .into_owned())
}
