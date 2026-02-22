use std::collections::HashMap;

use goose::prompt_template;

static GENUI_MD: &str = include_str!("../src/prompts/genui.md");
static GENUI_OUTPUT_CONTRACT: &str =
    include_str!("../src/prompts/partials/genui_output_contract.md");
static GENUI_CATALOG_PROMPT: &str = include_str!("../src/agents/genui_catalog_prompt.txt");
static JSON_RENDER_VISUAL_SKILL: &str =
    include_str!("../src/agents/builtin_skills/skills/json_render_visual.md");

#[test]
fn genui_md_includes_output_contract_partial() {
    assert!(
        GENUI_MD.contains("{% include \"partials/genui_output_contract.md\" %}"),
        "genui.md should include the shared output contract partial"
    );
}

#[test]
fn output_contract_has_core_invariants() {
    assert!(
        GENUI_OUTPUT_CONTRACT.contains("Do not include extra prose"),
        "output contract must forbid prose"
    );
    assert!(
        GENUI_OUTPUT_CONTRACT.contains("Markdown fences are **optional**"),
        "output contract must allow unfenced specs"
    );
    assert!(
        GENUI_OUTPUT_CONTRACT.contains("```json-render"),
        "output contract must specify the json-render fence when fences are used"
    );
}

#[test]
fn genui_mode_renders_with_includes() {
    let context: HashMap<String, String> = HashMap::new();
    let rendered = prompt_template::render_template("genui.md", &context)
        .expect("genui.md should render via prompt_template");

    assert!(
        rendered.contains("Output contract"),
        "Rendered genui.md should contain the contract heading"
    );
    assert!(
        rendered.contains("Do not include extra prose"),
        "Rendered genui.md should contain the no-prose rule"
    );
}

#[test]
fn other_genui_guidance_must_not_require_fences() {
    let disallowed_phrases = [
        "Output ONLY one ```json-render",
        "MUST be a single fenced",
        "always wrap output in a json-render fenced",
    ];

    for phrase in disallowed_phrases {
        assert!(
            !GENUI_CATALOG_PROMPT.contains(phrase),
            "genui catalog prompt must not require fences; found disallowed phrase: {phrase}"
        );
        assert!(
            !JSON_RENDER_VISUAL_SKILL.contains(phrase),
            "json_render_visual skill must not require fences; found disallowed phrase: {phrase}"
        );
    }

    // Positive assertion: these guidance docs should explicitly mention fences are optional.
    assert!(
        GENUI_CATALOG_PROMPT.contains("Markdown fences are optional"),
        "genui catalog prompt should explicitly say fences are optional"
    );
    assert!(
        JSON_RENDER_VISUAL_SKILL.contains("Markdown fences are optional"),
        "json_render_visual skill should explicitly say fences are optional"
    );
}
