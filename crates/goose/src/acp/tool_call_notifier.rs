use agent_client_protocol::schema::v1::{
    SessionId, SessionNotification, SessionUpdate, ToolCallUpdate,
};
use agent_client_protocol::{Client, ConnectionTo};

#[derive(Clone)]
pub(crate) struct ToolCallNotifier {
    connection: ConnectionTo<Client>,
    session_id: SessionId,
}

impl ToolCallNotifier {
    pub(crate) fn new(connection: ConnectionTo<Client>, session_id: SessionId) -> Self {
        Self {
            connection,
            session_id,
        }
    }

    pub(crate) fn send_update(
        &self,
        update: ToolCallUpdate,
    ) -> Result<(), agent_client_protocol::Error> {
        self.connection.send_notification(SessionNotification::new(
            self.session_id.clone(),
            SessionUpdate::ToolCallUpdate(update),
        ))
    }
}
