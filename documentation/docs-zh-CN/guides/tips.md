---
title: "快速使用技巧"
sidebar_position: 30
sidebar_label: "快速技巧"
description: "总结与 goose 协作时的关键实践建议。"
---

### goose 会替你执行任务
goose 是一个 AI agent，这意味着你可以直接让它代你完成很多事情，例如打开应用、执行 shell 命令、自动化工作流、写代码、浏览网页等等。

### 用自然语言和 goose 交流
你不需要使用复杂语法或特殊格式。像平时和同事交流一样直接跟 goose 说话就行，哪怕带口语、缩写、please 或 thank you，它也能理解。

### 把 goose 扩展到任意应用
goose 的能力是可扩展的。作为一个 [MCP](https://modelcontextprotocol.io/) client，goose 可以通过 [extensions](/extensions) 连接你的应用和服务，从而在整个工作流中协同工作。

### 选择 goose 拥有多少控制权
你可以按需配置 goose 需要多少[监督](/zh-CN/docs/guides/goose-permissions)。既可以允许它完全自治，也可以要求它在执行动作前审批，或者纯粹只聊天不执行操作。

### 选对 LLM
你和 goose 的协作体验，很大程度上取决于你选择的 [LLM](/blog/2025/03/31/goose-benchmark)。模型负责规划，goose 负责执行。选模型时，建议重点看工具支持能力、擅长场景和成本。

### 保持会话短一些
LLM 都有上下文窗口限制，超过以后可能会忘掉前面内容。留意 token 使用情况，并在需要时[新开会话](/zh-CN/docs/guides/sessions/session-management)。

### 用 Quick Launcher 更快开始会话
按下 `Cmd+Option+Shift+G`（macOS）或 `Ctrl+Alt+Shift+G`（Windows/Linux），输入 prompt，就能立刻开始一个新会话。

### 关闭不必要的扩展和工具
开太多扩展会拖慢性能。只启用真正必要的[扩展和工具](/zh-CN/docs/guides/managing-tools/tool-permissions)，能提升工具选择准确度、节省上下文窗口，并减少 provider 工具上限压力。

:::tip 扩展很多时考虑 Code Mode
如果你启用了很多扩展，可以考虑打开 [Code Mode](/zh-CN/docs/guides/managing-tools/code-mode)。这是一种按需发现工具的替代式调用方式。
:::

### 把你的偏好教给 goose
你可以用 [`.goosehints` 或其他上下文文件](/zh-CN/docs/guides/context-engineering/using-goosehints)，以及 [skills](/zh-CN/docs/guides/context-engineering/using-skills) 来告诉 goose 你的长期项目偏好；对于希望它之后动态记住的内容，可以使用 [Memory extension](/zh-CN/docs/mcp/memory-mcp)。这两种方式都能帮助你节省上下文窗口，同时保留偏好信息。

### 保护敏感文件
goose 通常很积极地执行修改。如果你不希望它接触某些文件，可以创建一个 [.gooseignore](/zh-CN/docs/guides/using-gooseignore) 文件，把不允许它修改的路径列进去。

### 用好版本控制
尽早、频繁地提交代码改动。这样一旦出现意外变更，你能快速回滚。

### 控制 goose 可用的扩展范围
管理员可以使用 [allowlist](/zh-CN/docs/guides/allowlist) 把 goose 限制在经过批准的扩展集合中，以避免从未知 MCP server 安装风险扩展。

### 建立可复用模板
如果某次会话效果很好，可以把它整理成一个可复用的 [recipe](/zh-CN/docs/guides/recipes/session-recipes)，方便分享给别人或以后重复使用。

### 保持实验心态
第一次没做对很正常。不断迭代 prompt 和工具选择，本来就是使用 goose 的一部分。

### 自定义侧边栏
goose Desktop 支持你按自己的习惯来[自定义侧边栏](/zh-CN/docs/guides/desktop-navigation)。你可以调整位置、样式和显示项。

### 保持 goose 为最新版本
定期[更新 goose](/zh-CN/docs/guides/updating-goose)，可以及时获得新功能、bug 修复和性能改进。

### 用双模型节省成本
你可以使用 [lead/worker model](/zh-CN/docs/tutorials/lead-worker/)：前几轮让“lead”模型负责规划，后续再交给成本更低的“worker”模型执行。

### 让 Recipes 可以重复运行
写 [recipes](/zh-CN/docs/guides/recipes/session-recipes) 时，先检查当前状态再执行动作，这样就能安全地重复运行，避免报错或重复创建资源。

### 给 Recipes 加日志
在每个关键步骤里加上有意义的日志输出，后续排查或调试失败原因会容易很多。
