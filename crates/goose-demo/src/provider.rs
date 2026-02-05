//! LLM provider abstraction with runtime provider selection.
//!
//! This module provides a `ModelDyn` trait for dynamic dispatch across providers,
//! allowing runtime selection of the LLM provider without generics infection.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::{Future, Stream, StreamExt};
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{
    CompletionError, CompletionModel, CompletionRequest, CompletionResponse, GetTokenUsage, Usage,
};
use rig::message::ToolCall;
use rig::providers::{anthropic, openai};
use rig::streaming::StreamedAssistantContent;
use tokio::sync::mpsc;
use tracing::debug;

// ============================================================================
// Provider Configuration
// ============================================================================

/// Provider configuration loaded from environment
#[derive(Clone)]
pub struct ProviderConfig {
    pub provider: String,
    pub model: String,
}

impl ProviderConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let provider = std::env::var("GOOSE_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        let model = std::env::var("GOOSE_MODEL").unwrap_or_else(|_| match provider.as_str() {
            "anthropic" => "claude-sonnet-4-20250514".to_string(),
            _ => "gpt-4o".to_string(),
        });

        Self { provider, model }
    }
}

// ============================================================================
// Unified Stream Types
// ============================================================================

/// A chunk from a streaming completion - our unified type across all providers.
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Text content
    Text(String),
    /// A complete tool call
    ToolCall(ToolCall),
    /// Reasoning content (for models that support it)
    Reasoning(String),
    /// Stream finished with optional usage stats
    Done { usage: Option<Usage> },
}

/// A stream of chunks - type-erased so it works with any provider.
pub type ChunkStream = Pin<Box<dyn Stream<Item = Result<StreamChunk, CompletionError>> + Send>>;

// ============================================================================
// Dynamic Model Trait
// ============================================================================

/// A boxed future that is Send - used for trait object compatibility.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Trait for dynamic dispatch of completion models.
///
/// This provides the minimal interface needed for runtime provider selection.
/// Any rig `CompletionModel` automatically implements this via the blanket impl.
pub trait ModelDyn: Send + Sync {
    /// Generate a completion (non-streaming).
    fn completion(
        &self,
        request: CompletionRequest,
    ) -> BoxFuture<'_, Result<CompletionResponse<()>, CompletionError>>;

    /// Generate a streaming completion, returning a unified stream of chunks.
    fn stream(&self, request: CompletionRequest) -> BoxFuture<'_, Result<ChunkStream, CompletionError>>;
}

/// Blanket implementation: any rig `CompletionModel` can be used as a `ModelDyn`.
impl<M> ModelDyn for M
where
    M: CompletionModel + Send + Sync,
    M::StreamingResponse: Clone + Unpin + GetTokenUsage + Send + 'static,
{
    fn completion(
        &self,
        request: CompletionRequest,
    ) -> BoxFuture<'_, Result<CompletionResponse<()>, CompletionError>> {
        Box::pin(async move {
            let resp = CompletionModel::completion(self, request).await?;
            Ok(CompletionResponse {
                choice: resp.choice,
                usage: resp.usage,
                raw_response: (),
            })
        })
    }

    fn stream(&self, request: CompletionRequest) -> BoxFuture<'_, Result<ChunkStream, CompletionError>> {
        Box::pin(async move {
            let stream = CompletionModel::stream(self, request).await?;
            
            // Wrap the provider-specific stream in our unified ChunkStream
            let unified: ChunkStream = Box::pin(UnifiedStream { inner: stream });
            Ok(unified)
        })
    }
}

/// Wrapper that converts a provider-specific stream to our unified `StreamChunk` type.
struct UnifiedStream<R>
where
    R: Clone + Unpin + GetTokenUsage,
{
    inner: rig::streaming::StreamingCompletionResponse<R>,
}

impl<R> Stream for UnifiedStream<R>
where
    R: Clone + Unpin + GetTokenUsage,
{
    type Item = Result<StreamChunk, CompletionError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => {
                // Stream ended - get usage from the response if available
                let usage = self.inner.response.as_ref().and_then(|r| r.token_usage());
                Poll::Ready(Some(Ok(StreamChunk::Done { usage })))
            }
            Poll::Ready(Some(Ok(content))) => {
                let chunk = match content {
                    StreamedAssistantContent::Text(t) => StreamChunk::Text(t.text),
                    StreamedAssistantContent::ToolCall(tc) => StreamChunk::ToolCall(tc),
                    StreamedAssistantContent::Reasoning(r) => {
                        StreamChunk::Reasoning(r.reasoning.join(""))
                    }
                    StreamedAssistantContent::ReasoningDelta { reasoning, .. } => {
                        StreamChunk::Reasoning(reasoning)
                    }
                    // Ignore tool call deltas and final response (we handle final in None case)
                    StreamedAssistantContent::ToolCallDelta { .. } => {
                        // Re-poll to get next meaningful chunk
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    StreamedAssistantContent::Final(_) => {
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                };
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
        }
    }
}

// ============================================================================
// Model wrapper for convenient usage
// ============================================================================

/// Dynamic model wrapper that works with any provider at runtime.
///
/// This wraps a `dyn ModelDyn` trait object, allowing runtime provider selection
/// without requiring generics throughout your codebase.
pub struct Model {
    inner: Arc<dyn ModelDyn>,
}

impl Model {
    /// Create a model from provider configuration.
    ///
    /// Adding a new provider is just one line - create the client and wrap the model.
    pub fn from_config(config: &ProviderConfig) -> crate::Result<Self> {
        let inner: Arc<dyn ModelDyn> = match config.provider.as_str() {
            "openai" => Arc::new(openai::Client::from_env().completion_model(&config.model)),
            "anthropic" => Arc::new(anthropic::Client::from_env().completion_model(&config.model)),
            // Add more providers here - each is just one line:
            // "gemini" => Arc::new(gemini::Client::from_env().completion_model(&config.model)),
            // "cohere" => Arc::new(cohere::Client::from_env().completion_model(&config.model)),
            // "together" => Arc::new(together::Client::from_env().completion_model(&config.model)),
            // "groq" => Arc::new(groq::Client::from_env().completion_model(&config.model)),
            // "deepseek" => Arc::new(deepseek::Client::from_env().completion_model(&config.model)),
            // "xai" => Arc::new(xai::Client::from_env().completion_model(&config.model)),
            // "ollama" => Arc::new(ollama::Client::from_env().completion_model(&config.model)),
            other => {
                return Err(crate::Error::Provider(format!(
                    "Unknown provider: {}",
                    other
                )))
            }
        };

        Ok(Self { inner })
    }

    /// Generate a non-streaming completion.
    pub async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse<()>, CompletionError> {
        self.inner.completion(request).await
    }

    /// Generate a streaming completion, returning a stream of unified chunks.
    pub async fn stream(&self, request: CompletionRequest) -> Result<ChunkStream, CompletionError> {
        self.inner.stream(request).await
    }

    /// Stream a completion request, sending chunks through a channel.
    ///
    /// Returns a receiver for chunks and a handle to await the final result.
    /// This is useful when you want to process chunks in a separate task.
    pub fn stream_with_channel(
        &self,
        request: CompletionRequest,
    ) -> (mpsc::Receiver<StreamChunk>, StreamHandle) {
        let (tx, rx) = mpsc::channel(32);
        let inner = Arc::clone(&self.inner);

        let handle = StreamHandle {
            inner: tokio::spawn(async move {
                let mut stream = inner.stream(request).await?;
                let mut tool_calls = Vec::new();
                let mut text = String::new();

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(StreamChunk::Text(t)) => {
                            text.push_str(&t);
                            if tx.send(StreamChunk::Text(t)).await.is_err() {
                                debug!("Stream receiver dropped, ending early");
                                break;
                            }
                        }
                        Ok(StreamChunk::ToolCall(tc)) => {
                            tool_calls.push(tc.clone());
                            if tx.send(StreamChunk::ToolCall(tc)).await.is_err() {
                                debug!("Stream receiver dropped, ending early");
                                break;
                            }
                        }
                        Ok(StreamChunk::Reasoning(r)) => {
                            if tx.send(StreamChunk::Reasoning(r)).await.is_err() {
                                debug!("Stream receiver dropped, ending early");
                                break;
                            }
                        }
                        Ok(done @ StreamChunk::Done { .. }) => {
                            let _ = tx.send(done).await;
                            break;
                        }
                        Err(e) => {
                            if e.to_string().contains("aborted") {
                                break;
                            }
                            return Err(e);
                        }
                    }
                }

                Ok(StreamResult { tool_calls, text })
            }),
        };

        (rx, handle)
    }
}

// ============================================================================
// Stream Handle and Result
// ============================================================================

/// Result of a streaming completion
pub struct StreamResult {
    /// All tool calls from the response
    pub tool_calls: Vec<ToolCall>,
    /// All text concatenated
    pub text: String,
}

/// Handle to await the completion of a streaming request
pub struct StreamHandle {
    inner: tokio::task::JoinHandle<Result<StreamResult, CompletionError>>,
}

impl StreamHandle {
    /// Wait for the stream to complete and get the final result
    pub async fn await_result(self) -> Result<StreamResult, CompletionError> {
        self.inner
            .await
            .map_err(|e| CompletionError::ProviderError(format!("Stream task failed: {}", e)))?
    }
}
