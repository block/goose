---
title: "扩展设计"
description: "介绍 goose 扩展框架的核心概念、架构和最佳实践。"
sidebar_position: 2
---

# 扩展设计

这份文档介绍 goose 中[扩展框架](/zh-CN/docs/getting-started/using-extensions)的设计与实现方式。扩展框架让 AI agent 能通过统一的工具接口与不同扩展交互。

## 核心概念

### Extension

Extension 表示任何可以被 AI agent 操作的组件。扩展通过 Tools 暴露能力，并维护自己的状态。核心接口由 `Extension` trait 定义：

```rust
#[async_trait]
pub trait Extension: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn instructions(&self) -> &str;
    fn tools(&self) -> &[Tool];
    async fn status(&self) -> AnyhowResult<HashMap<String, Value>>;
    async fn call_tool(&self, tool_name: &str, parameters: HashMap<String, Value>) -> ToolResult<Value>;
}
```

### Tools

Tool 是扩展向 agent 提供能力的主要方式。每个工具通常包含：

- 名称
- 描述
- 参数列表
- 实际执行功能的实现

一个工具需要接收 `Value`，并返回异步的 `AgentResult<Value>`，这样才能与 agent 的 tool calling 框架兼容。

```rust
async fn echo(&self, params: Value) -> AgentResult<Value>
```

## 架构概览

扩展框架主要由这几部分组成：

1. **Extension Trait**：所有扩展都必须实现的核心接口
2. **Error Handling**：工具执行时使用的专用错误类型
3. **Proc Macros**：用于简化工具定义和注册的宏能力（文档原文中标注为尚未完全实现）

## 错误处理

扩展系统主要使用两类错误：

- `ErrorData`：专门描述工具执行错误
- `anyhow::Error`：用于扩展状态和其它通用操作

这种拆分让工具执行时能提供更精确的错误反馈，同时保留一般性操作所需的灵活性。

## 最佳实践

### 工具设计

1. **命名清晰**：使用动作导向的名称，例如 `create_user`，而不是 `user`
2. **参数明确**：每个参数都应有清楚的描述
3. **错误具体**：尽可能返回具体错误，因为这些错误会进一步成为模型可理解的提示
4. **状态显式**：清楚表达工具会如何修改状态

### 扩展实现

1. **状态封装**：把扩展状态保持为私有并受控
2. **错误传播**：工具执行中优先使用 `?` 与 `ErrorData` 传播错误
3. **状态清晰**：提供结构化、可读性高的状态输出
4. **文档完整**：为所有工具和副作用写清楚说明

### 示例实现

下面是一个简单扩展的完整示例：

```rust
use goose_macros::tool;

struct FileSystem {
    registry: ToolRegistry,
    root_path: PathBuf,
}

impl FileSystem {
    #[tool(
        name = "read_file",
        description = "Read contents of a file"
    )]
    async fn read_file(&self, path: String) -> ToolResult<Value> {
        let full_path = self.root_path.join(path);
        let content = tokio::fs::read_to_string(full_path)
            .await
            .map_err(|e| ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: Cow::from(e.to_string(),
                data: None,
            }))?;

        Ok(json!({ "content": content }))
    }
}

#[async_trait]
impl Extension for FileSystem {
    // ... implement trait methods ...
}
```

## 测试

扩展最好从多个层次进行测试：

1. 单个工具的单元测试
2. 扩展行为的集成测试
3. 工具不变量的 property 测试

示例测试：

```rust
#[tokio::test]
async fn test_echo_tool() {
    let extension = TestExtension::new();
    let result = extension.call_tool(
        "echo",
        hashmap!{ "message" => json!("hello") }
    ).await;

    assert_eq!(result.unwrap(), json!({ "response": "hello" }));
}
```
