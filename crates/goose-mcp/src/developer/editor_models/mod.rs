mod morphllm_editor;
mod openai_compatible_editor;
mod relace_editor;

use anyhow::Result;

pub use morphllm_editor::MorphLLMEditor;
pub use openai_compatible_editor::OpenAICompatibleEditor;
pub use relace_editor::RelaceEditor;

/// Enum for different editor models that can perform intelligent code editing
#[derive(Debug)]
pub enum EditorModel {
    MorphLLM(MorphLLMEditor),
    OpenAICompatible(OpenAICompatibleEditor),
    Relace(RelaceEditor),
}

impl EditorModel {
    /// Call the editor API to perform intelligent code replacement
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

    /// Get the description for the str_replace command when this editor is active
    pub fn get_str_replace_description(&self) -> &'static str {
        match self {
            EditorModel::MorphLLM(editor) => editor.get_str_replace_description(),
            EditorModel::OpenAICompatible(editor) => editor.get_str_replace_description(),
            EditorModel::Relace(editor) => editor.get_str_replace_description(),
        }
    }
}

/// Trait for individual editor implementations
pub trait EditorModelImpl {
    /// Call the editor API to perform intelligent code replacement
    async fn edit_code(
        &self,
        original_code: &str,
        old_str: &str,
        update_snippet: &str,
    ) -> Result<String, String>;

    /// Get the description for the str_replace command when this editor is active
    fn get_str_replace_description(&self) -> &'static str;
}

/// Factory function to create the appropriate editor model based on environment variables
pub fn create_editor_model() -> Option<EditorModel> {
    // Don't use Editor API during tests
    if cfg!(test) {
        return None;
    }

    // Check if basic editor API variables are set
    let api_key = std::env::var("GOOSE_EDITOR_API_KEY").ok()?;
    let host = std::env::var("GOOSE_EDITOR_HOST").ok()?;
    let model = std::env::var("GOOSE_EDITOR_MODEL").ok()?;

    if api_key.is_empty() || host.is_empty() || model.is_empty() {
        return None;
    }

    // Determine which editor to use based on the host
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
