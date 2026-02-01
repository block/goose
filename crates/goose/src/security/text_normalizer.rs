use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Approximate character-to-token ratio for chunking estimation
const CHARS_PER_TOKEN_ESTIMATE: usize = 4;

/// Marker returned when normalization results in empty content
const EMPTY_CONTENT_MARKER: &str = "EMPTY_CONTENT";

pub struct TextNormalizer;

static PERMISSIONS_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[d\-][rwx\-]{9}").unwrap());
static FILE_SIZE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d+[BKMGT]?\b").unwrap());
static TIMESTAMP_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2}\s+\d{1,2}:\d{2}\b")
        .unwrap()
});

impl TextNormalizer {
    pub fn normalize(text: &str) -> String {
        if text.trim().is_empty() {
            return EMPTY_CONTENT_MARKER.to_string();
        }
        let cleaned = Self::remove_filesystem_metadata(text);
        let cleaned_lower = cleaned.to_lowercase();
        let words: Vec<&str> = cleaned_lower.split_whitespace().collect();
        let deduplicated = Self::deduplicate_words(&words);
        let bigram_deduped = Self::deduplicate_ngrams(&deduplicated, 2);
        let trigram_deduped = Self::deduplicate_ngrams(&bigram_deduped, 3);
        let final_result = trigram_deduped.replace(' ', "");
        if final_result.is_empty() {
            EMPTY_CONTENT_MARKER.to_string()
        } else {
            final_result
        }
    }

    #[allow(clippy::string_slice)]
    pub fn chunk_for_classification(text: &str, max_tokens: usize) -> Vec<String> {
        let max_chars = max_tokens * CHARS_PER_TOKEN_ESTIMATE;
        if text.len() <= max_chars {
            return vec![text.to_string()];
        }
        let mut chunks = Vec::new();
        let mut start = 0;
        while start < text.len() {
            let end = (start + max_chars).min(text.len());
            chunks.push(text[start..end].to_string());
            start = end;
        }
        chunks
    }

    fn remove_filesystem_metadata(text: &str) -> String {
        let no_perms = PERMISSIONS_PATTERN.replace_all(text, " ");
        let no_sizes = FILE_SIZE_PATTERN.replace_all(&no_perms, " ");
        let no_timestamps = TIMESTAMP_PATTERN.replace_all(&no_sizes, " ");
        let alpha_only: String = no_timestamps
            .chars()
            .map(|c| if c.is_alphabetic() { c } else { ' ' })
            .collect();
        let collapsed = alpha_only.split_whitespace().collect::<Vec<_>>().join(" ");
        collapsed
    }

    fn deduplicate_words(words: &[&str]) -> String {
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for word in words {
            if seen.insert(*word) {
                result.push(*word);
            }
        }
        result.join(" ")
    }

    fn deduplicate_ngrams(text: &str, n: usize) -> String {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.len() < n {
            return text.to_string();
        }
        let mut seen_ngrams: HashSet<Vec<String>> = HashSet::new();
        let mut keep_positions: HashSet<usize> = (0..words.len()).collect();
        for i in 0..=words.len().saturating_sub(n) {
            let ngram: Vec<String> = words[i..i + n].iter().map(|s| s.to_string()).collect();
            if !seen_ngrams.insert(ngram) {
                for pos in i..i + n {
                    keep_positions.remove(&pos);
                }
            }
        }

        let mut result: Vec<&str> = Vec::new();
        for (i, word) in words.iter().enumerate() {
            if keep_positions.contains(&i) {
                result.push(word);
            }
        }

        result.join(" ")
    }
}
