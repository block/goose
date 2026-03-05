//! Semantic routing via TF-IDF cosine similarity.
//!
//! Provides a lightweight, zero-dependency embedding layer that sits between
//! keyword matching (layer 1, <10ms) and LLM-as-Judge (layer 3, ~1-5s).
//! Routes are defined as collections of descriptive text (from `AgentMode`
//! `when_to_use` + `description`), converted to TF-IDF vectors at build time,
//! and matched against user messages via cosine similarity at query time (~1ms).
//!
//! This follows the Semantic Router pattern (Aurelio Labs) but uses TF-IDF
//! instead of neural embeddings to avoid external dependencies.
//!
//! # Architecture
//!
//! ```text
//! User Message
//!   ├── [1] Keyword/Rule Check (IntentRouter, <10ms)
//!   │     → If confident (>0.8), route immediately
//!   ├── [2] TF-IDF Cosine Similarity (SemanticRouter, ~1ms)
//!   │     → If confident (>threshold), route
//!   └── [3] LLM-as-Judge (OrchestratorAgent, ~1-5s)
//!         → For ambiguous/novel queries
//! ```

use std::collections::{HashMap, HashSet};

/// A route target: an (agent_name, mode_slug) pair with pre-computed TF-IDF vector.
#[derive(Debug, Clone)]
pub struct SemanticRoute {
    pub agent_name: String,
    pub mode_slug: String,
    /// Pre-computed TF-IDF vector (term → weight).
    tfidf: HashMap<String, f64>,
    /// L2 norm of the TF-IDF vector (cached for fast cosine similarity).
    norm: f64,
}

/// Result of a semantic routing query.
#[derive(Debug, Clone)]
pub struct SemanticMatch {
    pub agent_name: String,
    pub mode_slug: String,
    pub similarity: f32,
    pub top_terms: Vec<String>,
}

/// TF-IDF-based semantic router.
///
/// Pre-computes IDF from all route documents at construction time.
/// At query time, computes TF-IDF for the query and finds the most
/// similar route via cosine similarity.
#[derive(Debug, Clone)]
pub struct SemanticRouter {
    routes: Vec<SemanticRoute>,
    /// Inverse document frequency for each term across all route documents.
    idf: HashMap<String, f64>,
    /// Minimum similarity threshold for a confident match.
    threshold: f32,
}

impl SemanticRouter {
    /// Build a new semantic router from route definitions.
    ///
    /// Each route is defined as `(agent_name, mode_slug, document_text)` where
    /// `document_text` is typically the concatenation of `when_to_use` and
    /// `description` from `AgentMode`.
    pub fn new(route_definitions: Vec<(String, String, String)>, threshold: f32) -> Self {
        if route_definitions.is_empty() {
            return Self {
                routes: vec![],
                idf: HashMap::new(),
                threshold,
            };
        }

        // Tokenize all documents
        let documents: Vec<Vec<String>> = route_definitions
            .iter()
            .map(|(_, _, text)| tokenize(text))
            .collect();

        // Compute IDF across all documents
        let num_docs = documents.len() as f64;
        let mut doc_freq: HashMap<String, usize> = HashMap::new();
        for doc_tokens in &documents {
            let unique_terms: HashSet<&String> = doc_tokens.iter().collect();
            for term in unique_terms {
                *doc_freq.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let idf: HashMap<String, f64> = doc_freq
            .into_iter()
            .map(|(term, df)| {
                // Smoothed IDF: log((N + 1) / (df + 1)) + 1
                let idf_val = ((num_docs + 1.0) / (df as f64 + 1.0)).ln() + 1.0;
                (term, idf_val)
            })
            .collect();

        // Build TF-IDF vectors for each route
        let routes: Vec<SemanticRoute> = route_definitions
            .into_iter()
            .zip(documents.iter())
            .map(|((agent_name, mode_slug, _), tokens)| {
                let tfidf = compute_tfidf(tokens, &idf);
                let norm = l2_norm(&tfidf);
                SemanticRoute {
                    agent_name,
                    mode_slug,
                    tfidf,
                    norm,
                }
            })
            .collect();

        Self {
            routes,
            idf,
            threshold,
        }
    }

    /// Query the router with a user message.
    ///
    /// Returns the best match if similarity exceeds the threshold, plus all
    /// matches sorted by similarity (for debugging/logging).
    pub fn route(&self, user_message: &str) -> Option<SemanticMatch> {
        if self.routes.is_empty() {
            return None;
        }

        let query_tokens = tokenize(user_message);
        if query_tokens.is_empty() {
            return None;
        }

        let query_tfidf = compute_tfidf(&query_tokens, &self.idf);
        let query_norm = l2_norm(&query_tfidf);

        if query_norm == 0.0 {
            return None;
        }

        let mut best: Option<(usize, f64)> = None;

        for (i, route) in self.routes.iter().enumerate() {
            if route.norm == 0.0 {
                continue;
            }
            let sim = cosine_similarity(&query_tfidf, query_norm, &route.tfidf, route.norm);
            match best {
                Some((_, best_sim)) if sim > best_sim => {
                    best = Some((i, sim));
                }
                None => {
                    best = Some((i, sim));
                }
                _ => {}
            }
        }

        best.and_then(|(idx, sim)| {
            let sim_f32 = sim as f32;
            if sim_f32 >= self.threshold {
                let route = &self.routes[idx];
                let top_terms = find_top_matching_terms(&query_tfidf, &route.tfidf, 5);
                Some(SemanticMatch {
                    agent_name: route.agent_name.clone(),
                    mode_slug: route.mode_slug.clone(),
                    similarity: sim_f32,
                    top_terms,
                })
            } else {
                None
            }
        })
    }

    /// Get all matches above threshold, sorted by similarity (descending).
    pub fn route_all(&self, user_message: &str) -> Vec<SemanticMatch> {
        if self.routes.is_empty() {
            return vec![];
        }

        let query_tokens = tokenize(user_message);
        if query_tokens.is_empty() {
            return vec![];
        }

        let query_tfidf = compute_tfidf(&query_tokens, &self.idf);
        let query_norm = l2_norm(&query_tfidf);

        if query_norm == 0.0 {
            return vec![];
        }

        let mut matches: Vec<SemanticMatch> = self
            .routes
            .iter()
            .filter(|r| r.norm > 0.0)
            .map(|route| {
                let sim = cosine_similarity(&query_tfidf, query_norm, &route.tfidf, route.norm);
                let top_terms = find_top_matching_terms(&query_tfidf, &route.tfidf, 5);
                SemanticMatch {
                    agent_name: route.agent_name.clone(),
                    mode_slug: route.mode_slug.clone(),
                    similarity: sim as f32,
                    top_terms,
                }
            })
            .filter(|m| m.similarity >= self.threshold)
            .collect();

        matches.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches
    }

    /// Number of registered routes.
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// The similarity threshold.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }
}

// ─── Text Processing ──────────────────────────────────────────────────────

/// Stop words filtered during tokenization.
const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
    "from", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does",
    "did", "will", "would", "could", "should", "may", "might", "can", "shall", "it", "its", "this",
    "that", "these", "those", "i", "you", "he", "she", "we", "they", "me", "him", "her", "us",
    "them", "my", "your", "his", "our", "their", "what", "which", "who", "whom", "when", "where",
    "why", "how", "not", "no", "if", "then", "than", "so", "as", "just", "only", "also", "very",
    "too", "up", "out", "about", "into", "over", "after", "before", "between", "under", "above",
    "all", "each", "every", "both", "few", "more", "most", "other", "some", "such", "any", "own",
];

/// Tokenize text into lowercase terms, filtering stop words and short tokens.
fn tokenize(text: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOP_WORDS.iter().copied().collect();
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|w| w.len() >= 2 && !stop.contains(w))
        .map(|w| {
            // Simple suffix stemming: strip common suffixes
            stem(w)
        })
        .collect()
}

/// Minimal suffix stemmer — strips common English suffixes and deduplicates
/// trailing doubled consonants (e.g. "debugg" → "debug").
/// Not linguistically rigorous, but sufficient for route matching.
fn stem(word: &str) -> String {
    if word.len() <= 5 {
        return word.to_string();
    }
    // Order matters: longest suffixes first
    for suffix in &[
        "ation", "ment", "ness", "ious", "ible", "able", "ting", "ive", "ful", "ous", "ing", "ies",
        "ure", "ly", "ed", "er", "es", "al",
    ] {
        if let Some(stripped) = word.strip_suffix(suffix) {
            if stripped.len() >= 4 {
                return dedup_trailing(stripped);
            }
        }
    }
    // Simple plural: strip trailing "s" if result is long enough
    if let Some(stripped) = word.strip_suffix('s') {
        if stripped.len() >= 4 {
            return stripped.to_string();
        }
    }
    word.to_string()
}

/// Remove doubled trailing consonant: "debugg" → "debug", "runn" → "run".
fn dedup_trailing(s: &str) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    if len >= 2 && chars[len - 1] == chars[len - 2] {
        let c = chars[len - 1];
        if c.is_ascii_alphabetic() && !"aeiou".contains(c) {
            chars.pop();
            return chars.into_iter().collect();
        }
    }
    s.to_string()
}

// ─── TF-IDF Computation ──────────────────────────────────────────────────

/// Compute TF-IDF vector for a token sequence given pre-computed IDF.
fn compute_tfidf(tokens: &[String], idf: &HashMap<String, f64>) -> HashMap<String, f64> {
    let total = tokens.len() as f64;
    if total == 0.0 {
        return HashMap::new();
    }

    // Term frequency
    let mut tf: HashMap<String, f64> = HashMap::new();
    for token in tokens {
        *tf.entry(token.clone()).or_insert(0.0) += 1.0;
    }

    // TF-IDF = (count / total) * IDF
    tf.into_iter()
        .map(|(term, count)| {
            let tf_val = count / total;
            let idf_val = idf.get(&term).copied().unwrap_or(1.0);
            (term, tf_val * idf_val)
        })
        .collect()
}

/// L2 norm of a sparse vector.
fn l2_norm(vec: &HashMap<String, f64>) -> f64 {
    vec.values().map(|v| v * v).sum::<f64>().sqrt()
}

/// Cosine similarity between two sparse TF-IDF vectors with pre-computed norms.
fn cosine_similarity(
    a: &HashMap<String, f64>,
    a_norm: f64,
    b: &HashMap<String, f64>,
    b_norm: f64,
) -> f64 {
    if a_norm == 0.0 || b_norm == 0.0 {
        return 0.0;
    }

    // Dot product: iterate over the smaller vector
    let (small, large) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    let dot: f64 = small
        .iter()
        .filter_map(|(term, val)| large.get(term).map(|other_val| val * other_val))
        .sum();

    dot / (a_norm * b_norm)
}

/// Find top N terms contributing most to similarity between query and route.
fn find_top_matching_terms(
    query: &HashMap<String, f64>,
    route: &HashMap<String, f64>,
    n: usize,
) -> Vec<String> {
    let mut contributions: Vec<(String, f64)> = query
        .iter()
        .filter_map(|(term, q_val)| route.get(term).map(|r_val| (term.clone(), q_val * r_val)))
        .collect();

    contributions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    contributions.into_iter().take(n).map(|(t, _)| t).collect()
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("Write code and debug errors");
        assert!(tokens.contains(&"write".to_string()));
        assert!(tokens.contains(&"code".to_string()));
        assert!(tokens.contains(&"debug".to_string()));
        assert!(tokens.contains(&"error".to_string())); // stemmed from "errors"
                                                        // "and" is a stop word
        assert!(!tokens.iter().any(|t| t == "and"));
    }

    #[test]
    fn test_tokenize_filters_short_words() {
        let tokens = tokenize("I a am go to be");
        // All are either stop words or < 2 chars
        assert!(tokens.is_empty() || tokens.iter().all(|t| t.len() >= 2));
    }

    #[test]
    fn test_stem_common_suffixes() {
        assert_eq!(stem("debugging"), "debug");
        assert_eq!(stem("implementation"), "implement");
        assert_eq!(stem("deployment"), "deploy");
        assert_eq!(stem("testing"), "test");
        assert_eq!(stem("reviewed"), "review");
        assert_eq!(stem("configures"), "configur");
        // Short words should not be stemmed further
        assert_eq!(stem("fix"), "fix");
        assert_eq!(stem("run"), "run");
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let mut a = HashMap::new();
        a.insert("test".to_string(), 1.0);
        a.insert("code".to_string(), 0.5);
        let norm_a = l2_norm(&a);

        let sim = cosine_similarity(&a, norm_a, &a, norm_a);
        assert!(
            (sim - 1.0).abs() < 1e-10,
            "Identical vectors should have similarity 1.0"
        );
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let mut a = HashMap::new();
        a.insert("test".to_string(), 1.0);
        let norm_a = l2_norm(&a);

        let mut b = HashMap::new();
        b.insert("deploy".to_string(), 1.0);
        let norm_b = l2_norm(&b);

        let sim = cosine_similarity(&a, norm_a, &b, norm_b);
        assert!(
            sim.abs() < 1e-10,
            "Orthogonal vectors should have similarity 0.0"
        );
    }

    #[test]
    fn test_empty_router() {
        let router = SemanticRouter::new(vec![], 0.1);
        assert_eq!(router.route_count(), 0);
        assert!(router.route("hello world").is_none());
    }

    #[test]
    fn test_route_developer_write() {
        let routes = vec![
            (
                "Developer Agent".into(),
                "write".into(),
                "Write code, implement features, create files, build projects, add functionality, \
                 fix issues, update configurations, develop software, deploy infrastructure"
                    .into(),
            ),
            (
                "QA Agent".into(),
                "debug".into(),
                "Debug errors, diagnose crashes, troubleshoot failures, fix bugs, investigate \
                 broken behavior, reproduce issues, isolate root cause, analyze stack traces"
                    .into(),
            ),
            (
                "Research Agent".into(),
                "ask".into(),
                "Research topics, compare technologies, fact-check, analyze competition, \
                 synthesize documentation, investigate alternatives, survey landscape"
                    .into(),
            ),
        ];

        let router = SemanticRouter::new(routes, 0.05);

        let result = router.route("implement a new feature for user authentication");
        assert!(result.is_some(), "Should match a route");
        let m = result.unwrap();
        assert_eq!(m.agent_name, "Developer Agent");
        assert_eq!(m.mode_slug, "write");
    }

    #[test]
    fn test_route_qa_debug() {
        let routes = vec![
            (
                "Developer Agent".into(),
                "write".into(),
                "Write code, implement features, create files, build projects, add functionality, \
                 fix issues, update configurations, develop software, deploy infrastructure"
                    .into(),
            ),
            (
                "QA Agent".into(),
                "debug".into(),
                "Debug errors, diagnose crashes, troubleshoot failures, fix bugs, investigate \
                 broken behavior, reproduce issues, isolate root cause, analyze stack traces, \
                 examine logs"
                    .into(),
            ),
            (
                "Security Agent".into(),
                "review".into(),
                "Security review, threat modeling, vulnerability analysis, compliance audit, \
                 penetration testing, security assessment"
                    .into(),
            ),
        ];

        let router = SemanticRouter::new(routes, 0.05);

        let result =
            router.route("there's a crash in the logs, can you debug this stack trace error?");
        assert!(result.is_some(), "Should match a route");
        let m = result.unwrap();
        assert_eq!(m.agent_name, "QA Agent");
        assert_eq!(m.mode_slug, "debug");
    }

    #[test]
    fn test_route_security_review() {
        let routes = vec![
            (
                "Developer Agent".into(),
                "write".into(),
                "Write code, implement features, create files, build projects".into(),
            ),
            (
                "Security Agent".into(),
                "review".into(),
                "Security review, threat modeling, vulnerability analysis, compliance audit, \
                 penetration testing, security assessment, CVE analysis"
                    .into(),
            ),
        ];

        let router = SemanticRouter::new(routes, 0.05);

        let result = router.route("check for security vulnerabilities and do a threat model");
        assert!(result.is_some(), "Should match a route");
        let m = result.unwrap();
        assert_eq!(m.agent_name, "Security Agent");
    }

    #[test]
    fn test_route_below_threshold_returns_none() {
        let routes = vec![(
            "Developer Agent".into(),
            "write".into(),
            "Write code, implement features, create files, build projects".into(),
        )];

        // Very high threshold
        let router = SemanticRouter::new(routes, 0.99);

        let result = router.route("tell me a joke");
        assert!(result.is_none(), "Should not match with high threshold");
    }

    #[test]
    fn test_route_all_returns_sorted() {
        let routes = vec![
            (
                "Developer Agent".into(),
                "write".into(),
                "Write code, implement features, create files, build projects, add functionality"
                    .into(),
            ),
            (
                "Developer Agent".into(),
                "debug".into(),
                "Debug errors, diagnose crashes, troubleshoot failures, fix bugs, code issues"
                    .into(),
            ),
            (
                "Research Agent".into(),
                "ask".into(),
                "Research topics, compare technologies, analyze documentation".into(),
            ),
        ];

        let router = SemanticRouter::new(routes, 0.01);

        let matches = router.route_all("fix this code bug and debug the error");
        assert!(!matches.is_empty(), "Should have matches");
        // Verify sorted descending
        for window in matches.windows(2) {
            assert!(
                window[0].similarity >= window[1].similarity,
                "Results should be sorted by similarity descending"
            );
        }
    }

    #[test]
    fn test_top_matching_terms() {
        let mut query = HashMap::new();
        query.insert("debug".to_string(), 2.0);
        query.insert("error".to_string(), 1.5);
        query.insert("crash".to_string(), 1.0);
        query.insert("random".to_string(), 0.5);

        let mut route = HashMap::new();
        route.insert("debug".to_string(), 2.0);
        route.insert("error".to_string(), 1.0);
        route.insert("crash".to_string(), 0.8);

        let terms = find_top_matching_terms(&query, &route, 2);
        assert_eq!(terms.len(), 2);
        assert_eq!(terms[0], "debug"); // highest contribution: 2.0 * 2.0 = 4.0
        assert_eq!(terms[1], "error"); // second: 1.5 * 1.0 = 1.5
    }
}
