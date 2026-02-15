//! Streaming markdown buffer for safe incremental rendering.
//!
//! This module provides a buffer that accumulates streaming markdown chunks
//! and determines safe points to flush content for rendering. It tracks
//! open markdown constructs (code blocks, bold, links, etc.) to ensure
//! we only output complete, well-formed markdown.
//!
//! # Example
//!
//! ```
//! use goose_cli::session::streaming_buffer::MarkdownBuffer;
//!
//! let mut buf = MarkdownBuffer::new();
//!
//! // Partial bold - buffers until closed
//! assert_eq!(buf.push("Hello **wor"), Some("Hello ".to_string()));
//! assert_eq!(buf.push("ld**!"), Some("**world**!".to_string()));
//!
//! // At end of stream, flush remaining content
//! let remaining = buf.flush();
//! ```

use regex::Regex;
use std::sync::LazyLock;

/// Regex that tokenizes markdown inline elements.
/// Order matters: longer/more-specific patterns first.
static INLINE_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(",
        r"\\.",                 // Escaped char (highest priority)
        r"|`+",                 // Inline code (variable length backticks)
        r"|\*\*\*",             // Bold+italic
        r"|\*\*",               // Bold
        r"|\*",                 // Italic
        r"|___",                // Bold+italic (underscore)
        r"|__",                 // Bold (underscore)
        r"|_",                  // Italic (underscore)
        r"|~~",                 // Strikethrough
        r"|\!\[",               // Image start
        r"|\]\(",               // Link URL start
        r"|\[",                 // Link text start
        r"|\]",                 // Bracket close (without following paren)
        r"|\)",                 // Link URL end
        r"|[^\\\*_`~\[\]!()]+", // Plain text (no special chars)
        r"|.",                  // Any other single char
        r")"
    ))
    .unwrap()
});

/// A streaming markdown buffer that tracks open constructs.
///
/// Accumulates chunks and returns content that is safe to render,
/// holding back any incomplete markdown constructs.
#[derive(Default)]
pub struct MarkdownBuffer {
    buffer: String,
}

/// Tracks the current parsing state for markdown constructs.
#[derive(Default, Debug, Clone, PartialEq)]
struct ParseState {
    // Block-level state (line-aware)
    in_code_block: bool,
    code_fence_char: char,
    code_fence_len: usize,
    in_table: bool,
    pending_heading: bool,

    // Inline state (regex tokenizer)
    in_inline_code: bool,
    inline_code_len: usize,
    in_bold: bool,
    in_italic: bool,
    in_strikethrough: bool,
    in_link_text: bool,
    in_link_url: bool,
    in_image_alt: bool,
}

impl ParseState {
    /// Returns true if no markdown constructs are currently open.
    fn is_clean(&self) -> bool {
        !self.in_code_block
            && !self.in_table
            && !self.pending_heading
            && !self.in_inline_code
            && !self.in_bold
            && !self.in_italic
            && !self.in_strikethrough
            && !self.in_link_text
            && !self.in_link_url
            && !self.in_image_alt
    }
}

// SAFETY: All string slicing in this impl is safe because:
// - We only slice at positions derived from ASCII characters (newlines, #, |, etc.)
// - The regex tokenizer operates on valid UTF-8 and returns byte positions at char boundaries
// - Code fence detection uses chars().take_while() which respects UTF-8
#[allow(clippy::string_slice)]
impl MarkdownBuffer {
    /// Create a new empty buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a chunk of markdown text to the buffer.
    ///
    /// Returns any content that is safe to render, or None if the buffer
    /// contains only incomplete constructs.
    pub fn push(&mut self, chunk: &str) -> Option<String> {
        self.buffer.push_str(chunk);
        let safe_end = self.find_safe_end();

        if safe_end > 0 {
            // SAFETY: safe_end is always at a valid UTF-8 char boundary because:
            // - We only set it after processing complete regex tokens (which match
            //   valid UTF-8 sequences) or at newline positions (ASCII, single byte)
            // - The regex tokenizer operates on &str which guarantees UTF-8
            let to_render = self.buffer[..safe_end].to_string();
            self.buffer = self.buffer[safe_end..].to_string();
            Some(to_render)
        } else {
            None
        }
    }

    /// Flush any remaining content from the buffer.
    ///
    /// Call this at the end of a stream to get any buffered content,
    /// even if markdown constructs are unclosed.
    pub fn flush(&mut self) -> String {
        std::mem::take(&mut self.buffer)
    }

    /// Find the last byte position where the parse state is "clean".
    fn find_safe_end(&self) -> usize {
        let mut state = ParseState::default();
        let mut last_safe: usize = 0;
        let bytes = self.buffer.as_bytes();
        let len = bytes.len();
        let mut pos: usize = 0;

        while pos < len {
            // Check for block-level constructs at line starts
            let at_line_start = pos == 0 || bytes[pos - 1] == b'\n';

            if at_line_start {
                // Process block-level constructs
                if let Some(new_pos) = self.process_line_start(&mut state, pos) {
                    pos = new_pos;

                    // After processing line start, check if state is clean
                    if state.is_clean() {
                        last_safe = pos;
                    }
                    continue;
                }
            }

            // Inside a code block, just scan for newlines
            if state.in_code_block {
                // Find next newline or end of buffer
                while pos < len && bytes[pos] != b'\n' {
                    pos += 1;
                }
                if pos < len {
                    pos += 1; // Skip the newline
                }
                continue;
            }

            // Process inline constructs using the regex tokenizer
            let remaining = &self.buffer[pos..];

            // Find next newline to bound our inline processing
            let line_end = remaining.find('\n').map(|i| pos + i + 1).unwrap_or(len);

            // Tokenize up to the newline
            let line_content = &self.buffer[pos..line_end];

            for cap in INLINE_TOKEN_RE.find_iter(line_content) {
                let token = cap.as_str();
                let token_end = pos + cap.end();

                self.process_inline_token(&mut state, token);

                if state.is_clean() {
                    last_safe = token_end;
                }
            }

            // If we processed a complete line (ending with newline), clear pending_heading
            if line_end <= len && line_end > pos && bytes[line_end - 1] == b'\n' {
                state.pending_heading = false;
                // Update last_safe if state is now clean after clearing heading
                if state.is_clean() {
                    last_safe = line_end;
                }
            }

            pos = line_end;
        }

        last_safe
    }

    /// Process block-level constructs at the start of a line.
    ///
    /// Returns the new position after processing, or None if no block construct found.
    fn process_line_start(&self, state: &mut ParseState, pos: usize) -> Option<usize> {
        let remaining = &self.buffer[pos..];

        // If we were pending a heading, the newline completes it
        if state.pending_heading {
            state.pending_heading = false;
        }

        // Check for code fence (``` or ~~~)
        if let Some(fence_result) = self.check_code_fence(remaining, state) {
            return Some(pos + fence_result);
        }

        // If in code block, don't process other block constructs
        if state.in_code_block {
            return None;
        }

        // Check for heading (# at start of line)
        if remaining.starts_with('#') {
            // Count the # characters
            let hashes = remaining.chars().take_while(|&c| c == '#').count();
            // Valid heading: 1-6 hashes followed by space or newline
            if hashes <= 6 {
                let after_hashes = &remaining[hashes..];
                if after_hashes.is_empty()
                    || after_hashes.starts_with(' ')
                    || after_hashes.starts_with('\n')
                {
                    state.pending_heading = true;
                    // Don't advance pos, let inline processing handle the content
                    return None;
                }
            }
        }

        // Check for table row (| at start of line)
        if remaining.starts_with('|') {
            state.in_table = true;
            return None; // Let inline processing handle the row
        }

        // Check for blank line (closes table)
        if (remaining.starts_with('\n') || remaining.is_empty()) && state.in_table {
            state.in_table = false;
            return Some(pos + 1);
        }

        // If in table but line doesn't start with |, close table
        if state.in_table && !remaining.starts_with('|') {
            state.in_table = false;
        }

        None
    }

    /// Check for a code fence and update state accordingly.
    ///
    /// Returns the position after the fence line if found, None otherwise.
    fn check_code_fence(&self, line: &str, state: &mut ParseState) -> Option<usize> {
        let trimmed = line.trim_start();

        // Determine fence character and count
        let fence_char = trimmed.chars().next()?;
        if fence_char != '`' && fence_char != '~' {
            return None;
        }

        let fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
        if fence_len < 3 {
            return None;
        }

        let after_fence = &trimmed[fence_len..];

        if state.in_code_block {
            // Check if this is a valid closing fence
            if fence_char == state.code_fence_char
                && fence_len >= state.code_fence_len
                && (after_fence.is_empty()
                    || after_fence.starts_with('\n')
                    || after_fence.trim().is_empty())
            {
                // Valid closing fence
                state.in_code_block = false;
                state.code_fence_char = '\0';
                state.code_fence_len = 0;

                // Return position after the fence line
                if let Some(newline_pos) = line.find('\n') {
                    return Some(newline_pos + 1);
                } else {
                    return Some(line.len());
                }
            }
        } else {
            // Opening fence - can have info string after it
            state.in_code_block = true;
            state.code_fence_char = fence_char;
            state.code_fence_len = fence_len;

            // Don't return safe position - we're now in a code block
            if let Some(newline_pos) = line.find('\n') {
                return Some(newline_pos + 1);
            } else {
                return Some(line.len());
            }
        }

        None
    }

    /// Process an inline token and update state.
    fn process_inline_token(&self, state: &mut ParseState, token: &str) {
        // Escaped characters don't affect state
        if token.starts_with('\\') && token.len() == 2 {
            return;
        }

        // Inline code (backticks)
        if token.starts_with('`') {
            let tick_count = token.len();
            if state.in_inline_code {
                if tick_count == state.inline_code_len {
                    state.in_inline_code = false;
                    state.inline_code_len = 0;
                }
            } else {
                state.in_inline_code = true;
                state.inline_code_len = tick_count;
            }
            return;
        }

        // Inside inline code, nothing else matters
        if state.in_inline_code {
            return;
        }

        // Bold/italic markers
        match token {
            "***" | "___" => {
                // Toggle both bold and italic
                if state.in_bold && state.in_italic {
                    state.in_bold = false;
                    state.in_italic = false;
                } else if state.in_bold {
                    state.in_italic = !state.in_italic;
                } else if state.in_italic {
                    state.in_bold = !state.in_bold;
                } else {
                    state.in_bold = true;
                    state.in_italic = true;
                }
            }
            "**" | "__" => {
                state.in_bold = !state.in_bold;
            }
            "*" | "_" => {
                state.in_italic = !state.in_italic;
            }
            "~~" => {
                state.in_strikethrough = !state.in_strikethrough;
            }
            "![" => {
                state.in_image_alt = true;
            }
            "[" => {
                if !state.in_link_text && !state.in_image_alt {
                    state.in_link_text = true;
                }
            }
            "](" => {
                if state.in_link_text {
                    state.in_link_text = false;
                    state.in_link_url = true;
                } else if state.in_image_alt {
                    state.in_image_alt = false;
                    state.in_link_url = true;
                }
            }
            "]" => {
                // Unmatched ] - could be part of incomplete link
                // Don't close link_text here, wait for ]( or next chunk
            }
            ")" => {
                if state.in_link_url {
                    state.in_link_url = false;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    /// Process chunks through the buffer and return all outputs (skipping None, including flush)
    fn stream(chunks: &[&str]) -> Vec<String> {
        let mut buf = MarkdownBuffer::new();
        let mut results: Vec<String> = chunks.iter().filter_map(|chunk| buf.push(chunk)).collect();
        let remaining = buf.flush();
        if !remaining.is_empty() {
            results.push(remaining);
        }
        results
    }

    // ===========================================
    // Realistic LLM streaming scenarios
    // ===========================================

    #[test_case(
        &["I'll", " help", " you", " with", " that", "!"],
        &["I'll", " help", " you", " with", " that", "!"]
        ; "simple sentence streams through immediately without markdown"
    )]
    #[test_case(
        &["Here's the **important", "** part."],
        &["Here's the ", "**important** part."]
        ; "bold split mid-word"
    )]
    #[test_case(
        &["Use the `println!", "` macro."],
        &["Use the ", "`println!` macro."]
        ; "inline code split"
    )]
    #[test_case(
        &["Check [the docs](https://doc", "s.rs) for more."],
        &["Check ", "[the docs](https://docs.rs) for more."]
        ; "link url split"
    )]
    fn test_inline_streaming(chunks: &[&str], expected: &[&str]) {
        assert_eq!(stream(chunks), expected);
    }

    // ===========================================
    // Code blocks (most important for bat rendering)
    // ===========================================

    #[test_case(
        &["```rust\n", "fn main() {\n", "    println!(\"hello\");\n", "}\n", "```\n"],
        &["```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n"]
        ; "rust code block streamed line by line"
    )]
    #[test_case(
        &["Here's an exa", "mple:\n\n```python\nprint(\"``", "`nested```\")\n```\n\nNice!"],
        &["Here's an exa", "mple:\n", "\n```python\nprint(\"```nested```\")\n```\n\nNice!"]
        ; "code block with backticks in string literal"
    )]
    #[test_case(
        &["````md\n", "```\ninner\n```\n", "````\n"],
        &["````md\n```\ninner\n```\n````\n"]
        ; "nested code fence with longer outer fence"
    )]
    #[test_case(
        &["~~~bash\n", "echo 'hello'\n", "~", "~~\n"],
        &["~~~bash\necho 'hello'\n~~~\n"]
        ; "tilde code fence"
    )]
    #[test_case(
        &["```\ncode"],
        &["```\ncode"]
        ; "unclosed code block flushes at end"
    )]
    fn test_code_blocks(chunks: &[&str], expected: &[&str]) {
        assert_eq!(stream(chunks), expected);
    }

    // ===========================================
    // Headings
    // ===========================================

    #[test_case(
        &["# Getting St", "arted\n\nFirst, install..."],
        &["# Getting Started\n\nFirst, install..."]
        ; "heading split mid-word"
    )]
    #[test_case(
        &["## API Reference\n\n###", " Methods\n\n"],
        &["## API Reference\n\n", "### Methods\n\n"]
        ; "multiple headings in one chunk"
    )]
    fn test_headings(chunks: &[&str], expected: &[&str]) {
        assert_eq!(stream(chunks), expected);
    }

    // ===========================================
    // Tables
    // ===========================================

    #[test_case(
        &["| Name | Value |\n", "|------|-------|\n", "| foo  | 42    |\n", "\nMore text"],
        &["| Name | Value |\n|------|-------|\n| foo  | 42    |\n\nMore text"]
        ; "table streamed row by row"
    )]
    #[test_case(
        &["| A | B |\n|---|---|\n| 1 | 2 |\n\n"],
        &["| A | B |\n|---|---|\n| 1 | 2 |\n\n"]
        ; "table followed by blank line"
    )]
    fn test_tables(chunks: &[&str], expected: &[&str]) {
        assert_eq!(stream(chunks), expected);
    }

    // ===========================================
    // Mixed formatting (realistic assistant responses)
    // ===========================================

    #[test_case(
        &[
            "Here's how to do it:\n\n",
            "1. First, run `cargo", " build`\n",
            "2. Then check the **out", "put**\n\n",
            "```rust\n",
            "fn main() {}\n",
            "```\n"
        ],
        &[
            "Here's how to do it:\n\n",
            "1. First, run ",
            "`cargo build`\n",
            "2. Then check the ",
            "**output**\n\n",
            "```rust\nfn main() {}\n```\n"
        ]
        ; "typical assistant response with list code and formatting"
    )]
    #[test_case(
        &[
            "See the [**Rust Book**](https://doc.rust-l",
            "ang.org/book/) for more info.\n\n",
            "Key points:\n- Use `Result` for errors\n- Prefer `Option` over null"
        ],
        &[
            "See the ",
            "[**Rust Book**](https://doc.rust-lang.org/book/) for more info.\n\n",
            "Key points:\n- Use `Result` for errors\n- Prefer `Option` over null"
        ]
        ; "link with nested bold and list"
    )]
    #[test_case(
        &[
            "![screenshot](./img/sc",
            "reen.png)\n\nAs shown above..."
        ],
        &[
            "![screenshot](./img/screen.png)\n\nAs shown above..."
        ]
        ; "image with split url"
    )]
    fn test_mixed_content(chunks: &[&str], expected: &[&str]) {
        assert_eq!(stream(chunks), expected);
    }

    // ===========================================
    // Edge cases and escapes
    // ===========================================

    #[test_case(
        &["Use \\* for bullet points, not \\`code\\`"],
        &["Use \\* for bullet points, not \\`code\\`"]
        ; "escaped markdown characters"
    )]
    #[test_case(
        &["Price: $100 * 2 = $200"],
        &["Price: $100 ", "* 2 = $200"]
        ; "asterisk in math context treated as italic marker"
    )]
    #[test_case(
        &[""],
        &[] as &[&str]
        ; "empty input produces no output"
    )]
    #[test_case(
        &["Hello 世界! Here's some **太字** text."],
        &["Hello 世界! Here's some **太字** text."]
        ; "unicode content"
    )]
    #[test_case(
        &["**bold *and italic* together**"],
        &["**bold *and italic* together**"]
        ; "nested bold and italic"
    )]
    #[test_case(
        &["***bold italic***"],
        &["***bold italic***"]
        ; "combined bold italic marker"
    )]
    #[test_case(
        &["~~stri", "ke~~ and **bo", "ld**"],
        &["~~strike~~ and ", "**bold**"]
        ; "strikethrough and bold split"
    )]
    fn test_edge_cases(chunks: &[&str], expected: &[&str]) {
        assert_eq!(stream(chunks), expected);
    }

    // ===========================================
    // Incomplete constructs at stream end
    // ===========================================

    #[test_case(
        &["This is **incomplete bold"],
        &["This is ", "**incomplete bold"]
        ; "unclosed bold flushes"
    )]
    #[test_case(
        &["Check [broken link](http://"],
        &["Check ", "[broken link](http://"]
        ; "unclosed link flushes"
    )]
    #[test_case(
        &["Start of `code"],
        &["Start of ", "`code"]
        ; "unclosed inline code flushes"
    )]
    fn test_incomplete_constructs(chunks: &[&str], expected: &[&str]) {
        assert_eq!(stream(chunks), expected);
    }
}
