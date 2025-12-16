---
title: Advanced Cognee Usage with goose
description: Advanced patterns for using Cognee knowledge graph with goose for enhanced memory and automation
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Advanced Cognee Usage with goose

This tutorial covers advanced usage patterns for the Cognee extension with goose, including automated memory management, knowledge graph optimization, and various integration strategies.

## Overview

While the basic [Cognee MCP setup](../mcp/cognee-mcp.md) gets you started, this tutorial explores how to make goose autonomously use the knowledge graph and optimize your workflow.

## Key Concepts

### Knowledge Graph Memory
Cognee creates a structured knowledge graph that:
- Interconnects conversations, documents, images, and audio transcriptions
- Supports over 30 data sources
- Replaces traditional RAG systems with dynamic relationship mapping
- Enables complex multi-hop reasoning

### Search Types
Understanding Cognee's search types is crucial for effective usage:

| Search Type | Use Case | Description |
|-------------|----------|-------------|
| `SUMMARIES` | Summary requests | High-level overviews |
| `INSIGHTS` | Relationship queries | Connections between entities |
| `CHUNKS` | Specific facts | Raw text segments |
| `COMPLETION` | Explanations | LLM-generated responses |
| `GRAPH_COMPLETION` | Complex relations | Multi-hop reasoning |
| `GRAPH_SUMMARY` | Concise answers | Brief, focused responses |
| `GRAPH_COMPLETION_COT` | Multi-hop Q&A | Chain-of-thought reasoning |
| `GRAPH_CONTEXT_EXT` | Context extension | Expanded context |
| `CODE` | Code examples | Programming-related queries |

## Automation Strategies

<Tabs>
<TabItem value="method1" label="Method 1 (Slow)" default>

### Instruction Files

Use instruction files for consistent behavior across sessions. This method uses fewer tokens but has slower startup.

Create `~/.config/goose/cognee-instructions.md`:

````markdown
You are an LLM agent with access to a Cognee knowledge graph for memory.

**IMPORTANT RULES:**
- Never call the `prune` command
- Always search memory before responding to user queries
- Automatically cognify new information you learn about the user

**Memory Workflow:**
1. **Before each response**: Search the knowledge graph
   - Map user request to appropriate search type:
     - Summary → SUMMARIES
     - Relationships → INSIGHTS  
     - Specific facts → CHUNKS
     - Explanations → COMPLETION
     - Complex relations → GRAPH_COMPLETION
     - Code examples → CODE
   
2. **Search command**:
   ```text
   cognee-mcp__search(\{
     search_query: "user prompt",
     search_type: "mapped type"
   \})
   ```

3. **Incorporate results** into your response

**Memory Updates:**
- When you learn new facts, preferences, or relationships about the user
- Call: `cognee-mcp__cognify(\{ data: "information" \})`
- Monitor with: `cognee-mcp__cognify_status()`

**Code Analysis:**
- When asked to analyze code repositories
- Use: `cognee-mcp__codify(\{ repo_path: "path" \})`
- Only process files returned by `rg --files`
````

Start goose with instructions:
```bash
goose run -i ~/.config/goose/cognee-instructions.md -s
```

</TabItem>
<TabItem value="method2" label="Method 2">

### goosehints File

For faster startup with higher token usage, add to your `.goosehints` file:

```text
COGNEE_MEMORY_SYSTEM:
You have access to a Cognee knowledge graph for persistent memory.

MEMORY_RETRIEVAL_PROTOCOL:
- Before responding, determine request type and map to search type
- Search types: SUMMARIES, INSIGHTS, CHUNKS, COMPLETION, GRAPH_COMPLETION, CODE
- Always call: cognee-mcp__search with search_query and search_type parameters
- Incorporate memory results into responses

MEMORY_STORAGE_PROTOCOL:
- Auto-cognify new user facts, preferences, relationships
- Call: cognee-mcp__cognify with data parameter
- Never use prune command

CODE_ANALYSIS_PROTOCOL:
- For repositories: cognee-mcp__codify with repo_path parameter
- Only process files from rg --files output
```

</TabItem>
</Tabs>

### Strategy 3: Memory MCP Integration

Combine with the [Memory MCP extension](../mcp/memory-mcp.md) for hybrid approach:

1. Store Cognee usage patterns as memories
2. Use Memory MCP to trigger Cognee searches
3. Lower token usage than goosehints
4. More reliable than pure instruction files

## Advanced Workflows

### Developer Workflow

For software development projects:

```bash
# Start goose with Cognee
goose session

# In goose, analyze your codebase
> goose, please codify this repository and then help me understand the architecture
```

goose will:
1. Run `cognee-mcp__codify` on your repository
2. Build a code knowledge graph
3. Answer architecture questions using the graph

### Research Workflow

For research and documentation:

```bash
# Cognify research documents
> goose, please cognify the contents of these research papers: paper1.pdf, paper2.pdf, paper3.pdf

# Later, query relationships
> What are the connections between the methodologies in these papers?
```

### Personal Assistant Workflow

For personal productivity:

```bash
# Store preferences
> Remember that I prefer morning meetings, work best with 2-hour focused blocks, and need 15-minute breaks between calls

# Query later
> Based on my preferences, how should I structure tomorrow's schedule?
```

## Performance Optimization

### Server Configuration

For optimal performance, run Cognee as a separate server:

```bash
# Create optimized startup script
cat > start-cognee-optimized.sh << 'EOF'
#!/bin/bash
set -e

# Performance settings
export DEBUG=false
export LOG_LEVEL=WARNING
export RATE_LIMIT_INTERVAL=30

# Model configuration
export LLM_API_KEY=${OPENAI_API_KEY}
export LLM_MODEL=openai/gpt-4o-mini  # Faster, cheaper model
export EMBEDDING_API_KEY=${OPENAI_API_KEY}
export EMBEDDING_MODEL=openai/text-embedding-3-small  # Faster embedding

# Server settings
export HOST=0.0.0.0
export PORT=8000

cd /path/to/cognee-mcp
uv run python src/server.py --transport sse
EOF

chmod +x start-cognee-optimized.sh
```

### Memory Management

Monitor and manage your knowledge graph:

```bash
# Check status
> goose, what's the status of the cognify pipeline?

# Selective pruning (if needed)
> goose, can you help me identify outdated information in the knowledge graph?
```

## Troubleshooting

### Common Issues

1. **Slow startup**: Use Method 2 (separate server) configuration
2. **Memory not persisting**: Check file permissions and paths
3. **Search returning empty results**: Ensure data was properly cognified
4. **High token usage**: Use instruction files instead of goosehints

### Debug Commands

```bash
# Check Cognee logs
tail -f ~/.local/share/cognee/logs/cognee.log

# Test server connection
curl http://localhost:8000/health

# Verify knowledge graph status
# In goose session:
> goose, run cognify_status and codify_status
```

## Best Practices

### Data Organization

1. **Use nodesets** for organizing different types of information:
   ```bash
   # Developer rules
   > goose, add these coding standards to the 'developer_rules' nodeset
   
   # Project-specific info  
   > goose, cognify this project documentation with nodeset 'project_alpha'
   ```

2. **Regular maintenance**:
   - Review and update stored information monthly
   - Remove outdated preferences and facts
   - Optimize search queries based on usage patterns

### Integration Patterns

1. **Layered approach**: Use both Memory MCP and Cognee for different purposes
2. **Context switching**: Different instruction files for different workflows
3. **Selective automation**: Not every interaction needs knowledge graph queries

## Examples

### Code Review Assistant

```bash
# Setup
> goose, codify this repository and remember that I prefer: functional programming patterns, comprehensive tests, and clear documentation

# Usage
> Review this pull request and check it against my coding preferences
```

### Meeting Assistant

```bash
# Before meeting
> goose, cognify the agenda and participant backgrounds from these documents

# During/after meeting
> Based on the knowledge graph, what are the key action items and how do they relate to our previous discussions?
```

### Research Assistant

```bash
# Literature review
> goose, cognify these 10 research papers and create a knowledge graph of the relationships between their methodologies

# Synthesis
> What are the emerging patterns in the research and what gaps exist?
```

This advanced usage guide should help you maximize the potential of Cognee with goose for sophisticated knowledge management and automation workflows.
