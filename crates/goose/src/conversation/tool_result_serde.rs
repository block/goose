use crate::mcp_utils::ToolResult;
use rmcp::model::{CallToolRequestParam, ErrorCode, ErrorData, JsonObject};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

pub fn serialize<T, S>(value: &ToolResult<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    match value {
        Ok(val) => {
            let mut state = serializer.serialize_struct("ToolResult", 2)?;
            state.serialize_field("status", "success")?;
            state.serialize_field("value", val)?;
            state.end()
        }
        Err(err) => {
            let mut state = serializer.serialize_struct("ToolResult", 2)?;
            state.serialize_field("status", "error")?;
            state.serialize_field("error", &err.to_string())?;
            state.end()
        }
    }
}

#[derive(Deserialize)]
struct ToolCallWithValueArguments {
    name: String,
    arguments: serde_json::Value,
}

impl ToolCallWithValueArguments {
    fn into_call_tool_request_param(self) -> CallToolRequestParam {
        let arguments = match self.arguments {
            serde_json::Value::Object(map) => Some(map),
            serde_json::Value::Null => None,
            other => {
                let mut map = JsonObject::new();
                map.insert("value".to_string(), other);
                Some(map)
            }
        };
        CallToolRequestParam {
            name: Cow::Owned(self.name),
            arguments,
        }
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<ToolResult<CallToolRequestParam>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ResultFormat {
        SuccessWithCallToolRequestParam {
            status: String,
            value: CallToolRequestParam,
        },
        SuccessWithToolCallValueArguments {
            status: String,
            value: ToolCallWithValueArguments,
        },
        Error {
            status: String,
            error: String,
        },
    }

    let format = ResultFormat::deserialize(deserializer)?;

    match format {
        ResultFormat::SuccessWithCallToolRequestParam { status, value } => {
            if status == "success" {
                Ok(Ok(value))
            } else {
                Err(serde::de::Error::custom(format!(
                    "Expected status 'success', got '{}'",
                    status
                )))
            }
        }
        ResultFormat::SuccessWithToolCallValueArguments { status, value } => {
            if status == "success" {
                Ok(Ok(value.into_call_tool_request_param()))
            } else {
                Err(serde::de::Error::custom(format!(
                    "Expected status 'success', got '{}'",
                    status
                )))
            }
        }
        ResultFormat::Error { status, error } => {
            if status == "error" {
                Ok(Err(ErrorData {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: Cow::from(error),
                    data: None,
                }))
            } else {
                Err(serde::de::Error::custom(format!(
                    "Expected status 'error', got '{}'",
                    status
                )))
            }
        }
    }
}

pub mod call_tool_result {
    use super::*;
    use rmcp::model::{CallToolResult, Content};

    pub fn serialize<S>(
        value: &ToolResult<CallToolResult>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        super::serialize(value, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ToolResult<CallToolResult>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ResultFormat {
            SuccessWithCallToolResult {
                status: String,
                value: CallToolResult,
            },
            SuccessWithContentVec {
                status: String,
                value: Vec<Content>,
            },
            Error {
                status: String,
                error: String,
            },
        }

        let format = ResultFormat::deserialize(deserializer)?;

        match format {
            ResultFormat::SuccessWithCallToolResult { status, value } => {
                if status == "success" {
                    Ok(Ok(value))
                } else {
                    Err(serde::de::Error::custom(format!(
                        "Expected status 'success', got '{}'",
                        status
                    )))
                }
            }
            ResultFormat::SuccessWithContentVec { status, value } => {
                if status == "success" {
                    Ok(Ok(CallToolResult::success(value)))
                } else {
                    Err(serde::de::Error::custom(format!(
                        "Expected status 'success', got '{}'",
                        status
                    )))
                }
            }
            ResultFormat::Error { status, error } => {
                if status == "error" {
                    Ok(Err(ErrorData {
                        code: ErrorCode::INTERNAL_ERROR,
                        message: Cow::from(error),
                        data: None,
                    }))
                } else {
                    Err(serde::de::Error::custom(format!(
                        "Expected status 'error', got '{}'",
                        status
                    )))
                }
            }
        }
    }
}
