use serde::{Deserialize, Serialize};
use strum::{Display, EnumMessage, EnumString, IntoStaticStr, VariantNames};

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Eq,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Display,
    EnumMessage,
    EnumString,
    IntoStaticStr,
    VariantNames,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum GooseMode {
    #[strum(message = "Automatically approve tool calls")]
    Auto,
    #[strum(message = "Ask before every tool call")]
    Approve,
    /// Default for Avocado Work consumer distro — ask before sensitive tool calls.
    #[default]
    #[strum(message = "Ask only for sensitive tool calls")]
    SmartApprove,
    #[strum(message = "Chat only, no tool calls")]
    Chat,
}
