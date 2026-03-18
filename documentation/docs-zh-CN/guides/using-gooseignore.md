---
title: "阻止 goose 访问文件"
description: "介绍如何使用 `.gooseignore` 控制 goose 的文件访问范围。"
sidebar_position: 80
---

# 阻止 goose 访问文件

`.gooseignore` 是一个文本文件，用来定义 goose 不会访问的文件和目录匹配规则。这意味着当你使用 Developer 扩展的工具时，goose 无法读取、修改、删除这些文件，也不能在这些文件上运行 shell 命令。

:::info Developer extension only
`.gooseignore` 目前只影响 [Developer](/zh-CN/docs/mcp/developer-mcp) 扩展中的工具。其它扩展不会受到这些规则限制。
:::

本指南会带你了解如何使用 `.gooseignore`，阻止 goose 修改特定文件和目录。

## 创建 `.gooseignore`

goose 支持两类 `.gooseignore` 文件：

- **全局 ignore 文件**：在 `~/.config/goose` 下创建 `.gooseignore`。这些限制会对你所有 goose 会话生效，无论当前目录是什么。
- **本地 ignore 文件**：在你希望应用规则的项目根目录创建 `.gooseignore`。这些限制只会在该目录内工作时生效。

:::tip
你可以同时使用全局和本地 `.gooseignore`。当两者都存在时，goose 会同时应用两边的规则；本地规则还可以通过否定模式覆盖全局规则。
:::

## `.gooseignore` 示例

在 `.gooseignore` 文件中，你可以写入想要忽略的文件匹配模式。下面是一些常见例子：

```plaintext
# 按文件名忽略特定文件
settings.json         # 只忽略名为 "settings.json" 的文件

# 按扩展名忽略文件
*.pdf                # 忽略所有 PDF 文件
*.config             # 忽略所有以 .config 结尾的文件

# 忽略目录及其所有内容
backup/              # 忽略 "backup" 目录中的所有内容
downloads/           # 忽略 "downloads" 目录中的所有内容

# 忽略任意目录下同名文件
**/credentials.json  # 忽略任意目录中的 credentials.json
```

## 否定模式

使用 `!` 前缀可以把某些文件从忽略规则中排除出来。这样你就能先用较宽的规则整体忽略，再为特定文件留例外。

在每个 `.gooseignore` 文件里，规则是**从上到下依次处理**的，因此后面的规则可以覆盖前面的规则。否定模式也可以跨文件生效：例如，你可以在本地 `.gooseignore` 中使用否定规则，重新允许那些被全局 `.gooseignore` 屏蔽的文件。

```plaintext
# 忽略所有环境变量文件
**/.env*

# 但允许示例文件
!.env.example

# 忽略所有日志文件
*.log

# 但允许 error 日志
!error.log

# 忽略 config 目录中的所有 JSON 文件
config/*.json

# 但允许模板文件
!config/template.json
```

:::tip Pattern Order Matters
否定模式必须写在它要覆盖的规则之后。`!` 只是把前面已经被忽略的文件重新纳入可访问范围。
:::

## Ignore 文件类型与优先级

goose 会同时读取全局和本地 `.gooseignore` 文件，并按照优先级处理。后出现的规则可以覆盖更早的规则。

### 当存在 ignore 文件时

当 `.gooseignore` 文件存在时，规则按下面顺序应用：

1. **全局 `.gooseignore`**（先应用）
   - 位置：`~/.config/goose/.gooseignore`
   - 影响你机器上的所有项目

2. **本地 `.gooseignore`**（后应用，可覆盖全局）
   - 位置：当前工作目录，也就是你希望这些规则生效的项目根目录
   - 用于项目级规则，可覆盖全局模式

```text
~/.config/goose/
└── .gooseignore      ← 先应用全局规则

Project/
├── .gooseignore      ← 后应用本地规则（可覆盖全局）
└── src/
```

因为规则是按顺序处理的，你可以在本地 `.gooseignore` 里使用否定模式，让某些原本被全局规则阻止访问的文件重新开放。

**示例：在某个项目里覆盖全局限制**

```plaintext
# ~/.config/goose/.gooseignore（全局）
**/.env*              # 在所有地方禁止访问 .env 文件

# your-project/.gooseignore（本地）
!.env.example         # 只在这个项目里允许访问 .env.example
```

在这个例子中，`.env` 和 `.env.local` 依旧会被阻止，但 `.env.example` 会在该项目中可访问。

### 默认模式（没有 ignore 文件时）

如果你没有创建任何 `.gooseignore` 文件，无论是全局还是本地，goose 会默认保护这些敏感文件：

```plaintext
**/.env
**/.env.*
**/secrets.*
```

:::info
这些默认规则只会在**完全没有** `.gooseignore` 文件时生效。一旦你创建了全局或本地 `.gooseignore`，如果还希望保留这些保护，就需要把对应规则自己写进去。
:::

## 常见使用场景

`.gooseignore` 常见于这些场景：

- **生成文件**：避免 goose 修改自动生成代码或构建产物
- **第三方代码**：防止 goose 改动外部库或依赖
- **关键配置**：保护重要配置文件不被误改
- **版本控制目录**：防止改动 `.git` 等版本控制文件
- **自定义限制**：按你的项目边界定义 goose 不应访问的文件范围
