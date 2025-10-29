use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GooseMode {
    Auto,
    Approve,
    SmartApprove,
    Chat,
}

impl Display for GooseMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GooseMode::Auto => write!(f, "auto"),
            GooseMode::Approve => write!(f, "approve"),
            GooseMode::SmartApprove => write!(f, "smart_approve"),
            GooseMode::Chat => write!(f, "chat"),
        }
    }
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
