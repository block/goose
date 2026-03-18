---
title: "goose Desktop 的 macOS Sandbox"
description: "介绍 goose Desktop 在 macOS 上的可选 sandbox 机制。"
---

# goose Desktop 的 macOS Sandbox

goose Desktop 提供了一个可选的 macOS sandbox。当你需要更严格地控制 goose 能访问什么、连接哪里、是否符合安全策略时，可以启用它。典型用途包括：

- **限制文件系统访问**：阻止修改 SSH key、shell 配置文件以及 goose 自身配置
- **控制网络连接**：让所有流量都经过一个可过滤的本地代理，拦截未批准域名
- **防止安全绕过**：阻止隧道工具、raw socket 等绕过限制的方式
- **审计与策略执行**：记录所有网络活动，并满足合规需求

goose 本身仍会以完整工具权限运行，但 sandbox 会叠加两层保护：

- **文件访问控制**：利用 Apple 的 `sandbox-exec` 在系统层限制文件和网络访问
- **出站连接控制**：通过本地 egress proxy 过滤并记录对外连接

:::info macOS Requirement
这个 sandbox 依赖 `/usr/bin/sandbox-exec`，它只在 macOS 上可用，也被称为 Apple 的 seatbelt 技术。
:::

## 快速开始 {#quick-start}

启用 sandbox 的方式，是在终端里设置环境变量后再启动 goose Desktop：

```bash
export GOOSE_SANDBOX=true
open -a Goose
```

启用后，应用启动时会自动：

1. 生成一份 seatbelt sandbox profile
2. 在 localhost 上启动一个本地 HTTP CONNECT proxy
3. 在 `sandbox-exec` 内部启动 goose Desktop 的 `goosed` 后端，并强制所有流量经过代理

sandbox 会一直生效，直到你退出 goose Desktop。要关闭它，只需退出应用并正常重启，或者在启动时设置 `GOOSE_SANDBOX=false`。

## 配置 {#configuration}

所有配置都通过环境变量完成。默认值以安全优先为主，但你可以根据自己的安全要求进行调整。

### Core

| Variable | Default | Description |
|----------|---------|-------------|
| `GOOSE_SANDBOX` | `false` | 设置为 `true` 或 `1` 以启用 sandbox。启动方式见[快速开始](#quick-start)。 |

----

### 文件系统

[seatbelt sandbox profile](https://github.com/block/goose/blob/main/ui/desktop/src/sandbox/index.ts) 会阻止写入这些敏感文件：

- `~/.ssh/`：防止篡改 SSH key
- `~/.bashrc`、`~/.zshrc`、`~/.bash_profile`、`~/.zprofile`：防止注入 shell 配置
- `~/.config/goose/sandbox/`：保护 sandbox 配置不被 sandbox 内进程修改
- `~/.config/goose/config.yaml`：保护 goose 配置本身

#### 环境变量

| Variable | Default | Description |
|----------|---------|-------------|
| `GOOSE_SANDBOX_PROTECT_FILES` | `true` | 对上述敏感文件启用写保护，设为 `false` 可关闭 |

----

### 直接网络访问

seatbelt sandbox 会禁止所有直接网络访问，强制所有流量经过代理。唯一允许的连接包括：

- **Localhost**：让 `goosed` 能连接本地代理和自身服务端口
- **Unix sockets**：用于本地进程间通信
- **mDNSResponder**：用于 DNS 解析

:::info Not Configurable
只要启用 sandbox，这些限制就始终有效，不能单独关闭。
:::

----

### 进程限制

seatbelt sandbox 还会阻止可能绕过安全控制的工具和系统调用：

- **隧道工具**：阻止 `nc`、`ncat`、`netcat`、`socat`、`telnet`，防止绕过代理
- **Raw sockets**：阻止在 `AF_INET` / `AF_INET6` 上创建 `SOCK_RAW`，防止构造原始网络包
- **内核扩展**：拒绝 `system-kext-load`

#### 环境变量

| Variable | Default | Description |
|----------|---------|-------------|
| `GOOSE_SANDBOX_BLOCK_RAW_SOCKETS` | `true` | 阻止 `SOCK_RAW`，设为 `false` 可关闭 |
| `GOOSE_SANDBOX_BLOCK_TUNNELING` | `true` | 阻止 `nc` / `netcat` / `socat` / `telnet`，设为 `false` 可关闭 |

----

### 网络过滤

egress proxy 会检查并过滤所有对外连接。你可以通过 blocklist 文件和相关配置变量自定义过滤规则。

egress proxy 会按以下顺序检查连接：

1. **Loopback 检测**：阻止把代理当作转发器回连 localhost
2. **裸 IP 拦截**：阻止直接连接 IP 地址而非域名
3. **域名 blocklist**：`blocked.txt` 中列出的域名会被拒绝，包括其所有子域
4. **SSH / Git 主机限制**：SSH 端口（22、2222、7999）只允许访问已知 Git 主机

如果你需要基于 LaunchDarkly 的可选动态 egress 控制，请参见 [LaunchDarkly（可选）](#launchdarkly-optional)。

#### 环境变量

| Variable | Default | Description |
|----------|---------|-------------|
| `GOOSE_SANDBOX_ALLOW_IP` | `false` | 设为 `true` 以允许直接连接裸 IP 地址 |
| `GOOSE_SANDBOX_BLOCK_LOOPBACK` | `false` | 设为 `true` 以阻止经代理回连 loopback |
| `GOOSE_SANDBOX_ALLOW_SSH` | `true` | 设为 `false` 以阻止所有 SSH 流量 |
| `GOOSE_SANDBOX_GIT_HOSTS` | 内置列表 | 允许的 SSH Git 主机列表，逗号分隔，例如 `github.com,gitlab.com` |
| `GOOSE_SANDBOX_SSH_ALL_HOSTS` | `false` | 设为 `true` 以允许 SSH 连接任意主机，而不限于 Git 主机 |

#### 管理域名 Blocklist

文件 `~/.config/goose/sandbox/blocked.txt` 控制代理应拦截哪些域名。第一次运行时，它会根据内置模板自动创建。

```text
# 每行一个域名。子域会自动被一并拦截。
# 以 # 开头的行是注释。
evil.com          # 拦截 evil.com 和 *.evil.com
pastebin.com
transfer.sh
webhook.site
```

:::tip 热更新
修改 `blocked.txt` 会立即生效。代理会通过 `fs.watch` 监听这个文件并自动重载，无需重启。
:::

#### 通过 SSH 使用 Git

SSH Git 操作（例如 `git clone git@github.com:...`）在 sandbox 中仍然可以工作，因为 goose 会附带一个 `connect-proxy.pl` 脚本，作为 SSH 的 `ProxyCommand`。这样 SSH 连接仍会经过 egress proxy，并继续套用相同的 allowlist 规则。

默认情况下，SSH 只允许连接到常见的 Git 托管域名，例如 GitHub、GitLab 和 Bitbucket。你可以这样调整：

```bash
# 添加自定义 Git 主机
export GOOSE_SANDBOX_GIT_HOSTS="github.com,gitlab.com,your-gitea.internal.com"

# 或允许 SSH 访问任意主机
export GOOSE_SANDBOX_SSH_ALL_HOSTS=true
```

----

### LaunchDarkly（可选） {#launchdarkly-optional}

对于企业环境，LaunchDarkly 可以提供可选的动态 egress 控制。如果没有配置，sandbox 会退回使用本地 `blocked.txt` blocklist。

#### 环境变量

| Variable | Default | Description |
|----------|---------|-------------|
| `LAUNCHDARKLY_CLIENT_ID` | — | 用于启用动态 egress 控制的 LaunchDarkly client SDK key |
| `GOOSE_SANDBOX_LD_FAILOVER` | — | LaunchDarkly 不可达时的回退模式：`allow`、`deny` 或 `blocklist` |

## 配置示例

### 最大安全模式

```bash
export GOOSE_SANDBOX=true
# 保持默认值即可启用所有保护
```

### 允许连接裸 IP（例如内部 API）

```bash
export GOOSE_SANDBOX=true
export GOOSE_SANDBOX_ALLOW_IP=true
```

### 完全禁用 SSH

```bash
export GOOSE_SANDBOX=true
export GOOSE_SANDBOX_ALLOW_SSH=false
```

### 宽松模式（限制更少）

```bash
export GOOSE_SANDBOX=true
export GOOSE_SANDBOX_PROTECT_FILES=false
export GOOSE_SANDBOX_BLOCK_RAW_SOCKETS=false
export GOOSE_SANDBOX_BLOCK_TUNNELING=false
export GOOSE_SANDBOX_ALLOW_IP=true
export GOOSE_SANDBOX_SSH_ALL_HOSTS=true
```

### 配合 LaunchDarkly 做 egress 控制

```bash
export GOOSE_SANDBOX=true
export LAUNCHDARKLY_CLIENT_ID=sdk-your-key-here
export GOOSE_SANDBOX_LD_FAILOVER=blocklist
```

## 故障排查

- **报错："GOOSE_SANDBOX=true but sandbox-exec is not available (macOS only)"**  
  说明你当前不在 macOS 上，或者系统里没有 `/usr/bin/sandbox-exec`。这个 sandbox 只支持 macOS。

- **扩展或工具无法访问网络**  
  检查目标域名是否在 `~/.config/goose/sandbox/blocked.txt` 中；如果目标是裸 IP 地址，则可能需要启用 `GOOSE_SANDBOX_ALLOW_IP=true`。

- **通过 SSH 执行 `git clone` 失败**  
  目标主机可能不在默认 Git 主机 allowlist 中。你可以通过 `GOOSE_SANDBOX_GIT_HOSTS=your-host.com` 添加，或者设置 `GOOSE_SANDBOX_SSH_ALL_HOSTS=true`。

- **想查看代理到底拦截了什么？**  
  查看[Desktop 应用日志](/zh-CN/docs/guides/logs#desktop-application-log)。被拦截的连接会以 `[sandbox-proxy]` 前缀记录，并附带拦截原因。
