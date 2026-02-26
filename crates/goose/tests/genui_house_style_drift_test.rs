use std::collections::HashMap;

static GENUI_PROMPT_MD: &str = include_str!("../src/prompts/genui.md");
static GENUI_CATALOG_PROMPT: &str = include_str!("../src/agents/genui_catalog_prompt.txt");

fn assert_contains_all(text: &str, required: &[&str]) {
    let haystack = text.to_ascii_lowercase();
    let mut missing = Vec::new();
    for needle in required {
        if !haystack.contains(&needle.to_ascii_lowercase()) {
            missing.push(*needle);
        }
    }
    assert!(
        missing.is_empty(),
        "Missing required guidance: {missing:?}\n--- text excerpt ---\n{}",
        text.lines().take(160).collect::<Vec<_>>().join("\n")
    );
}

fn assert_contains_none(text: &str, forbidden: &[&str]) {
    let haystack = text.to_ascii_lowercase();
    let mut present = Vec::new();
    for needle in forbidden {
        if haystack.contains(&needle.to_ascii_lowercase()) {
            present.push(*needle);
        }
    }
    assert!(present.is_empty(), "Found forbidden guidance: {present:?}");
}

#[test]
fn genui_prompt_has_chat_safe_layout_rules() {
    assert_contains_all(
        GENUI_PROMPT_MD,
        &[
            "Grid.columns <= 2",
            "Avoid nested Cards inside Cards",
            "1-1.5 viewport heights",
            "maxWidth \"full\"",
            "avoid 4-column KPI grids",
        ],
    );

    // Root Card requirements should be explicit in the prompt guidance (not just enforced in
    // validation), since prompt drift can silently reintroduce chat-unfriendly layouts.
    assert_contains_all(GENUI_PROMPT_MD, &["root", "card", "centered=false"]);

    // Previously-contradictory guidance we don't want to re-introduce.
    assert_contains_none(
        GENUI_PROMPT_MD,
        &[
            "maxWidth=\"lg\"",
            "maxWidth \"lg\"",
            "Grid columns=3",
            "columns=4",
        ],
    );
}

#[test]
fn genui_catalog_prompt_has_chat_safe_layout_rules() {
    assert_contains_all(
        GENUI_CATALOG_PROMPT,
        &[
            "Grid.columns <= 2",
            "Prefer a single root Card with maxWidth=\"full\" and centered=false",
            "Avoid nested Cards inside Cards",
            "avoid 4-column KPI grids",
        ],
    );

    assert_contains_all(
        GENUI_CATALOG_PROMPT,
        &["root", "card", "maxwidth=\"full\"", "centered=false"],
    );

    assert_contains_none(
        GENUI_CATALOG_PROMPT,
        &["maxWidth=\"lg\"", "maxWidth \"lg\""],
    );
}

#[test]
fn prompt_and_catalog_agree_on_key_invariants() {
    let invariants: HashMap<&str, (&str, &str)> = HashMap::from([
        ("grid_columns", ("Grid.columns <= 2", "Grid.columns <= 2")),
        (
            "nested_cards",
            (
                "Avoid nested Cards inside Cards",
                "Avoid nested Cards inside Cards",
            ),
        ),
    ]);

    for (name, (prompt_needle, catalog_needle)) in invariants {
        assert!(
            GENUI_PROMPT_MD.contains(prompt_needle),
            "genui.md missing invariant {name}: {prompt_needle}"
        );
        assert!(
            GENUI_CATALOG_PROMPT.contains(catalog_needle),
            "genui_catalog_prompt.txt missing invariant {name}: {catalog_needle}"
        );
    }
}
