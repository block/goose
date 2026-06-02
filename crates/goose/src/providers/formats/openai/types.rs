use super::*;

pub(super) type ToolCallData = HashMap<
    i32,
    (
        String,
        String,
        String,
        Option<serde_json::Map<String, Value>>,
    ),
>;

pub(super) fn deserialize_null_default_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

pub(super) fn is_reserved_request_param_key(key: &str) -> bool {
    matches!(key, "messages" | "model" | "stream" | "stream_options")
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OpenAiFormatOptions {
    pub preserve_thinking_context: bool,
}

pub(super) fn merge_reasoning_text(prefix: &str, suffix: &str) -> String {
    if prefix.is_empty() {
        return suffix.to_string();
    }
    if suffix.is_empty() {
        return prefix.to_string();
    }
    if suffix.starts_with(prefix) {
        return suffix.to_string();
    }
    if prefix.ends_with(suffix) {
        return prefix.to_string();
    }

    format!("{prefix}{suffix}")
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub(super) struct DeltaToolCallFunction {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default_string")]
    pub arguments: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct DeltaToolCall {
    pub id: Option<String>,
    pub function: DeltaToolCallFunction,
    pub index: Option<i32>,
    pub r#type: Option<String>,
    #[serde(flatten)]
    pub extra: Option<serde_json::Map<String, Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub(super) enum DeltaContent {
    String(String),
    Array(Vec<DeltaContentPart>),
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct DeltaContentPart {
    pub r#type: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(rename = "thoughtSignature")]
    pub thought_signature: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub(super) struct Delta {
    #[serde(default)]
    pub content: Option<DeltaContent>,
    pub role: Option<String>,
    pub tool_calls: Option<Vec<DeltaToolCall>>,
    pub reasoning_details: Option<Vec<Value>>,
    pub reasoning: Option<String>,
    pub reasoning_content: Option<String>,
}

impl Delta {
    /// Prefer `reasoning_content` (DeepSeek/OpenRouter) over `reasoning`
    /// (vLLM); some servers (gpt-oss via vLLM) emit both. Skip empty values.
    pub fn reasoning_text(&self) -> Option<&str> {
        self.reasoning_content
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.reasoning.as_deref().filter(|s| !s.is_empty()))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct StreamingChoice {
    #[serde(default)]
    pub delta: Delta,
    pub index: Option<i32>,
    pub finish_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct StreamingChunk {
    pub choices: Vec<StreamingChoice>,
    pub created: Option<i64>,
    pub id: Option<String>,
    pub usage: Option<Value>,
    pub model: Option<String>,
}
