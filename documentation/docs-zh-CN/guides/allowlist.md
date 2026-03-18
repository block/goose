---
title: "goose 扩展 Allowlist"
description: "介绍如何通过 allowlist 控制 goose 可用的扩展范围。"
sidebar_position: 90
---

# goose 扩展 Allowlist

goose 是一个可扩展框架，默认允许你安装任意 MCP server。不过在一些场景里，你可能希望更严格地限制哪些 MCP server 能作为扩展安装，例如企业内控环境。

本指南说明如何创建一份适用于 goose Desktop 和 CLI 的安全扩展 **allowlist**。启用 allowlist 后，管理员可以控制哪些 MCP server 可以安装为 goose 扩展。goose 只会安装名单中的扩展，并阻止其它扩展的安装。

## 工作原理

1. allowlist 是一个 YAML 文件，其中列出了允许安装的扩展命令。
2. goose 会从 `GOOSE_ALLOWLIST` 环境变量指定的 URL 拉取这份 allowlist。
3. goose 会在首次需要时获取 allowlist 并缓存；每次重启 goose 时都会重新拉取。
4. 当用户尝试安装扩展时，goose 会把 MCP server 的安装命令与 allowlist 进行比对。
5. 如果命令不在 allowlist 中，这次扩展安装就会被拒绝。

## 配置

### 1. 创建并部署 allowlist

allowlist 必须是一个 YAML 文件，结构如下：

```yaml
extensions:
  - id: extension-id-1
    command: command-name-1
  - id: extension-id-2
    command: command-name-2
  # ... more extensions
```

#### 示例

下面这个例子里，只允许安装 Slack、GitHub 和 Jira 这三个扩展：

```yaml
extensions:
  - id: slack
    command: uvx mcp_slack
  - id: github
    command: uvx mcp_github
  - id: jira
    command: uvx mcp_jira
```

创建完成后，你需要把这份 allowlist 部署到一个可访问的 URL。

### 2. 设置环境变量

创建一个名为 `GOOSE_ALLOWLIST` 的环境变量，并把值设置为你 YAML 文件的 URL：

```bash
export GOOSE_ALLOWLIST=https://example.com/goose-allowlist.yaml
```

你也可以把这条 `export` 写到 shell 配置文件里。在 macOS 上通常是 `~/.bashrc` 或 `~/.zshrc`。

:::info
如果没有设置这个环境变量，就不会应用 allowlist 限制；在没有限制的情况下，所有扩展都可以安装。
:::

## 最佳实践

为了更有效地使用精确匹配的 allowlist，建议：

1. **明确命令**：写入你真正允许的完整命令字符串。
2. **包含完整路径**：如果你只希望允许某个特定路径下的命令，请把完整路径写进 allowlist。
3. **定期审计**：经常检查 allowlist，确认里面只包含你打算允许的命令。
4. **使用 HTTPS**：通过 HTTPS 提供 allowlist，避免中间人攻击。
5. **限制编辑权限**：确保只有授权用户可以修改 allowlist。
6. **核验条目**：逐条确认 allowlist 中只包含可信命令。
7. **监控安装失败**：关注扩展安装时被拒绝的命令，这可能意味着有人在尝试滥用。

## 排障

如果扩展被意外拒绝，请依次检查：

1. `GOOSE_ALLOWLIST` 环境变量是否设置正确。
2. 服务器是否能访问 allowlist 文件。
3. allowlist 文件是否是格式正确的 YAML。
4. 查看 [goose 日志系统](./logs.md)，确认是否有拉取或解析 allowlist 相关错误。
5. 扩展安装命令是否与 allowlist 中的命令完全一致。
