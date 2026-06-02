/// Remove repeated phrases from transcribed text.
///
/// Whisper models (especially smaller/quantized ones) tend to loop, producing output like
/// "I could build a record mode. I could build a record mode. I could build a record mode."
/// This function collapses adjacent duplicate sentences/phrases down to a single occurrence.
pub fn deduplicate_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Split into sentences on common boundaries (. ! ?)
    let sentences = split_into_sentences(trimmed);
    if sentences.len() <= 1 {
        return trimmed.to_string();
    }

    let mut result: Vec<&str> = Vec::new();

    let mut i = 0;
    while i < sentences.len() {
        // Try to find a repeating pattern starting at position i.
        // Check pattern lengths from 1 sentence up to half the remaining sentences.
        let remaining = sentences.len() - i;
        let max_pattern_len = remaining / 2;
        let mut best_pattern_len = 0;
        let mut best_repeat_count = 0;
        let mut best_total_consumed = 0;

        for pattern_len in 1..=max_pattern_len {
            let pattern = &sentences[i..i + pattern_len];
            let mut count = 1;
            let mut pos = i + pattern_len;
            while pos + pattern_len <= sentences.len() {
                let candidate = &sentences[pos..pos + pattern_len];
                if pattern
                    .iter()
                    .zip(candidate.iter())
                    .all(|(a, b)| a.trim() == b.trim())
                {
                    count += 1;
                    pos += pattern_len;
                } else {
                    break;
                }
            }
            // Prefer the pattern that removes the most repeated sentences
            let total_consumed = pattern_len * count;
            if count >= 2 && total_consumed > best_total_consumed {
                best_pattern_len = pattern_len;
                best_repeat_count = count;
                best_total_consumed = total_consumed;
            }
        }

        if best_repeat_count >= 2 {
            // Keep only the first occurrence of the repeated pattern
            for j in 0..best_pattern_len {
                result.push(sentences[i + j]);
            }
            i += best_pattern_len * best_repeat_count;
        } else {
            result.push(sentences[i]);
            i += 1;
        }
    }

    result.join("").trim_end().to_string()
}

#[allow(clippy::string_slice)] // Splitting on ASCII punctuation; indices are always valid UTF-8 boundaries
pub fn split_into_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut last = 0;
    let bytes = text.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        if b == b'.' || b == b'!' || b == b'?' {
            // Include trailing whitespace with the sentence
            let mut end = i + 1;
            while end < bytes.len() && bytes[end] == b' ' {
                end += 1;
            }
            sentences.push(&text[last..end]);
            last = end;
        }
    }

    // Don't forget the trailing fragment (if any)
    if last < text.len() {
        sentences.push(&text[last..]);
    }

    sentences
}

/// Detect repetition in token sequence, returning the index to truncate to if repetition found.
/// Filters out timestamp tokens (>= timestamp_begin) when looking for patterns.
/// Returns Some(truncate_index) if repetition detected, None otherwise.
pub fn detect_repetition_impl(
    tokens: &[u32],
    sample_begin: usize,
    timestamp_begin: u32,
) -> Option<usize> {
    if tokens.len() <= sample_begin {
        return None;
    }

    // Filter out timestamp tokens to get just text tokens, but remember original positions
    let text_tokens: Vec<(usize, u32)> = tokens[sample_begin..]
        .iter()
        .enumerate()
        .filter(|(_, &t)| t < timestamp_begin)
        .map(|(i, &t)| (i + sample_begin, t))
        .collect();

    // Need at least 3 tokens to detect any repetition (e.g., [A, A, A])
    if text_tokens.len() < 3 {
        return None;
    }

    let n = text_tokens.len();

    // Try different pattern lengths, starting from 1
    for pattern_len in 1..=(n / 2) {
        // Check if the last `pattern_len` tokens match the preceding `pattern_len` tokens
        let pattern_start = n - pattern_len;
        let prev_pattern_start = n - 2 * pattern_len;

        let matches = (0..pattern_len)
            .all(|i| text_tokens[prev_pattern_start + i].1 == text_tokens[pattern_start + i].1);

        if !matches {
            continue;
        }

        // Found adjacent repeated pattern - count total repetitions
        let mut repeat_count = 2;
        let mut check_start = prev_pattern_start;

        while check_start >= pattern_len {
            let earlier_start = check_start - pattern_len;
            let still_matches = (0..pattern_len)
                .all(|i| text_tokens[earlier_start + i].1 == text_tokens[pattern_start + i].1);
            if still_matches {
                repeat_count += 1;
                check_start = earlier_start;
            } else {
                break;
            }
        }

        // Trigger on: 3+ repeats of anything, or 2 repeats of 5+ token patterns
        if repeat_count >= 3 || (repeat_count >= 2 && pattern_len >= 5) {
            // Return the original token position after the first pattern
            let first_pattern_end_text_idx = check_start + pattern_len;
            let truncate_pos = text_tokens[first_pattern_end_text_idx].0;
            return Some(truncate_pos);
        }
    }

    None
}
