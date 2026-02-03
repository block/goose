//! Tool Search - Dynamic tool discovery (85% token reduction)
//!
//! Instead of loading all 50+ MCP tools upfront (55K tokens),
//! discover tools on-demand as needed (3-5 relevant tools, ~3K tokens)

use super::{ToolCategory, ToolDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for tool search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchConfig {
    pub max_results: usize,
    pub similarity_threshold: f32,
    pub include_examples: bool,
    pub cache_results: bool,
}

impl Default for ToolSearchConfig {
    fn default() -> Self {
        Self {
            max_results: 5,
            similarity_threshold: 0.7,
            include_examples: true,
            cache_results: true,
        }
    }
}

/// Result of a tool search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchResult {
    pub tool: ToolDefinition,
    pub relevance_score: f32,
    pub match_reason: String,
}

/// Tool Search Tool - Dynamic tool discovery
#[allow(dead_code)]
pub struct ToolSearchTool {
    config: ToolSearchConfig,
    tools: HashMap<String, ToolDefinition>,
    cache: HashMap<String, Vec<ToolSearchResult>>,
    embeddings: Option<ToolEmbeddings>,
}

#[allow(dead_code)]
struct ToolEmbeddings {
    tool_vectors: HashMap<String, Vec<f32>>,
}

impl ToolSearchTool {
    pub fn new(config: ToolSearchConfig) -> Self {
        Self {
            config,
            tools: HashMap::new(),
            cache: HashMap::new(),
            embeddings: None,
        }
    }

    pub fn register_tool(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.clone(), tool);
        // Invalidate cache
        self.cache.clear();
    }

    pub fn register_tools(&mut self, tools: Vec<ToolDefinition>) {
        for tool in tools {
            self.register_tool(tool);
        }
    }

    /// Search for relevant tools given a task description
    pub fn search(&mut self, query: &str) -> Vec<ToolSearchResult> {
        // Check cache first
        if self.config.cache_results {
            if let Some(cached) = self.cache.get(query) {
                return cached.clone();
            }
        }

        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        for tool in self.tools.values() {
            let score = self.calculate_relevance(tool, &query_lower, &query_words);

            if score >= self.config.similarity_threshold {
                results.push(ToolSearchResult {
                    tool: tool.clone(),
                    relevance_score: score,
                    match_reason: self.get_match_reason(tool, &query_words),
                });
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        // Limit results
        results.truncate(self.config.max_results);

        // Cache results
        if self.config.cache_results {
            self.cache.insert(query.to_string(), results.clone());
        }

        results
    }

    /// Get tools relevant to a specific task
    pub fn get_relevant_tools(&mut self, task: &str) -> Vec<ToolDefinition> {
        self.search(task).into_iter().map(|r| r.tool).collect()
    }

    /// Calculate relevance score using keyword matching
    fn calculate_relevance(&self, tool: &ToolDefinition, query: &str, query_words: &[&str]) -> f32 {
        let mut score: f32 = 0.0;
        let name_lower = tool.name.to_lowercase();
        let desc_lower = tool.description.to_lowercase();

        // Exact name match
        if query.contains(&name_lower) || name_lower.contains(query) {
            score += 0.5;
        }

        // Word matches in name
        for word in query_words {
            if name_lower.contains(word) {
                score += 0.2;
            }
        }

        // Word matches in description
        for word in query_words {
            if desc_lower.contains(word) {
                score += 0.1;
            }
        }

        // Category relevance
        let category_keywords = self.get_category_keywords(&tool.category);
        for word in query_words {
            if category_keywords
                .iter()
                .any(|k| k.contains(word) || word.contains(k))
            {
                score += 0.15;
            }
        }

        // Cap at 1.0
        score.min(1.0)
    }

    fn get_category_keywords(&self, category: &ToolCategory) -> Vec<&'static str> {
        match category {
            ToolCategory::FileSystem => vec![
                "file",
                "read",
                "write",
                "edit",
                "create",
                "delete",
                "directory",
                "path",
            ],
            ToolCategory::Search => vec!["search", "find", "grep", "glob", "pattern", "match"],
            ToolCategory::Execution => vec!["run", "execute", "bash", "shell", "command", "script"],
            ToolCategory::Web => vec!["web", "http", "fetch", "url", "api", "request"],
            ToolCategory::Database => vec!["database", "sql", "query", "table", "record"],
            ToolCategory::Git => vec!["git", "commit", "branch", "merge", "push", "pull"],
            ToolCategory::Testing => vec!["test", "assert", "verify", "check", "validate"],
            ToolCategory::General => vec![],
        }
    }

    fn get_match_reason(&self, tool: &ToolDefinition, query_words: &[&str]) -> String {
        let name_lower = tool.name.to_lowercase();
        let desc_lower = tool.description.to_lowercase();

        let matched_words: Vec<&str> = query_words
            .iter()
            .filter(|w| name_lower.contains(*w) || desc_lower.contains(*w))
            .copied()
            .collect();

        if matched_words.is_empty() {
            format!("Category match: {:?}", tool.category)
        } else {
            format!("Matched keywords: {}", matched_words.join(", "))
        }
    }

    /// Estimate token savings
    pub fn estimate_token_savings(&self, all_tools_tokens: usize) -> TokenSavings {
        let loaded_tools = self.config.max_results;
        let avg_tokens_per_tool = all_tools_tokens / self.tools.len().max(1);
        let search_tool_tokens = 500; // Approximate tokens for search tool itself

        let traditional_tokens = all_tools_tokens;
        let search_tokens = search_tool_tokens + (loaded_tools * avg_tokens_per_tool);
        let savings = traditional_tokens.saturating_sub(search_tokens);
        let savings_percent = if traditional_tokens > 0 {
            (savings as f32 / traditional_tokens as f32) * 100.0
        } else {
            0.0
        };

        TokenSavings {
            traditional_approach_tokens: traditional_tokens,
            tool_search_approach_tokens: search_tokens,
            tokens_saved: savings,
            savings_percent,
        }
    }

    /// Get total registered tools count
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Clear the search cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSavings {
    pub traditional_approach_tokens: usize,
    pub tool_search_approach_tokens: usize,
    pub tokens_saved: usize,
    pub savings_percent: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tools() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "Read".to_string(),
                description: "Read file contents".to_string(),
                category: ToolCategory::FileSystem,
                schema: None,
                examples: vec![],
                token_cost: 200,
            },
            ToolDefinition {
                name: "Write".to_string(),
                description: "Write content to a file".to_string(),
                category: ToolCategory::FileSystem,
                schema: None,
                examples: vec![],
                token_cost: 250,
            },
            ToolDefinition {
                name: "Bash".to_string(),
                description: "Execute shell commands".to_string(),
                category: ToolCategory::Execution,
                schema: None,
                examples: vec![],
                token_cost: 300,
            },
            ToolDefinition {
                name: "Grep".to_string(),
                description: "Search for patterns in files".to_string(),
                category: ToolCategory::Search,
                schema: None,
                examples: vec![],
                token_cost: 200,
            },
        ]
    }

    #[test]
    fn test_tool_search() {
        let mut search = ToolSearchTool::new(ToolSearchConfig::default());
        search.register_tools(create_test_tools());

        let results = search.search("read a file");
        assert!(!results.is_empty());
        assert_eq!(results[0].tool.name, "Read");
    }

    #[test]
    fn test_tool_search_execution() {
        let mut search = ToolSearchTool::new(ToolSearchConfig::default());
        search.register_tools(create_test_tools());

        let results = search.search("run a command");
        assert!(!results.is_empty());
        assert_eq!(results[0].tool.name, "Bash");
    }

    #[test]
    fn test_token_savings() {
        let mut search = ToolSearchTool::new(ToolSearchConfig {
            max_results: 3,
            ..Default::default()
        });
        search.register_tools(create_test_tools());

        let savings = search.estimate_token_savings(55000);
        // With 4 tools and max_results=3, savings is ~24%
        // Real benefit shows with 50+ tools where savings > 85%
        assert!(savings.savings_percent > 0.0);
        assert!(savings.tokens_saved > 0);
    }

    #[test]
    fn test_cache() {
        let mut search = ToolSearchTool::new(ToolSearchConfig {
            cache_results: true,
            ..Default::default()
        });
        search.register_tools(create_test_tools());

        // First search
        let results1 = search.search("read file");
        // Second search should hit cache
        let results2 = search.search("read file");

        assert_eq!(results1.len(), results2.len());
    }
}
