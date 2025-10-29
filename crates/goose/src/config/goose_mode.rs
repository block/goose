use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GooseMode {
    Auto,
    Approve,
    SmartApprove,
    Chat,
}

impl TryFrom<&String> for GooseMode {
    type Error = String;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "auto" => Ok(GooseMode::Auto),
            "approve" => Ok(GooseMode::Approve),
            "smart_approve" => Ok(GooseMode::SmartApprove),
            "chat" => Ok(GooseMode::Chat),
            _ => Err(format!("invalid mode: {}", value)),
        }
    }
}
