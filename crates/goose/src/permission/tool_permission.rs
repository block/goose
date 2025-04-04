use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ToolPermission {
    AlwaysAllow,
    AllowOnce,
    AlwaysDeny,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolPermissionConfirmation {
    pub tool_name: String,
    pub permission: ToolPermission,
}
