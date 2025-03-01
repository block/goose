use crate::message::Message;
use anyhow::Result;
use tracing::debug;

/// A more general abstraction allowing for custom compression strategy (specifically for memory condensation), instead of truncation-based ones.
pub trait Compressor {
    fn compress(
        &self,
        messages: &mut Vec<Message>,
        token_counts: &mut Vec<usize>,
        context_limit: usize,
    ) -> Result<(), anyhow::Error>;
}

pub fn compress_messages(
    messages: &mut Vec<Message>,
    token_counts: &mut Vec<usize>,
    context_limit: usize,
    compressor: &dyn Compressor,
) -> Result<(), anyhow::Error> {
    let total_tokens: usize = token_counts.iter().sum();
    debug!("Total tokens before compression: {}", total_tokens);

    // The compressor should determine whether we need to compress the messages or not. This
    // function just checks if the limit is satisfied.
    compressor.compress(messages, token_counts, context_limit)?;

    let total_tokens: usize = token_counts.iter().sum();
    debug!("Total tokens after compression: {}", total_tokens);

    // Compressor should handle this case.
    assert!(total_tokens <= context_limit, "Illegal compression result from the compressor: the number of tokens is greater than the limit.");

    debug!("Compression complete. Total tokens: {}", total_tokens);
    Ok(())
}
