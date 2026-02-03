# Agentic Goose - Feature-by-Feature Implementation Guide

## Overview

This guide provides step-by-step instructions for implementing each feature, including:
- Code to write
- How to build
- How to test (including real-time Playwright visualization)
- Verification criteria

---

## Feature 1: StateGraph Engine

### 1.1 What We're Building

A stateful execution engine that enables Code → Test → Fix → Test loops until success.

```
┌─────────────────────────────────────────────────────────────────┐
│                    StateGraph Engine                             │
│                                                                  │
│   ┌──────┐     ┌──────┐     ┌──────┐     ┌──────────┐          │
│   │ CODE │────►│ TEST │────►│ FIX  │────►│ VALIDATE │          │
│   └──────┘     └──┬───┘     └──────┘     └──────────┘          │
│                   │              │                               │
│                   │   fail       │                               │
│                   └──────────────┘                               │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### 1.2 Files to Create

#### `crates/goose/src/agents/state_graph/mod.rs`

```rust
//! StateGraph execution engine for self-correcting agent loops
//!
//! This module provides a graph-based execution model where nodes
//! represent actions (code, test, fix) and edges represent transitions.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub mod config;
pub mod edges;
pub mod execution;
pub mod nodes;

pub use config::StateGraphConfig;
pub use edges::ConditionalEdge;
pub use execution::GraphExecutor;
pub use nodes::{NodeHandler, NodeResult, NodeState};

/// The main StateGraph structure
#[derive(Debug)]
pub struct StateGraph<S: Clone + Send + Sync> {
    /// Nodes in the graph, keyed by name
    nodes: HashMap<String, Arc<dyn NodeHandler<S>>>,

    /// Edges defining transitions between nodes
    edges: HashMap<String, Vec<ConditionalEdge<S>>>,

    /// Entry point node name
    entry_point: String,

    /// Maximum iterations before stopping
    max_iterations: usize,

    /// Current state
    state: Arc<RwLock<S>>,
}

impl<S: Clone + Send + Sync + 'static> StateGraph<S> {
    /// Create a new StateGraph
    pub fn new(entry_point: &str, max_iterations: usize, initial_state: S) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            entry_point: entry_point.to_string(),
            max_iterations,
            state: Arc::new(RwLock::new(initial_state)),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, name: &str, handler: Arc<dyn NodeHandler<S>>) {
        self.nodes.insert(name.to_string(), handler);
    }

    /// Add an edge between nodes
    pub fn add_edge(&mut self, from: &str, edge: ConditionalEdge<S>) {
        self.edges
            .entry(from.to_string())
            .or_default()
            .push(edge);
    }

    /// Execute the graph until success or max iterations
    pub async fn execute(&self) -> GraphResult<S> {
        let executor = GraphExecutor::new(self);
        executor.run().await
    }
}

/// Result of graph execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphResult<S> {
    pub success: bool,
    pub iterations: usize,
    pub final_state: S,
    pub node_history: Vec<String>,
    pub error: Option<String>,
}
```

#### `crates/goose/src/agents/state_graph/nodes/mod.rs`

```rust
//! Node handlers for StateGraph execution

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod code_node;
pub mod fix_node;
pub mod test_node;
pub mod validate_node;

pub use code_node::CodeNode;
pub use fix_node::FixNode;
pub use test_node::TestNode;
pub use validate_node::ValidateNode;

/// Result returned by a node after execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResult {
    /// Whether the node executed successfully
    pub success: bool,

    /// Output message from the node
    pub message: String,

    /// Data to pass to next node
    pub data: serde_json::Value,

    /// Suggested next node (can be overridden by edges)
    pub suggested_next: Option<String>,
}

/// Current state of a node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeState {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
}

/// Trait that all nodes must implement
#[async_trait]
pub trait NodeHandler<S: Clone + Send + Sync>: Send + Sync {
    /// Execute this node with the given state
    async fn execute(&self, state: &mut S) -> NodeResult;

    /// Get the name of this node
    fn name(&self) -> &str;

    /// Get a description of what this node does
    fn description(&self) -> &str;
}
```

#### `crates/goose/src/agents/state_graph/nodes/test_node.rs`

```rust
//! TestNode - Executes tests with optional Playwright visualization

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::agents::extension_manager::ExtensionManager;
use super::{NodeHandler, NodeResult};

/// Configuration for test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Test command to run (e.g., "pytest tests/")
    pub command: String,

    /// Test framework (pytest, jest, cargo, go)
    pub framework: TestFramework,

    /// Enable visual Playwright testing
    pub visual: bool,

    /// Slow motion delay for visual testing (ms)
    pub slow_mo: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestFramework {
    Pytest,
    Jest,
    Cargo,
    GoTest,
    Playwright,
}

/// State for code-test-fix cycles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeTestFixState {
    pub files_modified: Vec<String>,
    pub test_results: Vec<TestResult>,
    pub patches_applied: Vec<String>,
    pub iteration: usize,
    pub all_tests_passed: bool,
}

/// Structured test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub status: TestStatus,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub duration_ms: u64,
    pub screenshot: Option<String>,  // Base64 for Playwright
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestStatus {
    Pass,
    Fail,
    Skip,
    Error,
}

/// Node that executes tests
pub struct TestNode {
    name: String,
    config: TestConfig,
    extension_manager: ExtensionManager,
}

impl TestNode {
    pub fn new(config: TestConfig, extension_manager: ExtensionManager) -> Self {
        Self {
            name: "test".to_string(),
            config,
            extension_manager,
        }
    }

    /// Execute tests with Playwright visualization
    async fn run_playwright_visual(&self, state: &mut CodeTestFixState) -> NodeResult {
        // Call Playwright MCP with headful mode
        let playwright_params = serde_json::json!({
            "action": "run_tests",
            "headful": true,              // VISIBLE TO USER!
            "slowMo": self.config.slow_mo.unwrap_or(500),
            "viewport": {"width": 1280, "height": 720}
        });

        println!("📺 Launching visible browser for testing...");
        println!("🎬 WATCH: User can see test execution in real-time");

        let result = self.extension_manager
            .call_tool("playwright", "browser_run_tests", playwright_params)
            .await;

        match result {
            Ok(response) => {
                let test_results: Vec<TestResult> = serde_json::from_value(
                    response.get("results").cloned().unwrap_or_default()
                ).unwrap_or_default();

                let all_passed = test_results.iter().all(|r| r.status == TestStatus::Pass);
                state.test_results = test_results.clone();
                state.all_tests_passed = all_passed;

                // Log visual feedback
                for result in &test_results {
                    let icon = match result.status {
                        TestStatus::Pass => "✅",
                        TestStatus::Fail => "❌",
                        TestStatus::Skip => "⏭️",
                        TestStatus::Error => "💥",
                    };
                    println!("  {} {}", icon, result.name);

                    if result.status == TestStatus::Fail {
                        if let Some(msg) = &result.message {
                            println!("     └─ {}", msg);
                        }
                    }
                }

                NodeResult {
                    success: all_passed,
                    message: if all_passed {
                        "All tests passed!".to_string()
                    } else {
                        format!("{} tests failed", test_results.iter().filter(|r| r.status == TestStatus::Fail).count())
                    },
                    data: serde_json::to_value(&test_results).unwrap(),
                    suggested_next: if all_passed {
                        Some("validate".to_string())
                    } else {
                        Some("fix".to_string())
                    },
                }
            }
            Err(e) => NodeResult {
                success: false,
                message: format!("Test execution error: {}", e),
                data: serde_json::Value::Null,
                suggested_next: Some("fix".to_string()),
            }
        }
    }

    /// Execute tests via shell command with structured output
    async fn run_shell_tests(&self, state: &mut CodeTestFixState) -> NodeResult {
        let command = match self.config.framework {
            TestFramework::Pytest => format!("{} --json-report --json-report-file=/tmp/test_results.json", self.config.command),
            TestFramework::Jest => format!("{} --json --outputFile=/tmp/test_results.json", self.config.command),
            TestFramework::Cargo => format!("{} -- --format json", self.config.command),
            TestFramework::GoTest => format!("{} -json", self.config.command),
            TestFramework::Playwright => self.config.command.clone(),
        };

        let shell_params = serde_json::json!({
            "command": command
        });

        let result = self.extension_manager
            .call_tool("developer", "shell", shell_params)
            .await;

        // Parse structured output based on framework
        // ... implementation details ...

        NodeResult {
            success: state.all_tests_passed,
            message: "Tests executed".to_string(),
            data: serde_json::to_value(&state.test_results).unwrap(),
            suggested_next: if state.all_tests_passed {
                Some("validate".to_string())
            } else {
                Some("fix".to_string())
            },
        }
    }
}

#[async_trait]
impl NodeHandler<CodeTestFixState> for TestNode {
    async fn execute(&self, state: &mut CodeTestFixState) -> NodeResult {
        println!("\n[StateGraph] Entering TEST node (iteration {})", state.iteration);

        if self.config.visual {
            self.run_playwright_visual(state).await
        } else {
            self.run_shell_tests(state).await
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Execute tests and parse results"
    }
}
```

### 1.3 Build Instructions

```bash
# Navigate to goose directory
cd C:\Users\Admin\Downloads\projects\goose

# Build the goose crate with new state_graph module
cargo build -p goose

# If there are errors, fix them and rebuild
cargo build -p goose 2>&1 | head -50
```

### 1.4 Test Instructions

#### Unit Tests

Create `crates/goose/src/agents/state_graph/tests.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_state_graph_creation() {
        let initial_state = CodeTestFixState {
            files_modified: vec![],
            test_results: vec![],
            patches_applied: vec![],
            iteration: 0,
            all_tests_passed: false,
        };

        let graph = StateGraph::new("code", 10, initial_state);
        assert_eq!(graph.entry_point, "code");
        assert_eq!(graph.max_iterations, 10);
    }

    #[tokio::test]
    async fn test_node_execution() {
        // Test that nodes execute correctly
        // ...
    }

    #[tokio::test]
    async fn test_edge_transitions() {
        // Test that edges work correctly
        // ...
    }
}
```

Run tests:

```bash
# Run state_graph unit tests
cargo test -p goose state_graph -- --nocapture

# Expected output:
# running 3 tests
# test agents::state_graph::tests::test_state_graph_creation ... ok
# test agents::state_graph::tests::test_node_execution ... ok
# test agents::state_graph::tests::test_edge_transitions ... ok
```

#### Visual Testing (Playwright)

```bash
# Start a sample web app for testing
cd /path/to/sample-app
npm start &

# Run goose with visual testing enabled
goose --visual "Fix the login tests"

# User will see:
# 1. Browser window opens
# 2. Agent navigates to localhost:3000
# 3. Agent clicks, types, interacts
# 4. User watches in real-time
# 5. Test results displayed
```

### 1.5 Verification Criteria

- [ ] `cargo build -p goose` succeeds
- [ ] `cargo test -p goose state_graph` passes all tests
- [ ] StateGraph can execute Code → Test → Fix loop
- [ ] TestNode with `visual: true` opens visible browser
- [ ] User can watch agent interact with web UI
- [ ] Graph exits successfully when all tests pass
- [ ] Graph respects max_iterations limit

---

## Feature 2: Mem0 Semantic Memory

### 2.1 What We're Building

A Python MCP sidecar that provides semantic memory using Mem0.

```
┌─────────────────────────────────────────────────────────────────┐
│                    Mem0 Memory Sidecar                           │
│                                                                  │
│  GOOSE ──► semantic_search("auth") ──► Mem0 ──► Qdrant          │
│                                                    │             │
│  GOOSE ◄── [JWT patterns, middleware location] ◄──┘             │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### 2.2 Files to Create

#### `goose-mem0-mcp/pyproject.toml`

```toml
[project]
name = "goose-mem0-mcp"
version = "0.1.0"
description = "Mem0 semantic memory MCP server for Goose"
requires-python = ">=3.10"
dependencies = [
    "mcp>=1.0.0",
    "mem0ai>=1.0.0",
    "qdrant-client>=1.7.0",
]

[project.scripts]
goose-mem0-mcp = "goose_mem0_mcp.server:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
```

#### `goose-mem0-mcp/src/goose_mem0_mcp/server.py`

```python
#!/usr/bin/env python3
"""
Mem0 Semantic Memory MCP Server for Goose

Provides long-term semantic memory using Mem0's hybrid storage:
- Vector store (Qdrant) for semantic search
- Key-value store for exact lookups
- Graph store for relationships
"""

import asyncio
import json
import os
from typing import Optional, List, Dict, Any

from mcp import Server, Tool, Resource
from mcp.types import TextContent
from mem0 import Memory

# Initialize server
server = Server("goose-mem0-mcp")

# Initialize Mem0 memory
# Configure based on environment variables
mem0_config = {
    "vector_store": {
        "provider": "qdrant",
        "config": {
            "host": os.getenv("QDRANT_HOST", "localhost"),
            "port": int(os.getenv("QDRANT_PORT", 6333)),
        }
    },
    "llm": {
        "provider": "openai",
        "config": {
            "model": os.getenv("MEM0_LLM_MODEL", "gpt-4o-mini"),
        }
    }
}

memory = Memory.from_config(mem0_config)


@server.tool("semantic_remember")
async def semantic_remember(
    content: str,
    category: str,
    user_id: str,
    tags: Optional[List[str]] = None
) -> Dict[str, Any]:
    """
    Store a memory with semantic indexing.

    Args:
        content: The information to remember
        category: Category for organization (e.g., "auth", "database")
        user_id: User identifier for scoping memories
        tags: Optional tags for filtering

    Returns:
        Memory ID and confirmation
    """
    metadata = {
        "category": category,
        "tags": tags or [],
    }

    result = memory.add(
        content,
        user_id=user_id,
        metadata=metadata
    )

    return {
        "success": True,
        "memory_id": result.get("id"),
        "message": f"Remembered: {content[:50]}..."
    }


@server.tool("semantic_search")
async def semantic_search(
    query: str,
    user_id: str,
    limit: int = 5,
    category: Optional[str] = None
) -> List[Dict[str, Any]]:
    """
    Search memories by semantic similarity.

    Args:
        query: Natural language query
        user_id: User identifier
        limit: Maximum results to return
        category: Optional category filter

    Returns:
        List of matching memories with similarity scores
    """
    filters = {}
    if category:
        filters["category"] = category

    results = memory.search(
        query,
        user_id=user_id,
        limit=limit,
        filters=filters if filters else None
    )

    return [
        {
            "id": r.get("id"),
            "content": r.get("memory"),
            "score": r.get("score", 0),
            "category": r.get("metadata", {}).get("category"),
            "tags": r.get("metadata", {}).get("tags", []),
        }
        for r in results
    ]


@server.tool("semantic_forget")
async def semantic_forget(memory_id: str) -> Dict[str, Any]:
    """
    Remove a specific memory by ID.

    Args:
        memory_id: The ID of the memory to remove

    Returns:
        Confirmation of deletion
    """
    memory.delete(memory_id)
    return {
        "success": True,
        "message": f"Forgotten memory {memory_id}"
    }


@server.tool("list_memories")
async def list_memories(
    user_id: str,
    category: Optional[str] = None,
    limit: int = 20
) -> List[Dict[str, Any]]:
    """
    List all memories for a user.

    Args:
        user_id: User identifier
        category: Optional category filter
        limit: Maximum results

    Returns:
        List of memories
    """
    all_memories = memory.get_all(user_id=user_id, limit=limit)

    if category:
        all_memories = [
            m for m in all_memories
            if m.get("metadata", {}).get("category") == category
        ]

    return all_memories


def main():
    """Run the MCP server."""
    import sys
    from mcp.server.stdio import stdio_server

    async def run():
        async with stdio_server() as (read_stream, write_stream):
            await server.run(
                read_stream,
                write_stream,
                server.create_initialization_options()
            )

    asyncio.run(run())


if __name__ == "__main__":
    main()
```

### 2.3 Build Instructions

```bash
# Navigate to the new package directory
cd C:\Users\Admin\Downloads\projects\goose
mkdir goose-mem0-mcp
cd goose-mem0-mcp

# Create package structure
mkdir -p src/goose_mem0_mcp

# Create files (as shown above)

# Install in development mode
pip install -e .

# Or use uv for faster installation
uv pip install -e .
```

### 2.4 Test Instructions

#### Local Testing

```bash
# Start Qdrant (required for vector storage)
docker run -p 6333:6333 qdrant/qdrant

# Test the MCP server directly
python -m goose_mem0_mcp.server

# In another terminal, send test commands via stdin
echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "semantic_remember", "arguments": {"content": "This project uses JWT auth", "category": "auth", "user_id": "test"}}, "id": 1}'
```

#### Integration with Goose

```bash
# Add to extensions.yaml
cat >> ~/.config/goose/extensions.yaml << 'EOF'
extensions:
  - name: mem0
    type: stdio
    cmd: goose-mem0-mcp
    env:
      QDRANT_HOST: localhost
      QDRANT_PORT: "6333"
EOF

# Test with goose
goose "Remember that this project uses JWT tokens with RS256"

# Close session, reopen
goose "How do we handle authentication in this project?"
# Should recall JWT information!
```

#### Visual Verification

```bash
# Run goose with memory demonstration
goose --demo-memory

# User sees:
# 1. "Storing memory: JWT tokens with RS256..."
# 2. Session ends
# 3. New session starts
# 4. "Recalling: Based on my memory, this project uses JWT..."
```

### 2.5 Verification Criteria

- [ ] `pip install -e .` succeeds in goose-mem0-mcp
- [ ] Qdrant container starts and accepts connections
- [ ] `semantic_remember` stores memories
- [ ] `semantic_search` retrieves relevant memories
- [ ] Memories persist across goose sessions
- [ ] Memory recall works without explicit prompting

---

## Feature 3: Real-Time Playwright Visual Testing

### 3.1 What We're Building

Visual test execution where users watch the agent interact with web UIs.

### 3.2 Configuration

#### `extensions.yaml` - Visual Testing Config

```yaml
extensions:
  - name: playwright-visual
    type: stdio
    cmd: npx
    args:
      - "@anthropic/playwright-mcp"
    env:
      # Enable visible browser
      PLAYWRIGHT_HEADLESS: "false"
      # Slow down for observation
      PLAYWRIGHT_SLOW_MO: "500"
      # Standard viewport
      PLAYWRIGHT_VIEWPORT_WIDTH: "1280"
      PLAYWRIGHT_VIEWPORT_HEIGHT: "720"
```

### 3.3 Usage

```bash
# Run goose with visual testing
goose --visual "Test the login flow on localhost:3000"

# What the user sees:
#
# TERMINAL                          BROWSER WINDOW
# ─────────────────────────────────────────────────────────
# [StateGraph] CODE node            (Browser opens)
# Writing login test...
#
# [StateGraph] TEST node            🔴 LIVE
# 🎬 WATCH: Navigating to           ┌─────────────────────┐
# localhost:3000                    │   Login Page         │
#                                   │                      │
# 🎬 WATCH: Clicking #login-btn     │ [  Login  ] ← click │
#                                   │                      │
# 🎬 WATCH: Typing "testuser"       │ User: [testuser___] │
#                                   │ Pass: [___________] │
#                                   │                      │
# 📸 Screenshot captured            └─────────────────────┘
#
# ❌ TEST FAILED
# Expected: /dashboard
# Got: /login?error=invalid
#
# [StateGraph] FIX node
# Analyzing failure...
```

### 3.4 Test Instructions

```bash
# 1. Start a sample web application
cd sample-app
npm install
npm start &
# App running at http://localhost:3000

# 2. Run goose with visual testing
goose --visual "Test the user registration flow"

# 3. Watch the browser window
# - Browser opens automatically
# - Agent navigates to localhost:3000
# - Agent clicks, types, submits forms
# - User observes in real-time
# - Test results displayed with screenshots

# 4. Verify iteration
# If test fails, agent should:
# - Identify failure reason
# - Modify code
# - Re-run test (visible again)
# - Continue until success
```

### 3.5 Verification Criteria

- [ ] Browser window opens when `--visual` flag used
- [ ] User can see all agent actions in real-time
- [ ] Slow-mo makes actions observable (not instant)
- [ ] Screenshots captured at key points
- [ ] Failed tests show error state in browser
- [ ] Re-runs are also visible to user

---

## Feature 4: Test Framework Integration

### 4.1 What We're Building

Structured test output parsing for precise fix targeting.

### 4.2 Files to Create

#### `crates/goose-mcp/src/testing/mod.rs`

```rust
//! Test framework integration with structured output parsing

pub mod parsers;

use serde::{Deserialize, Serialize};

/// Unified test result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub stack_trace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestStatus {
    Pass,
    Fail,
    Skip,
    Error,
}

/// Parse test output into structured results
pub trait TestParser: Send + Sync {
    fn parse(&self, output: &str) -> Vec<TestResult>;
    fn framework_name(&self) -> &str;
}

/// Get parser for a test framework
pub fn get_parser(framework: &str) -> Option<Box<dyn TestParser>> {
    match framework.to_lowercase().as_str() {
        "pytest" => Some(Box::new(parsers::PytestParser)),
        "jest" => Some(Box::new(parsers::JestParser)),
        "cargo" => Some(Box::new(parsers::CargoParser)),
        "go" | "gotest" => Some(Box::new(parsers::GoTestParser)),
        _ => None,
    }
}
```

#### `crates/goose-mcp/src/testing/parsers/pytest.rs`

```rust
//! Pytest JSON output parser

use super::{TestParser, TestResult, TestStatus};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PytestReport {
    tests: Vec<PytestTest>,
}

#[derive(Debug, Deserialize)]
struct PytestTest {
    nodeid: String,
    outcome: String,
    duration: f64,
    #[serde(default)]
    longrepr: Option<String>,
    #[serde(default)]
    location: Option<(String, u32, String)>,
}

pub struct PytestParser;

impl TestParser for PytestParser {
    fn parse(&self, output: &str) -> Vec<TestResult> {
        let report: PytestReport = match serde_json::from_str(output) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        report.tests.iter().map(|t| {
            let status = match t.outcome.as_str() {
                "passed" => TestStatus::Pass,
                "failed" => TestStatus::Fail,
                "skipped" => TestStatus::Skip,
                _ => TestStatus::Error,
            };

            let (file, line) = t.location
                .as_ref()
                .map(|(f, l, _)| (Some(f.clone()), Some(*l)))
                .unwrap_or((None, None));

            // Parse assertion message from longrepr
            let (message, expected, actual) = parse_assertion(&t.longrepr);

            TestResult {
                name: t.nodeid.clone(),
                status,
                duration_ms: (t.duration * 1000.0) as u64,
                file,
                line,
                message,
                expected,
                actual,
                stack_trace: t.longrepr.clone(),
            }
        }).collect()
    }

    fn framework_name(&self) -> &str {
        "pytest"
    }
}

fn parse_assertion(longrepr: &Option<String>) -> (Option<String>, Option<String>, Option<String>) {
    // Parse pytest assertion messages like:
    // "AssertionError: assert 401 == 200"
    // Extract expected (200) and actual (401)

    let Some(repr) = longrepr else {
        return (None, None, None);
    };

    // Look for common assertion patterns
    if let Some(caps) = regex::Regex::new(r"assert (\S+) == (\S+)")
        .ok()
        .and_then(|re| re.captures(repr))
    {
        return (
            Some(format!("Assertion failed: {} != {}", &caps[1], &caps[2])),
            Some(caps[2].to_string()),
            Some(caps[1].to_string()),
        );
    }

    (Some(repr.lines().next().unwrap_or("").to_string()), None, None)
}
```

### 4.3 Build Instructions

```bash
# Build goose-mcp with testing module
cargo build -p goose-mcp

# Run tests
cargo test -p goose-mcp testing
```

### 4.4 Test Instructions

```bash
# Create a sample pytest project with failing test
mkdir /tmp/test-project
cd /tmp/test-project

cat > test_auth.py << 'EOF'
def test_login_returns_200():
    # Intentionally failing test
    response_code = 401
    assert response_code == 200, "Login should return 200"

def test_logout_clears_session():
    session = {"user": "test"}
    # Forgot to clear session
    assert session == {}, "Session should be empty"
EOF

# Run pytest with JSON output
pytest --json-report --json-report-file=results.json

# Test the parser
cargo run -p goose-mcp --example parse_pytest results.json

# Expected output:
# TestResult {
#     name: "test_auth.py::test_login_returns_200",
#     status: Fail,
#     file: Some("test_auth.py"),
#     line: Some(3),
#     message: Some("Assertion failed: 401 != 200"),
#     expected: Some("200"),
#     actual: Some("401"),
# }
```

### 4.5 Verification Criteria

- [ ] Pytest JSON parser extracts all fields
- [ ] Jest JSON parser works correctly
- [ ] Cargo test JSON parser works correctly
- [ ] Go test JSON parser works correctly
- [ ] Expected/actual values extracted from assertions
- [ ] File and line numbers accurate
- [ ] Stack traces preserved for debugging

---

## Feature 5: Multi-Agent Collaboration

### 5.1 What We're Building

Agent registry for spawning specialist agents that collaborate.

### 5.2 Files to Create

#### `crates/goose/src/agents/agent_registry.rs`

```rust
//! Multi-agent registry for specialist collaboration

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use serde::{Deserialize, Serialize};

/// Message passed between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub from: String,
    pub to: Option<String>,  // None = broadcast
    pub content: String,
    pub message_type: MessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Query,
    Response,
    Finding,
    Consensus,
}

/// Role of a specialist agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentRole {
    Security,
    Performance,
    Test,
    Reviewer,
    Custom(String),
}

/// Handle to a running agent
pub struct AgentHandle {
    pub id: String,
    pub role: AgentRole,
    pub tx: mpsc::Sender<AgentMessage>,
}

/// Result of consensus gathering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    pub approved: bool,
    pub votes: HashMap<String, bool>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub agent: String,
    pub severity: Severity,
    pub message: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

/// Registry for managing multiple agents
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<String, AgentHandle>>>,
    message_bus: broadcast::Sender<AgentMessage>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            message_bus: tx,
        }
    }

    /// Spawn a specialist agent
    pub async fn spawn_specialist(
        &self,
        role: AgentRole,
        instructions: &str,
    ) -> Result<AgentHandle, anyhow::Error> {
        let id = format!("{:?}_{}", role, uuid::Uuid::new_v4());
        let (tx, mut rx) = mpsc::channel(32);

        // Create agent handle
        let handle = AgentHandle {
            id: id.clone(),
            role: role.clone(),
            tx,
        };

        // Store in registry
        self.agents.write().await.insert(id.clone(), handle);

        // Subscribe to message bus
        let mut bus_rx = self.message_bus.subscribe();

        // Spawn agent task
        let agent_id = id.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle direct messages
                    Some(msg) = rx.recv() => {
                        println!("[{}] Received: {:?}", agent_id, msg);
                        // Process message...
                    }
                    // Handle broadcast messages
                    Ok(msg) = bus_rx.recv() => {
                        if msg.to.is_none() || msg.to.as_ref() == Some(&agent_id) {
                            println!("[{}] Broadcast: {:?}", agent_id, msg);
                            // Process message...
                        }
                    }
                }
            }
        });

        Ok(AgentHandle {
            id,
            role,
            tx: mpsc::channel(32).0,
        })
    }

    /// Broadcast message to all agents
    pub async fn broadcast(&self, message: AgentMessage) {
        let _ = self.message_bus.send(message);
    }

    /// Wait for consensus from specified agents
    pub async fn wait_for_consensus(
        &self,
        agent_ids: &[&str],
        query: &str,
    ) -> ConsensusResult {
        // Send consensus request
        self.broadcast(AgentMessage {
            from: "orchestrator".to_string(),
            to: None,
            content: query.to_string(),
            message_type: MessageType::Consensus,
        }).await;

        // Collect responses (simplified)
        let mut votes = HashMap::new();
        let mut findings = Vec::new();

        // In real implementation, wait for all agents to respond
        for id in agent_ids {
            votes.insert(id.to_string(), true);  // Placeholder
        }

        let approved = votes.values().all(|&v| v);

        ConsensusResult {
            approved,
            votes,
            findings,
        }
    }
}
```

### 5.3 Build Instructions

```bash
# Build with agent registry
cargo build -p goose

# Run tests
cargo test -p goose agent_registry
```

### 5.4 Test Instructions

```bash
# Test multi-agent collaboration
goose "Review this PR with security and performance specialists"

# User sees:
# [Orchestrator] Spawning Security agent...
# [Orchestrator] Spawning Performance agent...
# [Orchestrator] Spawning Test agent...
#
# [Security] Analyzing code for vulnerabilities...
# [Security] Finding: SQL injection risk in line 47
#
# [Performance] Analyzing code for bottlenecks...
# [Performance] Finding: N+1 query in UserService
#
# [Test] Checking test coverage...
# [Test] Warning: Missing test for error handling
#
# [Orchestrator] Gathering consensus...
# [Orchestrator] Result: BLOCK (2 critical findings)
#
# Summary:
# 🔴 CRITICAL: SQL injection (Security)
# 🔴 CRITICAL: N+1 query (Performance)
# 🟡 WARNING: Missing tests (Test)
#
# Recommendation: Fix critical issues before merge
```

### 5.5 Verification Criteria

- [ ] `spawn_specialist` creates new agent
- [ ] Agents receive broadcast messages
- [ ] Agents can send findings to orchestrator
- [ ] `wait_for_consensus` aggregates responses
- [ ] Critical findings block approval
- [ ] Findings include severity and location

---

## Summary: Build & Test Commands

### Quick Reference

```bash
# Full build
cargo build --workspace

# Run all tests
cargo test --workspace

# Build specific features
cargo build -p goose -F state_graph
cargo build -p goose-mcp -F testing

# Run visual tests
goose --visual "test task"

# Run with memory demo
goose --demo-memory

# Run with multi-agent
goose --specialists "security,performance" "review this code"
```

### Test Matrix

| Feature | Unit Test | Integration Test | Visual Test |
|---------|-----------|------------------|-------------|
| StateGraph | `cargo test state_graph` | `cargo test -p goose-test` | `goose --visual` |
| Mem0 Memory | `pytest goose-mem0-mcp` | `goose --demo-memory` | N/A |
| Playwright | N/A | N/A | `goose --visual` |
| Test Parsing | `cargo test testing` | `cargo test -p goose-test` | N/A |
| Multi-Agent | `cargo test agent_registry` | `goose --specialists` | N/A |

---

## Next Steps

After implementing all features:

1. Run full test suite: `cargo test --workspace`
2. Run visual demonstration: `goose --visual --demo`
3. Run benchmarks: `cargo run -p goose-bench`
4. Create release: `cargo build --release`
