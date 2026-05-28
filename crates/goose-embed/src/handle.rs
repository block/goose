//! Stream type returned from [`crate::Goose::reply`].

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::BoxStream;
use futures::Stream;

use goose::agents::AgentEvent;

/// Stream of [`AgentEvent`]s produced by an embedded agent.
///
/// Yielded by [`crate::Goose::reply`]. The stream borrows from the parent
/// [`crate::Goose`] handle, so keep the handle alive for as long as you're
/// reading from the stream.
pub struct ReplyStream<'a> {
    pub(crate) inner: BoxStream<'a, anyhow::Result<AgentEvent>>,
}

impl<'a> Stream for ReplyStream<'a> {
    type Item = anyhow::Result<AgentEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}
