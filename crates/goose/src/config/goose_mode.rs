use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GooseMode {
    Auto,
    Approve,
    SmartApprove,
    Chat,
}

impl FromStr for GooseMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(GooseMode::Auto),
            "approve" => Ok(GooseMode::Approve),
            "smart_approve" => Ok(GooseMode::SmartApprove),
            "chat" => Ok(GooseMode::Chat),
            _ => Err(format!("invalid mode: {}", s)),
        }
    }
}
