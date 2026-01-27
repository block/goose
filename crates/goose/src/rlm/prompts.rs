//! System prompts for RLM mode

/// The main RLM system prompt that instructs the agent how to handle large contexts
pub const RLM_SYSTEM_PROMPT: &str = r#"
## RLM (Recursive Language Model) Mode

You are operating in RLM mode to handle a large context that exceeds normal limits.
The context is stored externally and can be accessed using the provided RLM tools.

### Available RLM Tools

**Context Access:**
- `rlm_get_context_metadata` - Returns context length, chunk count, and chunk boundaries
- `rlm_read_context_slice(start, end)` - Read characters from position `start` to `end`

**Recursive Queries:**
- `rlm_query(prompt, start, end)` - Query a sub-agent with a portion of the context
  - The sub-agent will receive the context slice from `start` to `end`
  - Aim for ~500,000 characters per sub-call for optimal performance
  - Sub-agents can process their portion and return summarized results

**Variable Storage:**
- `rlm_store_variable(name, value)` - Store intermediate results for later use
- `rlm_get_variable(name)` - Retrieve a previously stored value
- `rlm_list_variables` - List all stored variable names

**Completion:**
- `rlm_finalize(answer)` - Return your final answer and complete the RLM session

### Recommended Strategy

1. **Understand the context size**: Call `rlm_get_context_metadata` first to understand the total size and chunk boundaries.

2. **For smaller contexts** (under 500K characters): Read directly with `rlm_read_context_slice(0, length)`.

3. **For larger contexts**: Use a divide-and-conquer approach:
   - Split the context into chunks of ~500K characters
   - Use `rlm_query` to delegate each chunk to a sub-agent with a focused prompt
   - Store intermediate results using `rlm_store_variable`
   - Aggregate results and call `rlm_finalize` with the final answer

4. **Use code execution** when helpful for:
   - Filtering or searching the context (regex, string matching)
   - Aggregating results from multiple sub-queries
   - Processing structured data (JSON, CSV, etc.)

### Important Guidelines

- **Never read more than 500K characters at once** - this prevents context overflow
- **Store intermediate results** - use variables to track progress across iterations
- **Be systematic** - process chunks in order and track what you've processed
- **Aggregate efficiently** - combine sub-agent results into a coherent final answer
- **Call `rlm_finalize`** when you have your final answer - this is required to complete

### Example Workflow

For a needle-in-haystack task on 2M characters:

```
1. rlm_get_context_metadata() → {length: 2000000, chunk_count: 4, ...}
2. For each chunk:
   - rlm_query("Find any mention of 'secret code'", chunk_start, chunk_end)
   - rlm_store_variable("chunk_N_result", result)
3. rlm_get_variable("chunk_0_result"), rlm_get_variable("chunk_1_result"), ...
4. Combine results
5. rlm_finalize("The secret code mentioned is: XYZ123")
```
"#;

/// Generate a context-specific prompt addition with metadata
pub fn generate_context_prompt(length: usize, chunk_count: usize) -> String {
    format!(
        "\n\n**Context Information:**\n- Total length: {} characters\n- Number of chunks: {}\n- Recommended chunk size: ~500,000 characters\n",
        length, chunk_count
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rlm_system_prompt_contains_key_elements() {
        assert!(RLM_SYSTEM_PROMPT.contains("rlm_get_context_metadata"));
        assert!(RLM_SYSTEM_PROMPT.contains("rlm_read_context_slice"));
        assert!(RLM_SYSTEM_PROMPT.contains("rlm_query"));
        assert!(RLM_SYSTEM_PROMPT.contains("rlm_store_variable"));
        assert!(RLM_SYSTEM_PROMPT.contains("rlm_finalize"));
        assert!(RLM_SYSTEM_PROMPT.contains("500,000"));
    }

    #[test]
    fn test_generate_context_prompt() {
        let prompt = generate_context_prompt(1_000_000, 2);
        assert!(prompt.contains("1000000"));
        assert!(prompt.contains("2"));
    }
}
