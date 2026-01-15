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
            task: None,
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

        let original_value = serde_json::Value::deserialize(deserializer)?;

        let format = ResultFormat::deserialize(&original_value).map_err(|e| {
            tracing::debug!(
                "Failed to deserialize call_tool_result: {}. Original data: {}",
                e,
                serde_json::to_string(&original_value)
                    .unwrap_or_else(|_| "<invalid json>".to_string())
            );
            serde::de::Error::custom(e)
        })?;

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

    pub fn validate(result: ToolResult<CallToolResult>) -> ToolResult<CallToolResult> {
        match &result {
            Ok(call_tool_result) => match serde_json::to_string(call_tool_result) {
                Ok(json_str) => match serde_json::from_str::<CallToolResult>(&json_str) {
                    Ok(_) => result,
                    Err(e) => {
                        tracing::error!("CallToolResult failed validation by deserialization: {}. Original data: {}", e, json_str);
                        Err(ErrorData {
                            code: ErrorCode::INTERNAL_ERROR,
                            message: Cow::from(format!("Tool result validation failed: {}", e)),
                            data: None,
                        })
                    }
                },
                Err(e) => {
                    tracing::error!("CallToolResult failed serialization: {}", e);
                    Err(ErrorData {
                        code: ErrorCode::INTERNAL_ERROR,
                        message: Cow::from(format!("Tool result serialization failed: {}", e)),
                        data: None,
                    })
                }
            },
            Err(_) => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{CallToolResult, Content, ErrorCode, ErrorData};
    use std::borrow::Cow;
    #[test]
    fn test_validate_accepts_valid_call_tool_result() {
        let valid_result = CallToolResult {
            content: vec![Content::text("test")],
            is_error: Some(false),
            structured_content: None,
            meta: None,
        };

        let tool_result: ToolResult<CallToolResult> = Ok(valid_result);
        let validated = call_tool_result::validate(tool_result);

        assert!(
            validated.is_ok(),
            "Expected validation to pass for valid CallToolResult"
        );
    }
    #[test]
    fn test_validate_returns_error_for_invalid_calltoolresult() {
        let valid_result = CallToolResult {
            content: vec![],
            is_error: Some(false),
            structured_content: None,
            meta: None,
        };

        let tool_result: ToolResult<CallToolResult> = Ok(valid_result);
        let validated = call_tool_result::validate(tool_result);

        assert!(validated.is_err());
        assert!(validated
            .unwrap_err()
            .message
            .contains("Tool result validation failed"))
    }

    #[test]
    fn test_validate_passes_through_errors() {
        let error_result: ToolResult<CallToolResult> = Err(ErrorData {
            code: ErrorCode::INTERNAL_ERROR,
            message: Cow::from("test error"),
            data: None,
        });

        let validated = call_tool_result::validate(error_result.clone());

        assert!(validated.is_err());
        assert_eq!(validated.unwrap_err().message, "test error");
    }
}
