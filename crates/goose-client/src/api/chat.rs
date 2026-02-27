use crate::error::Result;
use crate::streaming::SseStream;
use crate::types::events::MessageEvent;
use crate::types::requests::ChatRequest;
use crate::GooseClient;
use futures::Stream;
use goose::conversation::message::Message;

impl GooseClient {
    /// Send a message and receive a streaming reply.
    ///
    /// Returns a `Stream` of `MessageEvent` items. The stream ends with a
    /// `MessageEvent::Finish` event. `MessageEvent::Ping` heartbeats are included
    /// and may be filtered by the caller if not needed.
    pub async fn reply(
        &self,
        request: ChatRequest,
    ) -> Result<impl Stream<Item = Result<MessageEvent>>> {
        let resp = self.http.post_streaming("/reply", &request).await?;
        Ok(SseStream::new(resp.bytes_stream()))
    }

    /// Convenience wrapper: send a plain text message to an existing session.
    pub async fn send_message(
        &self,
        session_id: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<impl Stream<Item = Result<MessageEvent>>> {
        let request = ChatRequest::new(session_id, Message::user().with_text(text.into()));
        self.reply(request).await
    }
}
