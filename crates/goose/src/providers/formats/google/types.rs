use super::*;

#[derive(Serialize)]
pub(super) struct TextPart<'a> {
    pub text: &'a str,
}

#[derive(Serialize)]
pub(super) struct SystemInstruction<'a> {
    pub parts: [TextPart<'a>; 1],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ToolsWrapper {
    pub function_declarations: Vec<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum ThinkingLevel {
    Low,
    High,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<ThinkingLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budget: Option<i32>,
    pub include_thoughts: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GoogleRequest<'a> {
    pub system_instruction: SystemInstruction<'a>,
    pub contents: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsWrapper>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}
