use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OAIUsage {
    pub prompt_tokens: Option<usize>,
    pub completion_tokens: Option<usize>,
    pub total_tokens: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAIContentFilterResult {
    pub filtered: bool,
    pub severity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAIPromptFilterResult {
    pub content_filter_results: HashMap<String, OAIContentFilterResult>,
    pub prompt_index: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OAIToolCallFunction {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub arguments: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OAIToolCall {
    pub function: OAIToolCallFunction,
    pub id: Option<String>,
    pub index: usize,
    #[serde(rename = "type")]
    pub type_: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OAIStreamDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<OAIToolCall>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAIStreamChoice {
    pub delta: OAIStreamDelta,
    pub finish_reason: Option<String>,
    pub index: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAIStreamChunk {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<i64>,
    pub model: Option<String>,
    pub system_fingerprint: Option<String>,
    pub choices: Vec<OAIStreamChoice>,
    pub usage: Option<OAIUsage>,
    pub prompt_filter_results: Option<Vec<OAIPromptFilterResult>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAIChatMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<OAIToolCall>,
    #[serde(default)]
    pub padding: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAIChatChoice {
    pub finish_reason: String,
    pub index: usize,
    #[serde(default)]
    pub content_filter_results: HashMap<String, OAIContentFilterResult>,
    pub message: OAIChatMessage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub system_fingerprint: Option<String>,
    pub choices: Vec<OAIChatChoice>,
    pub usage: Option<OAIUsage>,
    pub prompt_filter_results: Option<Vec<OAIPromptFilterResult>>,
}

#[derive(Debug)]
pub struct CollectedChoice {
    pub role: Option<String>,
    pub content: String,
    pub tool_calls: BTreeMap<usize, OAIToolCall>,
    pub tool_calls_order: Vec<usize>,
    pub finish_reason: Option<String>,
    pub content_filter_results: HashMap<String, OAIContentFilterResult>,
}

pub struct OAIStreamCollector {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<i64>,
    pub model: Option<String>,
    pub system_fingerprint: Option<String>,
    pub prompt_filter_results: Option<Vec<OAIPromptFilterResult>>,
    pub usage: Option<OAIUsage>,
    pub choices: BTreeMap<usize, CollectedChoice>,
}

impl Default for OAIStreamCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl OAIStreamCollector {
    pub fn new() -> Self {
        Self {
            id: None,
            object: None,
            created: None,
            model: None,
            system_fingerprint: None,
            prompt_filter_results: None,
            usage: None,
            choices: BTreeMap::new(),
        }
    }

    pub fn add_chunk(&mut self, chunk: &OAIStreamChunk) {
        for ch in chunk.choices.iter() {
            // Always ensure choice exists, even if all fields are absent!
            let idx = ch.index;
            let choice = self.choices.entry(idx).or_insert_with(|| CollectedChoice {
                role: None,
                content: String::new(),
                tool_calls: BTreeMap::new(),
                tool_calls_order: Vec::new(),
                finish_reason: None,
                content_filter_results: HashMap::new(),
            });

            if let Some(role) = &ch.delta.role {
                choice.role = Some(role.clone());
            }

            if let Some(c) = &ch.delta.content {
                choice.content.push_str(c);
            }

            for tc in &ch.delta.tool_calls {
                let ix = tc.index;
                let entry = choice.tool_calls.entry(ix).or_insert_with(|| tc.clone());
                // Always append arguments, regardless of what other fields are present - that's how OpenAI streams them
                // Merge tool_call fields as they arrive (Go-style). If the field is missing, retain the previous value.

                if let Some(name) = &tc.function.name {
                    entry.function.name = Some(name.clone());
                }
                entry.id = if let Some(s) = &tc.id {
                    if !s.is_empty() {
                        Some(s.clone())
                    } else {
                        entry.id.clone()
                    }
                } else {
                    entry.id.clone()
                };
                entry.type_ = if let Some(s) = &tc.type_ {
                    if !s.is_empty() {
                        Some(s.clone())
                    } else {
                        entry.type_.clone()
                    }
                } else {
                    entry.type_.clone()
                };
                // Only append non-empty fragments, guard against redundant final braces after JSON is complete
                if !tc.function.arguments.is_empty() {
                    // Skip appending fragments like '"}"' if the current arguments already ends correctly.
                    // This is a naive guard but works with broken completion fragments.
                    if !(tc.function.arguments == "\"}" && entry.function.arguments.ends_with('\"'))
                    {
                        entry.function.arguments.push_str(&tc.function.arguments);
                    }
                }
                if !choice.tool_calls_order.contains(&ix) {
                    choice.tool_calls_order.push(ix);
                }
            }

            if let Some(reason) = &ch.finish_reason {
                choice.finish_reason = Some(reason.clone());
            }
        }
    }

    pub fn build_response(self) -> OAIChatResponse {
        let mut choices = Vec::with_capacity(self.choices.len());
        for (idx, ch) in self.choices {
            let mut tool_calls = Vec::new();
            for ix in &ch.tool_calls_order {
                if let Some(tc) = ch.tool_calls.get(ix) {
                    tool_calls.push(tc.clone());
                }
            }
            let content = if ch.content.is_empty() {
                None
            } else {
                Some(ch.content)
            };
            choices.push(OAIChatChoice {
                finish_reason: ch.finish_reason.unwrap_or_default(),
                index: idx,
                content_filter_results: ch.content_filter_results,
                message: OAIChatMessage {
                    role: ch.role.unwrap_or_else(|| "assistant".to_string()),
                    content,
                    tool_calls,
                    padding: String::new(),
                },
            });
        }
        OAIChatResponse {
            id: self.id.unwrap_or_default(),
            object: self.object.unwrap_or_default(),
            created: self.created.unwrap_or(0),
            model: self.model.unwrap_or_default(),
            system_fingerprint: self.system_fingerprint,
            choices,
            usage: self.usage,
            prompt_filter_results: self.prompt_filter_results,
        }
    }
}
fn null_to_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

    const TOOL_STREAM: &str = r#"
data: {"choices":[],"created":0,"id":"","prompt_filter_results":[{"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"prompt_index":0}]}
data: {"choices":[{"index":0,"delta":{"content":null,"role":"assistant","tool_calls":[{"function":{"arguments":"","name":"get_weather"},"id":"call_7m75SYp4UrPhxhtdZdawEK5J","index":0,"type":"function"}]}}],"created":1747591235,"id":"chatcmpl-BYcbLSepxSXIxgUX2WZCFZrjqjp0l","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"arguments":"{\""},"index":0}]}}],"created":1747591235,"id":"chatcmpl-BYcbLSepxSXIxgUX2WZCFZrjqjp0l","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"arguments":"location"},"index":0}]}}],"created":1747591235,"id":"chatcmpl-BYcbLSepxSXIxgUX2WZCFZrjqjp0l","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"arguments":"\":\""},"index":0}]}}],"created":1747591235,"id":"chatcmpl-BYcbLSepxSXIxgUX2WZCFZrjqjp0l","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"arguments":"San"},"index":0}]}}],"created":1747591235,"id":"chatcmpl-BYcbLSepxSXIxgUX2WZCFZrjqjp0l","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"arguments":" Francisco"},"index":0}]}}],"created":1747591235,"id":"chatcmpl-BYcbLSepxSXIxgUX2WZCFZrjqjp0l","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"arguments":"\"}"},"index":0}]}}],"created":1747591235,"id":"chatcmpl-BYcbLSepxSXIxgUX2WZCFZrjqjp0l","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"finish_reason":"tool_calls","index":0,"delta":{"content":null}}],"created":1747591235,"id":"chatcmpl-BYcbLSepxSXIxgUX2WZCFZrjqjp0l","usage":{"completion_tokens":16,"completion_tokens_details":{"accepted_prediction_tokens":0,"rejected_prediction_tokens":0},"prompt_tokens":73,"prompt_tokens_details":{"cached_tokens":0},"total_tokens":89},"model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: [DONE]
"#;

    #[test]
    fn test_tool_call_streaming() {
        let mut collector = OAIStreamCollector::new();
        for line in TOOL_STREAM.lines() {
            // --- BEGIN GOOSE DEBUG ---
            let line = line.trim();
            if !line.starts_with("data: ") {
                continue;
            }
            let Some(payload) = line.get(6..) else {
                continue;
            };
            if payload == "[DONE]" {
                break;
            }
            let chunk: OAIStreamChunk = match from_str(payload) {
                Ok(c) => c,
                Err(e) => {
                    println!("JSON deserialize failed: {} | payload: {}", e, payload);
                    continue;
                }
            };
            println!("Parsed chunk. Choices length: {}", chunk.choices.len());
            collector.add_chunk(&chunk);
        }
        let resp = collector.build_response();
        assert_eq!(resp.choices.len(), 1);
        let choice = &resp.choices[0];
        assert_eq!(choice.message.role, "assistant");
        assert_eq!(choice.message.tool_calls.len(), 1);
        let tc = &choice.message.tool_calls[0];
        assert_eq!(tc.function.name.as_deref(), Some("get_weather"));
        assert_eq!(tc.function.arguments, r#"{"location":"San Francisco"}"#);
        assert_eq!(choice.finish_reason, "tool_calls");
    }

    const TEXT_STREAM: &str = r#"
data: {"choices":[],"created":0,"id":"","prompt_filter_results":[{"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"prompt_index":0}]}
data: {"choices":[{"index":0,"content_filter_offsets":{"check_offset":3458,"start_offset":3458,"end_offset":3494},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"delta":{"content":"","role":"assistant"}}],"created":1747592466,"id":"chatcmpl-BYcvCkaKJjQIM7e2j6vg08RIcY8qp","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"content_filter_offsets":{"check_offset":3458,"start_offset":3458,"end_offset":3494},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"delta":{"content":"Hello"}}],"created":1747592466,"id":"chatcmpl-BYcvCkaKJjQIM7e2j6vg08RIcY8qp","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"content_filter_offsets":{"check_offset":3458,"start_offset":3458,"end_offset":3494},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"delta":{"content":"!"}}],"created":1747592466,"id":"chatcmpl-BYcvCkaKJjQIM7e2j6vg08RIcY8qp","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"content_filter_offsets":{"check_offset":3458,"start_offset":3458,"end_offset":3494},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"delta":{"content":" How"}}],"created":1747592466,"id":"chatcmpl-BYcvCkaKJjQIM7e2j6vg08RIcY8qp","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"content_filter_offsets":{"check_offset":3458,"start_offset":3458,"end_offset":3494},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"delta":{"content":" can"}}],"created":1747592466,"id":"chatcmpl-BYcvCkaKJjQIM7e2j6vg08RIcY8qp","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"content_filter_offsets":{"check_offset":3458,"start_offset":3458,"end_offset":3494},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"delta":{"content":" I"}}],"created":1747592466,"id":"chatcmpl-BYcvCkaKJjQIM7e2j6vg08RIcY8qp","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"content_filter_offsets":{"check_offset":3458,"start_offset":3458,"end_offset":3494},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"delta":{"content":" assist"}}],"created":1747592466,"id":"chatcmpl-BYcvCkaKJjQIM7e2j6vg08RIcY8qp","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"content_filter_offsets":{"check_offset":3458,"start_offset":3458,"end_offset":3494},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"delta":{"content":" you"}}],"created":1747592466,"id":"chatcmpl-BYcvCkaKJjQIM7e2j6vg08RIcY8qp","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"content_filter_offsets":{"check_offset":3458,"start_offset":3458,"end_offset":3494},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"delta":{"content":" today"}}],"created":1747592466,"id":"chatcmpl-BYcvCkaKJjQIM7e2j6vg08RIcY8qp","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"content_filter_offsets":{"check_offset":3458,"start_offset":3458,"end_offset":3494},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"delta":{"content":"?"}}],"created":1747592466,"id":"chatcmpl-BYcvCkaKJjQIM7e2j6vg08RIcY8qp","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"index":0,"content_filter_offsets":{"check_offset":3458,"start_offset":3458,"end_offset":3494},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"delta":{"content":" 🌍"}}],"created":1747592466,"id":"chatcmpl-BYcvCkaKJjQIM7e2j6vg08RIcY8qp","model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: {"choices":[{"finish_reason":"stop","index":0,"content_filter_offsets":{"check_offset":3458,"start_offset":3458,"end_offset":3494},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"delta":{"content":null}}],"created":1747592466,"id":"chatcmpl-BYcvCkaKJjQIM7e2j6vg08RIcY8qp","usage":{"completion_tokens":13,"completion_tokens_details":{"accepted_prediction_tokens":0,"rejected_prediction_tokens":0},"prompt_tokens":1675,"prompt_tokens_details":{"cached_tokens":1536},"total_tokens":1688},"model":"gpt-4o-2024-11-20","system_fingerprint":"fp_ee1d74bde0"}
data: [DONE]
"#;

    #[test]
    fn test_text_streaming() {
        let mut collector = OAIStreamCollector::new();
        for line in TEXT_STREAM.lines() {
            let line = line.trim();
            if !line.starts_with("data: ") {
                continue;
            }
            let Some(payload) = line.get(6..) else {
                continue;
            };
            if payload == "[DONE]" {
                break;
            }
            let chunk: OAIStreamChunk = match from_str(payload) {
                Ok(c) => c,
                Err(e) => {
                    println!("JSON deserialize failed: {} | payload: {}", e, payload);
                    continue;
                }
            };
            collector.add_chunk(&chunk);
        }
        let resp = collector.build_response();
        assert_eq!(resp.choices.len(), 1);
        let choice = &resp.choices[0];
        assert_eq!(choice.message.role, "assistant");
        assert_eq!(
            choice.message.content.as_deref().unwrap_or(""),
            "Hello! How can I assist you today? 🌍"
        );
        assert_eq!(choice.finish_reason, "stop");
    }
    const CLAUDE_STREAM: &str = r#"
data: {"choices":[{"index":0,"delta":{"content":"I","role":"assistant"}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":"'ll","role":"assistant"}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":" help","role":"assistant"}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":" you examine","role":"assistant"}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":" the most","role":"assistant"}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":" recent commit using","role":"assistant"}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":" the shell","role":"assistant"}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":" comman","role":"assistant"}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":"d `git show HEAD","role":"assistant"}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":"`.","role":"assistant"}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"name":"developer__shell"},"id":"tooluse_9eC8o8MvTN-KOWuDGXgq1Q","index":0,"type":"function"}]}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"arguments":""},"index":0,"type":"function"}]}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"arguments":"{\"command"},"index":0,"type":"function"}]}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"arguments":"\": "},"index":0,"type":"function"}]}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"arguments":"\"git show H"},"index":0,"type":"function"}]}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"arguments":"EAD"},"index":0,"type":"function"}]}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"index":0,"delta":{"content":null,"tool_calls":[{"function":{"arguments":"\"}"},"index":0,"type":"function"}]}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","model":"claude-sonnet-4-20250514"}
data: {"choices":[{"finish_reason":"tool_calls","index":0,"delta":{"content":null}}],"created":1747613682,"id":"938bb8e2-6276-4a58-bca3-c675cfe7f2f5","usage":{"completion_tokens":56,"prompt_tokens":2594,"prompt_tokens_details":{"cached_tokens":0},"total_tokens":2650},"model":"claude-sonnet-4-20250514"}
data: [DONE]
"#;
    #[test]
    fn test_claude_streaming() {
        let mut collector = OAIStreamCollector::new();
        for line in CLAUDE_STREAM.lines() {
            let line = line.trim();
            if !line.starts_with("data: ") {
                continue;
            }
            let Some(payload) = line.get(6..) else {
                continue;
            };
            if payload == "[DONE]" {
                break;
            }
            let chunk: OAIStreamChunk = match from_str(payload) {
                Ok(c) => c,
                Err(e) => {
                    println!("JSON deserialize failed {} | payload: {}", e, payload);
                    continue;
                }
            };
            collector.add_chunk(&chunk);
        }
        let resp = collector.build_response();
        assert_eq!(resp.choices.len(), 1);
        let choice = &resp.choices[0];
        assert_eq!(choice.message.role, "assistant");
        assert_eq!(
            choice.message.content.as_deref().unwrap_or(""),
            "I'll help you examine the most recent commit using the shell command `git show HEAD`."
        );
        assert_eq!(choice.finish_reason, "tool_calls");
    }
}
