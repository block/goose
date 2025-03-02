use crate::agents::Capabilities;
use crate::message::Message;
use crate::token_counter::TokenCounter;
use anyhow::Result;
use async_trait::async_trait;
use tracing::debug;

/// A more general abstraction allowing for custom compression strategy (specifically for memory condensation), instead of truncation-based ones.
#[async_trait]
pub trait Compressor {
    async fn compress(
        &self,
        capabilities: &Capabilities,
        token_counter: &TokenCounter,
        messages: &mut Vec<Message>,
        token_counts: &mut Vec<usize>,
        context_limit: usize,
    ) -> Result<(), anyhow::Error>;
}

pub async fn compress_messages(
    capabilities: &Capabilities,
    token_counter: &TokenCounter,
    messages: &mut Vec<Message>,
    token_counts: &mut Vec<usize>,
    context_limit: usize,
    compressor: &(dyn Compressor + Send + Sync),
) -> Result<(), anyhow::Error> {
    let total_tokens: usize = token_counts.iter().sum();
    debug!("Total tokens before compression: {}", total_tokens);

    // The compressor should determine whether we need to compress the messages or not. This
    // function just checks if the limit is satisfied.
    compressor
        .compress(
            capabilities,
            token_counter,
            messages,
            token_counts,
            context_limit,
        )
        .await?;

    let total_tokens: usize = token_counts.iter().sum();
    debug!("Total tokens after compression: {}", total_tokens);

    // Compressor should handle this case.
    assert!(total_tokens <= context_limit, "Illegal compression result from the compressor: the number of tokens is greater than the limit.");

    debug!("Compression complete. Total tokens: {}", total_tokens);
    Ok(())
}
