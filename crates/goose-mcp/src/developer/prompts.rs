use include_dir::{include_dir, Dir};
use mcp_core::prompt::{Prompt, PromptArgument, PromptTemplate};

static PROMPTS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/developer/prompts");

pub fn create_prompts() -> Vec<Prompt> {
    let mut prompts = Vec::new();

    for entry in PROMPTS_DIR.files() {
        let prompt_str = String::from_utf8_lossy(entry.contents()).into_owned();

        let template: PromptTemplate =
            serde_json::from_str(&prompt_str).expect("Failed to parse prompt template");

        let arguments = template
            .arguments
            .into_iter()
            .map(|arg| PromptArgument {
                name: arg.name,
                description: arg.description,
                required: arg.required,
            })
            .collect();

        prompts.push(Prompt::new(&template.id, &template.template, arguments));
    }

    prompts
}
