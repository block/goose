use agent_client_protocol::schema::v1::SessionNotification;
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub(crate) struct SessionBroadcastEvent {
    pub(crate) source_id: String,
    pub(crate) notification: SessionNotification,
}

pub(crate) struct SessionBroadcastHub {
    tx: broadcast::Sender<SessionBroadcastEvent>,
}

impl SessionBroadcastHub {
    pub(crate) fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<SessionBroadcastEvent> {
        self.tx.subscribe()
    }

    pub(crate) fn publish(&self, source_id: &str, notification: SessionNotification) {
        let _ = self.tx.send(SessionBroadcastEvent {
            source_id: source_id.to_string(),
            notification,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::{
        ContentBlock, ContentChunk, SessionId, SessionUpdate, TextContent,
    };

    fn notification(text: &str) -> SessionNotification {
        SessionNotification::new(
            SessionId::new("session-a"),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new(text),
            ))),
        )
    }

    #[tokio::test]
    async fn broadcasts_live_events_to_subscribers() {
        let hub = SessionBroadcastHub::new();
        let mut first = hub.subscribe();
        let mut second = hub.subscribe();

        hub.publish("source-a", notification("hello"));

        let first_event = first.recv().await.unwrap();
        let second_event = second.recv().await.unwrap();

        assert_eq!(first_event.source_id, "source-a");
        assert_eq!(second_event.source_id, "source-a");
        assert_eq!(
            serde_json::to_value(first_event.notification).unwrap()["update"]["content"]["text"],
            "hello"
        );
    }
}
