use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_PROTOCOL_LINE_BYTES: usize = 256 * 1024;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeEvent {
    Ready {
        version: u16,
        npub: String,
    },
    Message {
        version: u16,
        message_id: String,
        group_id: String,
        group_name: String,
        sender: String,
        content: String,
    },
    GroupJoined {
        version: u16,
        group_id: String,
        group_name: String,
    },
    Response {
        version: u16,
        request_id: String,
        ok: bool,
        error: Option<String>,
    },
}

impl BridgeEvent {
    pub fn version(&self) -> u16 {
        match self {
            Self::Ready { version, .. }
            | Self::Message { version, .. }
            | Self::GroupJoined { version, .. }
            | Self::Response { version, .. } => *version,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeCommand {
    Send {
        version: u16,
        request_id: String,
        group_id: String,
        text: String,
    },
    Complete {
        version: u16,
        request_id: String,
        message_id: String,
    },
}

impl BridgeCommand {
    pub fn send(request_id: String, group_id: String, text: String) -> Self {
        Self::Send {
            version: PROTOCOL_VERSION,
            request_id,
            group_id,
            text,
        }
    }

    pub fn complete(request_id: String, message_id: String) -> Self {
        Self::Complete {
            version: PROTOCOL_VERSION,
            request_id,
            message_id,
        }
    }

    pub fn request_id(&self) -> &str {
        match self {
            Self::Send { request_id, .. } | Self::Complete { request_id, .. } => request_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_message_event() {
        let event: BridgeEvent = serde_json::from_str(
            r#"{"type":"message","version":1,"message_id":"event","group_id":"group","group_name":"Session","sender":"npub1sender","content":"hello"}"#,
        )
        .unwrap();

        assert_eq!(event.version(), PROTOCOL_VERSION);
        assert!(matches!(event, BridgeEvent::Message { content, .. } if content == "hello"));
    }

    #[test]
    fn serializes_send_command() {
        let value = serde_json::to_value(BridgeCommand::send(
            "request".into(),
            "group".into(),
            "hello".into(),
        ))
        .unwrap();

        assert_eq!(value["type"], "send");
        assert_eq!(value["version"], PROTOCOL_VERSION);
    }
}
