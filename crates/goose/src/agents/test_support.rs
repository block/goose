use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::stream;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use rmcp::model::Tool;
use tokio::sync::{mpsc, Mutex as AsyncMutex, Notify};

use super::provider_stream_coordinator::ProviderStreamItem;
use crate::conversation::message::Message;
use crate::providers::base::{MessageStream, Provider};

pub(crate) struct NativeSteeringTestProvider {
    streams: AsyncMutex<VecDeque<MessageStream>>,
    native_behaviors: AsyncMutex<VecDeque<NativeSteeringBehavior>>,
    prompts: Mutex<Vec<Vec<Message>>>,
    native_messages: Mutex<Vec<String>>,
    pub(crate) stream_calls: AtomicUsize,
    pub(crate) native_calls: AtomicUsize,
    stream_called: Notify,
    native_called: Notify,
}

pub(crate) enum NativeSteeringBehavior {
    Immediate(Result<bool, ProviderError>),
    Blocked {
        started: Arc<Notify>,
        release: Arc<Notify>,
        result: Result<bool, ProviderError>,
    },
}

impl NativeSteeringTestProvider {
    pub(crate) fn new(
        streams: impl IntoIterator<Item = MessageStream>,
        native_results: impl IntoIterator<Item = Result<bool, ProviderError>>,
    ) -> Arc<Self> {
        Self::with_streams_and_behaviors(
            streams,
            native_results
                .into_iter()
                .map(NativeSteeringBehavior::Immediate),
        )
    }

    pub(crate) fn with_behaviors(
        behaviors: impl IntoIterator<Item = NativeSteeringBehavior>,
    ) -> Arc<Self> {
        Self::with_streams_and_behaviors([], behaviors)
    }

    fn with_streams_and_behaviors(
        streams: impl IntoIterator<Item = MessageStream>,
        behaviors: impl IntoIterator<Item = NativeSteeringBehavior>,
    ) -> Arc<Self> {
        Arc::new(Self {
            streams: AsyncMutex::new(streams.into_iter().collect()),
            native_behaviors: AsyncMutex::new(behaviors.into_iter().collect()),
            prompts: Mutex::new(Vec::new()),
            native_messages: Mutex::new(Vec::new()),
            stream_calls: AtomicUsize::new(0),
            native_calls: AtomicUsize::new(0),
            stream_called: Notify::new(),
            native_called: Notify::new(),
        })
    }

    pub(crate) async fn wait_for_stream_calls(&self, expected: usize) {
        while self.stream_calls.load(Ordering::SeqCst) < expected {
            self.stream_called.notified().await;
        }
    }

    pub(crate) async fn wait_for_native_calls(&self, expected: usize) {
        while self.native_calls.load(Ordering::SeqCst) < expected {
            self.native_called.notified().await;
        }
    }

    pub(crate) fn prompts(&self) -> Vec<Vec<Message>> {
        self.prompts.lock().expect("prompts lock").clone()
    }

    pub(crate) fn native_messages(&self) -> Vec<String> {
        self.native_messages
            .lock()
            .expect("native messages lock")
            .clone()
    }
}

#[async_trait]
impl Provider for NativeSteeringTestProvider {
    fn get_name(&self) -> &str {
        "native-steering-test"
    }

    async fn stream(
        &self,
        _model_config: &ModelConfig,
        _system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        self.stream_calls.fetch_add(1, Ordering::SeqCst);
        self.prompts
            .lock()
            .expect("prompts lock")
            .push(messages.to_vec());
        self.stream_called.notify_one();
        self.streams
            .lock()
            .await
            .pop_front()
            .ok_or_else(|| ProviderError::ExecutionError("unexpected provider prompt".into()))
    }

    async fn steer_natively(
        &self,
        _session_id: &str,
        message: &Message,
    ) -> Result<bool, ProviderError> {
        self.native_calls.fetch_add(1, Ordering::SeqCst);
        self.native_messages
            .lock()
            .expect("native messages lock")
            .push(message.as_concat_text());
        self.native_called.notify_one();
        match self
            .native_behaviors
            .lock()
            .await
            .pop_front()
            .expect("native steering result")
        {
            NativeSteeringBehavior::Immediate(result) => result,
            NativeSteeringBehavior::Blocked {
                started,
                release,
                result,
            } => {
                started.notify_one();
                release.notified().await;
                result
            }
        }
    }
}

pub(crate) fn controlled_stream() -> (mpsc::UnboundedSender<ProviderStreamItem>, MessageStream) {
    let (tx, rx) = mpsc::unbounded_channel();
    let stream = stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });
    (tx, Box::pin(stream))
}
