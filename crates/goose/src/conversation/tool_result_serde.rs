use mcp_core::handler::{ToolError, ToolResult};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

// For deserialization, let's use a simpler approach that works with the format we're serializing to
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<ToolResult<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    // Define a helper enum to handle the two possible formats
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ResultFormat<T> {
        Success { status: String, value: T },
        Error { status: String, error: String },
    }

    let format = ResultFormat::deserialize(deserializer)?;

    match format {
        ResultFormat::Success { status, value } => {
            if status == "success" {
                Ok(Ok(value))
            } else {
                Err(serde::de::Error::custom(format!(
                    "Expected status 'success', got '{}'",
                    status
                )))
            }
        }
        ResultFormat::Error { status, error } => {
            if status == "error" {
                Ok(Err(ToolError::ExecutionError(error)))
            } else {
                Err(serde::de::Error::custom(format!(
                    "Expected status 'error', got '{}'",
                    status
                )))
            }
        }
    }
}
