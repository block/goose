mod morphllm_editor;
mod openai_compatible_editor;
mod relace_editor;

use anyhow::Result;

pub use morphllm_editor::MorphLLMEditor;
pub use openai_compatible_editor::OpenAICompatibleEditor;
pub use relace_editor::RelaceEditor;

#[derive(Debug, Clone)]
pub enum EditorModel {
    MorphLLM(MorphLLMEditor),
    OpenAICompatible(OpenAICompatibleEditor),
    Relace(RelaceEditor),
}

impl EditorModel {
    pub async fn edit_code(
        &self,
        original_code: &str,
        old_str: &str,
        update_snippet: &str,
    ) -> Result<String, String> {
        match self {
            EditorModel::MorphLLM(editor) => {
                editor
                    .edit_code(original_code, old_str, update_snippet)
                    .await
            }
            EditorModel::OpenAICompatible(editor) => {
                editor
                    .edit_code(original_code, old_str, update_snippet)
                    .await
            }
            EditorModel::Relace(editor) => {
                editor
                    .edit_code(original_code, old_str, update_snippet)
                    .await
            }
        }
    }

    pub fn get_str_replace_description(&self) -> &'static str {
        match self {
            EditorModel::MorphLLM(editor) => editor.get_str_replace_description(),
            EditorModel::OpenAICompatible(editor) => editor.get_str_replace_description(),
            EditorModel::Relace(editor) => editor.get_str_replace_description(),
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait EditorModelImpl {
    async fn edit_code(
        &self,
        original_code: &str,
        old_str: &str,
        update_snippet: &str,
    ) -> Result<String, String>;

    fn get_str_replace_description(&self) -> &'static str;
}

pub fn create_editor_model() -> Option<EditorModel> {
    if cfg!(test) {
        return None;
    }

    let api_key = std::env::var("GOOSE_EDITOR_API_KEY").ok()?;
    let host = std::env::var("GOOSE_EDITOR_HOST").ok()?;
    let model = std::env::var("GOOSE_EDITOR_MODEL").ok()?;

    if api_key.is_empty() || host.is_empty() || model.is_empty() {
        return None;
    }

    if host.contains("relace.run") {
        Some(EditorModel::Relace(RelaceEditor::new(api_key, host, model)))
    } else if host.contains("api.morphllm") || model.contains("morph") {
        Some(EditorModel::MorphLLM(MorphLLMEditor::new(
            api_key, host, model,
        )))
    } else {
        Some(EditorModel::OpenAICompatible(OpenAICompatibleEditor::new(
            api_key, host, model,
        )))
    }
}
