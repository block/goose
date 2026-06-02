use super::*;

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $str:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($variant),+ }

        impl FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_lowercase().as_str() {
                    $($str => Ok(Self::$variant),)+
                    other => Err(format!("unknown {}: '{other}'", stringify!($name))),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self { $(Self::$variant => write!(f, $str),)+ }
            }
        }
    }
}

string_enum!(ThinkingType { Adaptive => "adaptive", Enabled => "enabled", Disabled => "disabled" });

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnthropicFormatOptions {
    pub preserve_unsigned_thinking: bool,
    pub preserve_thinking_context: bool,
}

impl AnthropicFormatOptions {
    pub(super) fn for_model(self, model_config: &ModelConfig) -> Self {
        let preserve_thinking_context = model_config
            .get_config_param::<bool>(
                "preserve_thinking_context",
                "ANTHROPIC_PRESERVE_THINKING_CONTEXT",
            )
            .unwrap_or(self.preserve_thinking_context);
        let preserve_unsigned_thinking = model_config
            .get_config_param::<bool>(
                "preserve_unsigned_thinking",
                "ANTHROPIC_PRESERVE_UNSIGNED_THINKING",
            )
            .unwrap_or(self.preserve_unsigned_thinking)
            || preserve_thinking_context;

        Self {
            preserve_unsigned_thinking,
            preserve_thinking_context,
        }
    }
}

// Constants for frequently used strings in Anthropic API format
pub(super) const TYPE_FIELD: &str = "type";
pub(super) const CONTENT_FIELD: &str = "content";
pub(super) const TEXT_TYPE: &str = "text";
pub(super) const ROLE_FIELD: &str = "role";
pub(super) const USER_ROLE: &str = "user";
pub(super) const ASSISTANT_ROLE: &str = "assistant";
pub(super) const TOOL_USE_TYPE: &str = "tool_use";
pub(super) const TOOL_RESULT_TYPE: &str = "tool_result";
pub(super) const THINKING_TYPE: &str = "thinking";
pub(super) const REDACTED_THINKING_TYPE: &str = "redacted_thinking";
pub(super) const CACHE_CONTROL_FIELD: &str = "cache_control";
pub(super) const ID_FIELD: &str = "id";
pub(super) const NAME_FIELD: &str = "name";
pub(super) const INPUT_FIELD: &str = "input";
pub(super) const TOOL_USE_ID_FIELD: &str = "tool_use_id";
pub(super) const IS_ERROR_FIELD: &str = "is_error";
pub(super) const SIGNATURE_FIELD: &str = "signature";
pub(super) const DATA_FIELD: &str = "data";
pub(super) const EVENT_MESSAGE_START: &str = "message_start";
pub(super) const EVENT_MESSAGE_DELTA: &str = "message_delta";
pub(super) const EVENT_MESSAGE_STOP: &str = "message_stop";
pub(super) const EVENT_CONTENT_BLOCK_START: &str = "content_block_start";
pub(super) const EVENT_CONTENT_BLOCK_DELTA: &str = "content_block_delta";
pub(super) const EVENT_CONTENT_BLOCK_STOP: &str = "content_block_stop";
