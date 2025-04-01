use crate::token_counter::TokenCounter;
use mcp_core::{Content, tool::ToolCall};
use tracing::info;

/// Limits tool outputs to prevent context overflow errors
pub struct ToolOutputLimiter {
    token_counter: TokenCounter,
    context_ratio: f32, // Percentage of context window to allow for tool outputs
}

impl ToolOutputLimiter {
    pub fn new(token_counter: TokenCounter) -> Self {
        Self {
            token_counter,
            // Default to 70% of the model's context window
            // Note that we might still exceed it, since our token counting is approximate
            context_ratio: 0.7,
        }
    }
    
    /// Limit tool output to a percentage of the model's context window
    pub fn limit_tool_output(
        &self, 
        tool_call: &ToolCall, 
        content: Vec<Content>, 
        model_context_limit: usize
    ) -> Vec<Content> {
        // Calculate maximum allowed tokens based on configured ratio
        let max_tokens = (model_context_limit as f32 * self.context_ratio) as usize;
        
        // Count tokens in the content
        let token_count = self.count_tokens_for_content(&content);
        
        // If content is within limit, return as is
        if token_count <= max_tokens {
            return content;
        }
        
        // Log that we're truncating
        info!(
            "Truncating large tool output: tool={}, tokens={}, limit={} ({}% of context window)",
            tool_call.name, token_count, max_tokens, (self.context_ratio * 100.0) as usize
        );
        
        // Apply truncation
        self.truncate_content(content, token_count, max_tokens)
    }
    
    /// Count tokens in a vector of Content
    fn count_tokens_for_content(&self, content: &[Content]) -> usize {
        content.iter()
            .map(|c| match c {
                Content::Text(text_content) => self.token_counter.count_tokens(&text_content.text),
                _ => 0, // For simplicity, only count text content
            })
            .sum()
    }
    
    /// Truncate content to fit within token limit
    fn truncate_content(
        &self,
        content: Vec<Content>,
        current_tokens: usize,
        max_tokens: usize
    ) -> Vec<Content> {
        // For simplicity, we'll focus on text content
        if let Some(Content::Text(text_content)) = content.first() {
            let text = &text_content.text;
            
            // We need to truncate. Since we can't easily truncate by tokens directly,
            // we'll use a binary search approach to find the right truncation point.
            let mut start = 0;
            let mut end = text.len();
            let mut truncation_point = 0;
            
            // Binary search to find the maximum text length that fits within max_tokens
            while start < end {
                let mid = start + (end - start) / 2;
                let truncated_text = &text[0..mid];
                let token_count = self.token_counter.count_tokens(truncated_text);
                
                if token_count <= max_tokens {
                    // This fits, try a longer text
                    truncation_point = mid;
                    start = mid + 1;
                } else {
                    // Too many tokens, try a shorter text
                    end = mid;
                }
            }
            
            // Create truncated text with notice
            let truncated_text = format!(
                "{}\n\n[Output truncated: Tool returned {} tokens which exceeds the {} token limit ({}% of model context window)]",
                &text[..truncation_point],
                current_tokens,
                max_tokens,
                (self.context_ratio * 100.0) as usize
            );
            
            return vec![Content::text(truncated_text)];
        }
        
        // For non-text content, return a generic message
        vec![Content::text(format!(
            "Tool output was truncated: exceeded {} tokens ({}% of model context window)",
            max_tokens,
            (self.context_ratio * 100.0) as usize
        ))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::GPT_4O_TOKENIZER;
    use serde_json::json;
    
    fn create_test_limiter() -> ToolOutputLimiter {
        let token_counter = TokenCounter::new(GPT_4O_TOKENIZER);
        ToolOutputLimiter::new(token_counter)
    }
    
    #[test]
    fn test_no_truncation_needed() {
        let limiter = create_test_limiter();
        let tool_call = ToolCall::new("test_tool", json!({}));
        let content = vec![Content::text("Small output")];
        
        // With a large context window
        let result = limiter.limit_tool_output(&tool_call, content.clone(), 1000);
        
        // Should be unchanged
        assert_eq!(result, content);
    }
    
    #[test]
    fn test_truncation_applied() {
        let limiter = create_test_limiter();
        let tool_call = ToolCall::new("test_tool", json!({}));
        
        // Create a long text that will need truncation
        let long_text = "This is a very long output. ".repeat(100);
        let long_text_len = long_text.len();
        let content = vec![Content::text(long_text)];
        
        // With a small context window
        let result = limiter.limit_tool_output(&tool_call, content, 50);
        
        // Should be truncated with notice
        let truncated = result[0].as_text().unwrap();
        assert!(truncated.contains("Output truncated"));
        assert!(truncated.len() < long_text_len);
    }
}