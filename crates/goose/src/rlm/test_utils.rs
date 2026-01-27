//! Test utilities for RLM functionality
//!
//! This module provides test utilities like needle-in-haystack generators
//! for testing RLM's large context handling capabilities.

#[cfg(test)]
use super::context_store::ContextStore;
#[cfg(test)]
use super::RlmConfig;
#[cfg(test)]
use tempfile::TempDir;

/// Generate a "needle in haystack" test context
/// Places a magic number somewhere in a large amount of random text
pub fn generate_needle_haystack(
    total_chars: usize,
    needle: &str,
    needle_position: f64, // 0.0 to 1.0, where to place the needle
) -> String {
    let needle_pos = (total_chars as f64 * needle_position) as usize;

    // Generate filler text (lorem ipsum style)
    let words = [
        "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit",
        "sed", "do", "eiusmod", "tempor", "incididunt", "ut", "labore", "et", "dolore",
        "magna", "aliqua", "enim", "ad", "minim", "veniam", "quis", "nostrud",
        "exercitation", "ullamco", "laboris", "nisi", "aliquip", "ex", "ea", "commodo",
        "consequat", "duis", "aute", "irure", "in", "reprehenderit", "voluptate",
        "velit", "esse", "cillum", "fugiat", "nulla", "pariatur", "excepteur", "sint",
        "occaecat", "cupidatat", "non", "proident", "sunt", "culpa", "qui", "officia",
        "deserunt", "mollit", "anim", "id", "est", "laborum",
    ];

    let mut result = String::with_capacity(total_chars + needle.len() + 100);
    let mut word_idx = 0;
    let mut needle_inserted = false;

    while result.len() < total_chars {
        // Check if we should insert the needle
        if !needle_inserted && result.len() >= needle_pos {
            result.push_str("\n\n[IMPORTANT: ");
            result.push_str(needle);
            result.push_str("]\n\n");
            needle_inserted = true;
            continue;
        }

        // Add a word
        let word = words[word_idx % words.len()];

        if !result.is_empty() && !result.ends_with('\n') {
            result.push(' ');
        }

        // Capitalize first word of "sentence"
        if result.is_empty() || result.ends_with(". ") || result.ends_with('\n') {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                result.push(first.to_ascii_uppercase());
                result.extend(chars);
            }
        } else {
            result.push_str(word);
        }

        // Add punctuation occasionally
        if word_idx % 12 == 11 {
            result.push('.');
            if word_idx % 60 == 59 {
                result.push('\n');
            }
        }

        word_idx += 1;
    }

    // Ensure needle is inserted if we haven't yet
    if !needle_inserted {
        result.push_str("\n\n[IMPORTANT: ");
        result.push_str(needle);
        result.push_str("]\n\n");
    }

    result
}

/// Generate a multi-document test context
/// Creates multiple "documents" each containing some information
pub fn generate_multi_document_context(doc_count: usize, chars_per_doc: usize) -> (String, Vec<(String, String)>) {
    let mut context = String::new();
    let mut facts = Vec::new();

    for i in 0..doc_count {
        let doc_title = format!("Document_{}", i + 1);
        let fact_key = format!("fact_{}", i + 1);
        let fact_value = format!("value_{}", (i * 17 + 42) % 1000); // pseudo-random values

        context.push_str(&format!("\n=== {} ===\n", doc_title));
        context.push_str(&format!("This document contains information about {}.\n", doc_title));
        context.push_str(&format!("The key piece of information is: {} = {}\n", fact_key, fact_value));

        // Add filler
        let filler_needed = chars_per_doc.saturating_sub(context.len() % chars_per_doc);
        for _ in 0..(filler_needed / 20) {
            context.push_str("Additional context. ");
        }
        context.push('\n');

        facts.push((fact_key, fact_value));
    }

    (context, facts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_needle_haystack() {
        let needle = "The secret code is XYZ789";
        let context = generate_needle_haystack(10_000, needle, 0.5);

        assert!(context.contains(needle));
        assert!(context.len() >= 10_000);
    }

    #[test]
    fn test_generate_large_haystack() {
        let needle = "MAGIC_NUMBER_42";
        let context = generate_needle_haystack(1_000_000, needle, 0.75);

        assert!(context.contains(needle));
        assert!(context.len() >= 1_000_000);

        // Verify needle is roughly at 75% position
        let needle_pos = context.find(needle).unwrap();
        let ratio = needle_pos as f64 / context.len() as f64;
        assert!(ratio > 0.6 && ratio < 0.9, "Needle position ratio: {}", ratio);
    }

    #[test]
    fn test_generate_multi_document() {
        let (context, facts) = generate_multi_document_context(10, 1000);

        assert_eq!(facts.len(), 10);
        for (key, value) in &facts {
            assert!(context.contains(key));
            assert!(context.contains(value));
        }
    }

    #[tokio::test]
    async fn test_rlm_context_store_with_needle_haystack() {
        let temp_dir = TempDir::new().unwrap();
        let store = ContextStore::new(temp_dir.path().to_path_buf());

        let needle = "SECRET_CODE_12345";
        let context = generate_needle_haystack(100_000, needle, 0.5);

        // Store the context
        let metadata = store.store_context(&context).await.unwrap();
        assert!(metadata.length >= 100_000);

        // Verify we can find the needle by reading slices
        let needle_pos = context.find(needle).unwrap();
        let chunk_start = (needle_pos / 10_000) * 10_000;
        let chunk_end = chunk_start + 20_000;

        let slice = store.read_slice(chunk_start, chunk_end).await.unwrap();
        assert!(slice.contains(needle), "Needle not found in slice");
    }

    #[tokio::test]
    async fn test_rlm_chunking_strategy() {
        let temp_dir = TempDir::new().unwrap();
        let chunk_size = 50_000;
        let store = ContextStore::with_chunk_size(temp_dir.path().to_path_buf(), chunk_size);

        // Create context that spans multiple chunks
        let context = "x".repeat(150_000);
        let metadata = store.store_context(&context).await.unwrap();

        assert_eq!(metadata.chunk_count, 3);
        assert_eq!(metadata.chunk_boundaries[0], (0, 50_000));
        assert_eq!(metadata.chunk_boundaries[1], (50_000, 100_000));
        assert_eq!(metadata.chunk_boundaries[2], (100_000, 150_000));
    }

    #[test]
    fn test_rlm_config_threshold() {
        let config = RlmConfig {
            enabled: true,
            context_threshold: 50_000,
            ..Default::default()
        };

        // Generate contexts of different sizes
        let small = "a".repeat(40_000);
        let large = "a".repeat(60_000);

        assert!(!crate::rlm::is_rlm_candidate(&small, &config));
        assert!(crate::rlm::is_rlm_candidate(&large, &config));
    }
}
