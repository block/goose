use crate::error::{GooseClientError, Result};
use crate::types::events::MessageEvent;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

const MAX_SSE_BUFFER_BYTES: usize = 10 * 1024 * 1024;

/// Wraps a `reqwest` byte stream and parses it into `MessageEvent` items.
///
/// goose-server sends SSE in the simple format: `data: {json}\n\n`
/// with no `event:` or `id:` fields. Each event is terminated by a blank line.
pub(crate) struct SseStream {
    inner: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>,
    buffer: Vec<u8>,
}

impl SseStream {
    pub(crate) fn new(stream: impl Stream<Item = reqwest::Result<Bytes>> + Send + 'static) -> Self {
        Self {
            inner: Box::pin(stream),
            buffer: Vec::new(),
        }
    }
}

impl Stream for SseStream {
    type Item = Result<MessageEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let Some(event) = extract_event(&mut self.buffer) {
                return Poll::Ready(Some(event));
            }

            match self.inner.as_mut().poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => {
                    if !self.buffer.is_empty() {
                        let remaining = std::mem::take(&mut self.buffer);
                        if let Some(event) = parse_sse_line(&remaining) {
                            return Poll::Ready(Some(event));
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(GooseClientError::Http(e))));
                }
                Poll::Ready(Some(Ok(chunk))) => {
                    self.buffer.extend_from_slice(&chunk);
                }
            }
        }
    }
}

fn extract_event(buffer: &mut Vec<u8>) -> Option<Result<MessageEvent>> {
    let delimiter = match find_event_delimiter(buffer) {
        Some(d) => d,
        None => {
            if buffer.len() > MAX_SSE_BUFFER_BYTES {
                return Some(Err(GooseClientError::Stream(
                    "SSE buffer exceeded 10 MB without a complete event".to_string(),
                )));
            }
            return None;
        }
    };
    let event_bytes = buffer[..delimiter].to_vec();
    buffer.drain(..delimiter + 2);
    parse_sse_line(&event_bytes)
}

fn find_event_delimiter(buffer: &[u8]) -> Option<usize> {
    (0..buffer.len().saturating_sub(1)).find(|&i| buffer[i] == b'\n' && buffer[i + 1] == b'\n')
}

fn parse_sse_line(line: &[u8]) -> Option<Result<MessageEvent>> {
    let text = std::str::from_utf8(line).ok()?.trim();
    if text.is_empty() {
        return None;
    }
    let data = text.strip_prefix("data: ").unwrap_or(text);
    if data.is_empty() {
        return None;
    }
    Some(serde_json::from_str::<MessageEvent>(data).map_err(GooseClientError::Deserialization))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    fn bytes_stream(
        chunks: Vec<String>,
    ) -> impl Stream<Item = reqwest::Result<Bytes>> + Send + 'static {
        futures::stream::iter(chunks.into_iter().map(|s| Ok(Bytes::from(s))))
    }

    fn chunks(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|s| s.to_string()).collect()
    }

    #[tokio::test]
    async fn test_parses_single_event() {
        let stream = bytes_stream(chunks(&["data: {\"type\":\"Ping\"}\n\n"]));
        let mut sse = SseStream::new(stream);
        let event = sse.next().await.unwrap().unwrap();
        assert!(matches!(event, MessageEvent::Ping));
    }

    #[tokio::test]
    async fn test_parses_multiple_events_in_one_chunk() {
        let stream = bytes_stream(chunks(&[
            "data: {\"type\":\"Ping\"}\n\ndata: {\"type\":\"Ping\"}\n\n",
        ]));
        let mut sse = SseStream::new(stream);
        assert!(matches!(
            sse.next().await.unwrap().unwrap(),
            MessageEvent::Ping
        ));
        assert!(matches!(
            sse.next().await.unwrap().unwrap(),
            MessageEvent::Ping
        ));
        assert!(sse.next().await.is_none());
    }

    #[tokio::test]
    async fn test_parses_event_split_across_chunks() {
        let stream = bytes_stream(chunks(&["data: {\"type\":", "\"Ping\"}\n\n"]));
        let mut sse = SseStream::new(stream);
        assert!(matches!(
            sse.next().await.unwrap().unwrap(),
            MessageEvent::Ping
        ));
    }

    #[tokio::test]
    async fn test_parses_finish_event() {
        let json = r#"{"type":"Finish","reason":"stop","token_state":{"inputTokens":10,"outputTokens":5,"totalTokens":15,"accumulatedInputTokens":10,"accumulatedOutputTokens":5,"accumulatedTotalTokens":15}}"#;
        let stream = bytes_stream(chunks(&[&format!("data: {json}\n\n")]));
        let mut sse = SseStream::new(stream);
        let event = sse.next().await.unwrap().unwrap();
        assert!(matches!(event, MessageEvent::Finish { reason, .. } if reason == "stop"));
    }

    #[tokio::test]
    async fn test_parses_error_event() {
        let stream = bytes_stream(chunks(&[
            "data: {\"type\":\"Error\",\"error\":\"something went wrong\"}\n\n",
        ]));
        let mut sse = SseStream::new(stream);
        let event = sse.next().await.unwrap().unwrap();
        assert!(matches!(event, MessageEvent::Error { error } if error == "something went wrong"));
    }

    #[tokio::test]
    async fn test_returns_deserialization_error_for_invalid_json() {
        let stream = bytes_stream(chunks(&["data: not-json\n\n"]));
        let mut sse = SseStream::new(stream);
        let result = sse.next().await.unwrap();
        assert!(matches!(result, Err(GooseClientError::Deserialization(_))));
    }
}
