---
title: "代码库分析"
description: "介绍如何用语义分析和调用图理解代码库。"
sidebar_position: 111
---

# 代码库分析

[Developer 扩展](/zh-CN/docs/mcp/developer-mcp) 内置了一个 `analyze` 工具，可以帮助你理解代码结构、追踪符号使用情况，并查看跨文件调用图。只要启用了 Developer 扩展，这个工具就会自动可用，并支持[多种编程语言](https://github.com/block/goose/blob/main/crates/goose-mcp/src/developer/analyze/languages/mod.rs)。

<details>
<summary>示例分析：跨文件追踪某个函数</summary>

**Prompt：**

"Show me the call graph for the authenticate function"

**输出：**

:::info Desktop
I'll analyze how the `authenticate` function is used across your codebase.

Based on my analysis, here's how the `authenticate` function is used in your codebase:

#### Function Definition
The `authenticate` function is defined in `auth.py` at line 15. It takes username and password parameters and handles user authentication.

#### Incoming Calls (Functions that call authenticate)
- **login_handler()** in `api.py:25` - The API endpoint that processes login requests
- **admin_login()** in `admin.py:102` - Special authentication flow for admin users
- **test_auth()** in `tests/test_auth.py:45` - Test case for authentication

#### Outgoing Calls (Functions that authenticate calls)
- **validate_credentials()** in `db.py:102` - Checks credentials against the database
- **log_attempt()** in `logger.py:45` - Records authentication attempts
- **check_rate_limit()** in `security.py:78` - Prevents brute force attacks

This shows that `authenticate` is a central function in your authentication flow, called by multiple entry points and depending on database validation, logging, and security checks.
:::

</details>

## 分析模式

`analyze` 工具有三种主要工作模式：Structure、Semantic 和 Focus。具体使用哪种模式，取决于你分析的是目录、文件还是符号。你既可以直接自然语言提问，也可以用带[参数](#通用参数)的命令显式调用。

### 理解项目结构

如果你分析的是一个目录，`analyze` 会给出代码库结构概览，帮助你理解项目组织方式、识别大型文件并查看一些整体指标。

**自然语言示例：**

- “Can you analyze the structure of my src/ directory?”
- “Give me an overview of this project's code structure”
- “What's the main entry point of this Python project?”

**直接命令：**

```bash
# 默认深度（3 层）查看目录概览
analyze path="src/"

# 只查看 2 层子目录
analyze path="." max_depth=2
```

### 查看单个文件

如果目标是某个文件，`analyze` 会给出它的语义结构，例如函数、类和 import，帮助你快速理解文件布局，或定位特定实现。

**自然语言示例：**

- “What functions are in main.py?”
- “Show me the structure of src/utils.py”

**直接命令：**

```bash
analyze path="main.py"
analyze path="src/utils.py"
```

### 跨文件追踪一个符号

如果你要关注特定函数、类或方法，可以把它作为 focus，查看它在哪里定义、被哪些地方调用，以及它自己又调用了什么。这个能力对重构和调试非常有用。

**自然语言示例：**

- “Trace the dependencies for the authenticate function”
- “Show me the call graph for UserClass”

**直接命令：**

```bash
analyze path="src/" focus="authenticate"
analyze path="." focus="UserClass" follow_depth=3
```

## 通用参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `path` | 无，必填 | 要分析的文件或目录路径，可以是绝对路径或相对路径 |
| `focus` | 无 | 要追踪的符号名称。若要跨文件追踪，`path` 必须是目录 |
| `follow_depth` | 2 | 从 focus 符号向外追踪多少层（0=只看定义位置，1=直接调用者/被调用者，2=再往上一层，以此类推） |
| `max_depth` | 3 | 当 `path` 是目录时，最多分析多少层子目录（0=不限制） |
| `force` | false | 强制返回完整结果；否则当输出超过 1000 行时，只会给出警告消息 |

## 最佳实践

### 处理超大输出

如果分析结果超过 1000 行，工具默认只返回警告而不是完整内容。你可以这样处理：

- **使用 `force=true`**：强制返回完整结果，但会显著占用对话上下文
- **缩小分析范围**：只分析某个子目录或单个文件
- **降低深度**：目录分析时使用 `max_depth=1` 或 `max_depth=2`
- **交给 [subagent](/zh-CN/docs/guides/subagents)**：例如“Use a subagent to analyze the entire src/ directory and summarize the main components”，这样可以避免主对话被大段分析结果填满

### 性能建议

- 先从小范围开始，例如单个文件或局部子目录，再逐步扩展到整个项目
- 对目录分析优先使用 `max_depth=1` 或 `max_depth=2`
- 使用 [`.gooseignore`](/zh-CN/docs/guides/using-gooseignore) 和 `.gitignore` 排除无关目录，例如 `node_modules/`、构建产物或敏感文件
