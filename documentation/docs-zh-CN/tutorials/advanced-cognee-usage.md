---
title: 使用 goose 进阶集成 Cognee
description: 使用 Cognee 知识图谱增强 goose 的记忆、自动化和代码分析能力
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# 使用 goose 进阶集成 Cognee

本教程聚焦于 Cognee 扩展的进阶使用方式，包括自动化记忆管理、知识图谱优化，以及不同的集成策略。

## 概览

基础的 [Cognee MCP 配置](../mcp/cognee-mcp.md) 只能帮你“接上” Cognee；而本教程更关注如何让 goose **主动** 使用知识图谱，并把它真正纳入你的工作流。

## 关键概念

### 知识图谱记忆
Cognee 会构建一个结构化知识图谱，它能够：
- 把对话、文档、图片和音频转写内容关联起来
- 支持 30 多种数据源
- 用动态关系映射替代传统 RAG
- 支持复杂的多跳推理

### 搜索类型
理解 Cognee 的搜索类型，是高效使用它的关键：

| 搜索类型 | 适用场景 | 说明 |
|-------------|----------|------|
| `SUMMARIES` | 摘要请求 | 高层概览 |
| `INSIGHTS` | 关系查询 | 实体之间的连接 |
| `CHUNKS` | 具体事实 | 原始文本片段 |
| `COMPLETION` | 解释型问题 | 由 LLM 生成的回答 |
| `GRAPH_COMPLETION` | 复杂关系 | 多跳推理 |
| `GRAPH_SUMMARY` | 简明回答 | 短而聚焦的结果 |
| `GRAPH_COMPLETION_COT` | 多跳问答 | 连续推理 |
| `GRAPH_CONTEXT_EXT` | 上下文扩展 | 扩展相关上下文 |
| `CODE` | 代码示例 | 编程相关查询 |

## 自动化策略

<Tabs>
<TabItem value="method1" label="方法 1（较慢）" default>

### 指令文件

如果你想在多个 session 中保持一致行为，可以使用 instruction files。这种方式 token 开销较低，但启动速度会更慢一些。

创建 `~/.config/goose/cognee-instructions.md`：

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

使用 instruction file 启动 goose：
```bash
goose run -i ~/.config/goose/cognee-instructions.md -s
```

</TabItem>
<TabItem value="method2" label="方法 2">

### `.goosehints` 文件

如果你更想要更快的启动速度，并愿意接受更高的 token 消耗，可以把下面内容加入 `.goosehints`：

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

### 策略 3：结合 Memory MCP

你还可以搭配 [Memory MCP 扩展](../mcp/memory-mcp.md)，形成混合策略：

1. 把 Cognee 的使用规则保存为 memories
2. 用 Memory MCP 触发 Cognee 搜索
3. 相比 `.goosehints`，token 占用更低
4. 相比纯 instruction file，更稳定

## 进阶工作流

### 开发工作流

对于软件开发项目，可以这样使用：

```bash
# 启动 goose 并接入 Cognee
goose session

# 在 goose 中分析代码库
> goose, please codify this repository and then help me understand the architecture
```

goose 会：
1. 对你的仓库执行 `cognee-mcp__codify`
2. 建立代码知识图谱
3. 基于这个图谱回答架构问题

### 研究工作流

适合处理论文和文档：

```bash
# 将研究文档写入图谱
> goose, please cognify the contents of these research papers: paper1.pdf, paper2.pdf, paper3.pdf

# 随后查询关系
> What are the connections between the methodologies in these papers?
```

### 个人助理工作流

适合个人效率场景：

```bash
# 记录偏好
> Remember that I prefer morning meetings, work best with 2-hour focused blocks, and need 15-minute breaks between calls

# 后续查询
> Based on my preferences, how should I structure tomorrow's schedule?
```

## 性能优化

### 服务端配置

为了获得更好的性能，建议把 Cognee 作为独立服务运行：

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

### 记忆管理

你还需要定期监控与维护知识图谱：

```bash
# 查看状态
> goose, what's the status of the cognify pipeline?

# 按需清理（如果必要）
> goose, can you help me identify outdated information in the knowledge graph?
```

## 故障排查

### 常见问题

1. **启动慢**：优先使用更轻量的 server 配置
2. **记忆不持久**：检查文件路径和权限
3. **搜索结果为空**：确认数据是否真的完成了 cognify
4. **token 消耗过高**：优先使用 instruction files，而不是把所有规则都放进 `.goosehints`

### 调试命令

```bash
# 查看 Cognee 日志
tail -f ~/.local/share/cognee/logs/cognee.log

# 测试 server 连通性
curl http://localhost:8000/health

# 验证知识图谱状态
# In goose session:
> goose, run cognify_status and codify_status
```

## 最佳实践

### 数据组织

1. **使用 nodesets** 对不同类型的信息分组：
   ```bash
   # 开发规则
   > goose, add these coding standards to the 'developer_rules' nodeset
   
   # 项目专属信息  
   > goose, cognify this project documentation with nodeset 'project_alpha'
   ```

2. **定期维护**：
   - 每月回顾和更新已存信息
   - 清理过期偏好和事实
   - 根据实际使用情况优化搜索 query

### 集成模式

1. **分层方案**：Memory MCP 和 Cognee 分别承担不同职责
2. **上下文切换**：为不同工作流准备不同 instruction files
3. **选择性自动化**：并不是每次交互都必须触发知识图谱查询

## 示例

### 代码评审助手

```bash
# Setup
> goose, codify this repository and remember that I prefer: functional programming patterns, comprehensive tests, and clear documentation

# Usage
> Review this pull request and check it against my coding preferences
```

### 会议助手

```bash
# Before meeting
> goose, cognify the agenda and participant backgrounds from these documents

# During/after meeting
> Based on the knowledge graph, what are the key action items and how do they relate to our previous discussions?
```

### 研究助手

```bash
# Literature review
> goose, cognify these 10 research papers and create a knowledge graph of the relationships between their methodologies

# Synthesis
> What are the emerging patterns in the research and what gaps exist?
```

这份进阶指南的目标，是帮助你把 Cognee 从“只是接上了”推进到“真正能成为 goose 长期记忆和自动化的一部分”。关键不在安装，而在于把检索、记忆更新与代码知识图谱真正融入你的日常工作流。
