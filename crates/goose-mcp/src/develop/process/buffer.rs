//! Ring buffer for process output storage.

use crate::develop::process::types::OutputQuery;

/// Maximum bytes to retain in the output buffer.
const MAX_BUFFER_BYTES: usize = 1024 * 1024; // 1MB

/// Maximum bytes per line before truncation.
const MAX_LINE_BYTES: usize = 1024; // 1KB

/// Ring buffer that stores output lines, keeping the tail when full.
#[derive(Debug)]
pub struct OutputBuffer {
    lines: Vec<String>,
    total_bytes: usize,
}

impl Default for OutputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputBuffer {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            total_bytes: 0,
        }
    }

    /// Append raw output, splitting into lines and sanitizing.
    pub fn append(&mut self, data: &str) {
        for line in data.lines() {
            self.push_line(line);
        }
        // Handle trailing content without newline
        if !data.ends_with('\n') && !data.is_empty() {
            // The last "line" from lines() already captured it
        }
    }

    /// Push a single line, sanitizing and enforcing limits.
    fn push_line(&mut self, line: &str) {
        let sanitized = Self::sanitize_line(line);
        let line_bytes = sanitized.len();

        // Add the line
        self.lines.push(sanitized);
        self.total_bytes += line_bytes;

        // Evict old lines if over budget
        while self.total_bytes > MAX_BUFFER_BYTES && self.lines.len() > 1 {
            if let Some(removed) = self.lines.first() {
                self.total_bytes = self.total_bytes.saturating_sub(removed.len());
            }
            self.lines.remove(0);
        }
    }

    /// Sanitize a line: truncate if too long.
    fn sanitize_line(line: &str) -> String {
        // Line is already valid UTF-8 since it's a &str
        // Just truncate if too long
        if line.len() <= MAX_LINE_BYTES {
            line.to_string()
        } else {
            // Collect chars up to the byte limit
            let mut truncated = String::new();
            for c in line.chars() {
                if truncated.len() + c.len_utf8() > MAX_LINE_BYTES {
                    break;
                }
                truncated.push(c);
            }
            truncated.push_str("... [truncated]");
            truncated
        }
    }

    /// Get total line count.
    #[allow(dead_code)]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Query the buffer with optional slicing and grep.
    pub fn query(&self, q: &OutputQuery) -> String {
        let lines = &self.lines;
        let len = lines.len() as i64;

        // Apply grep filter first if present
        let filtered: Vec<(usize, &String)> = if let Some(ref pattern) = q.grep {
            lines
                .iter()
                .enumerate()
                .filter(|(_, line)| line.contains(pattern.as_str()))
                .collect()
        } else {
            lines.iter().enumerate().collect()
        };

        // If grep with context, expand the selection
        let result_lines: Vec<&String> =
            if q.grep.is_some() && (q.before.is_some() || q.after.is_some()) {
                let before = q.before.unwrap_or(0);
                let after = q.after.unwrap_or(0);

                // Collect all line indices that should be included
                let mut include: Vec<bool> = vec![false; lines.len()];
                for (idx, _) in &filtered {
                    let start = idx.saturating_sub(before);
                    let end = (*idx + after + 1).min(lines.len());
                    for item in include.iter_mut().take(end).skip(start) {
                        *item = true;
                    }
                }

                lines
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| include[*i])
                    .map(|(_, line)| line)
                    .collect()
            } else if q.grep.is_some() {
                // Grep without context - just matching lines
                filtered.iter().map(|(_, line)| *line).collect()
            } else {
                // No grep - apply slice to all lines
                let start = normalize_index(q.start.unwrap_or(0), len);
                let end = normalize_index(q.end.unwrap_or(len), len);

                if start >= end {
                    return String::new();
                }

                lines[start..end].iter().collect()
            };

        result_lines
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get all lines as a single string.
    pub fn full_output(&self) -> String {
        self.lines.join("\n")
    }

    /// Get a truncated preview: first N lines + last M lines with omission marker.
    pub fn preview(&self, head: usize, tail: usize) -> (String, usize) {
        let total = self.lines.len();

        if total <= head + tail {
            return (self.full_output(), 0);
        }

        let head_lines: Vec<&str> = self.lines[..head].iter().map(|s| s.as_str()).collect();
        let tail_lines: Vec<&str> = self.lines[total - tail..]
            .iter()
            .map(|s| s.as_str())
            .collect();
        let omitted = total - head - tail;

        let preview = format!(
            "{}\n... ({} lines omitted, use process_output to explore) ...\n{}",
            head_lines.join("\n"),
            omitted,
            tail_lines.join("\n")
        );

        (preview, omitted)
    }
}

/// Normalize a Python-style index to a valid array index.
fn normalize_index(idx: i64, len: i64) -> usize {
    if idx < 0 {
        (len + idx).max(0) as usize
    } else {
        (idx as usize).min(len as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_and_query() {
        let mut buf = OutputBuffer::new();
        buf.append("line1\nline2\nline3\n");

        assert_eq!(buf.line_count(), 3);
        assert_eq!(buf.full_output(), "line1\nline2\nline3");
    }

    #[test]
    fn test_slice_semantics() {
        let mut buf = OutputBuffer::new();
        buf.append("a\nb\nc\nd\ne\n");

        // [:2] - first 2 lines
        let q = OutputQuery {
            end: Some(2),
            ..Default::default()
        };
        assert_eq!(buf.query(&q), "a\nb");

        // [-2:] - last 2 lines
        let q = OutputQuery {
            start: Some(-2),
            ..Default::default()
        };
        assert_eq!(buf.query(&q), "d\ne");

        // [1:3] - middle slice
        let q = OutputQuery {
            start: Some(1),
            end: Some(3),
            ..Default::default()
        };
        assert_eq!(buf.query(&q), "b\nc");
    }

    #[test]
    fn test_grep() {
        let mut buf = OutputBuffer::new();
        buf.append("error: something\ninfo: ok\nerror: another\ninfo: done\n");

        let q = OutputQuery {
            grep: Some("error".to_string()),
            ..Default::default()
        };
        assert_eq!(buf.query(&q), "error: something\nerror: another");
    }

    #[test]
    fn test_grep_with_context() {
        let mut buf = OutputBuffer::new();
        buf.append("1\n2\n3\nMATCH\n5\n6\n7\n");

        let q = OutputQuery {
            grep: Some("MATCH".to_string()),
            before: Some(1),
            after: Some(2),
            ..Default::default()
        };
        assert_eq!(buf.query(&q), "3\nMATCH\n5\n6");
    }

    #[test]
    fn test_preview() {
        let mut buf = OutputBuffer::new();
        for i in 1..=100 {
            buf.append(&format!("line{}\n", i));
        }

        let (preview, omitted) = buf.preview(3, 3);
        assert!(preview.contains("line1"));
        assert!(preview.contains("line2"));
        assert!(preview.contains("line3"));
        assert!(preview.contains("line98"));
        assert!(preview.contains("line99"));
        assert!(preview.contains("line100"));
        assert!(preview.contains("94 lines omitted"));
        assert_eq!(omitted, 94);
    }

    #[test]
    fn test_line_truncation() {
        let mut buf = OutputBuffer::new();
        let long_line = "x".repeat(2000);
        buf.append(&long_line);

        let output = buf.full_output();
        assert!(output.len() < 2000);
        assert!(output.ends_with("... [truncated]"));
    }
}
