---
title: Known Issues
sidebar_label: Known Issues
description: goose 常见问题的系统化排查指南，包含逐步解决方案。
---

goose 和其他系统一样，偶尔也会遇到一些已知问题。这一页汇总了常见故障和对应的处理方式。

:::tip 没找到对应问题？
你可以到 [Discord 社区](https://discord.gg/goose-oss) 求助。为了更快获得帮助，建议先生成一份[诊断包](/zh-CN/docs/troubleshooting/diagnostics-and-reporting)。
:::

### goose 会修改文件

goose 会在完成任务的过程中主动编辑文件。为了避免覆盖你自己的改动，建议把你的个人改动先通过版本控制暂存或提交，把 goose 的改动单独保留到你完成 review 之后再处理。更稳妥的做法是把 goose 的修改拆成单独提交，便于回滚。

---

### 中断 goose

如果 goose 的方向跑偏，或者卡在错误路径上，可以直接[中断当前任务](/zh-CN/docs/guides/sessions/in-session-actions#interrupt-task)，然后补充新的说明或修正指令。

---

### 卡在循环里或没有响应 {#stuck-in-a-loop-or-unresponsive}

在长会话中，goose 偶尔可能出现“死循环”或者长时间没有响应。通常最简单的解决办法是结束当前会话并重新开始。

1. 按住 `Ctrl+C` 取消当前执行
2. 启动新会话：

   ```sh
   goose session
   ```

:::tip
如果任务很大或很复杂，拆成多个更小的会话通常更稳定。
:::

---

### 避免长时间不退出的命令

如果你在使用 goose CLI，并且项目里包含 Web 开发流程，像 `npm run dev`、`python -m http.server`、`webpack serve` 这类命令可能会一直运行，导致 goose 挂起等待。

你可以通过自定义 shell 行为来处理这些命令。详细说明见[环境变量指南](/zh-CN/docs/guides/environment-variables)。

---

### 上下文长度超限

当发送给 goose 的输入超过当前 LLM 模型可接受的 token 上限时，会出现这类错误。处理方式通常是把输入拆成更小的部分，或者把长期背景信息放进 [`.goosehints`](/zh-CN/docs/guides/context-engineering/using-goosehints)。在 goose Desktop 中，也可以结合[消息排队](/zh-CN/docs/guides/sessions/in-session-actions#queue-messages)逐步喂给模型。

---

### 使用 Ollama Provider

Ollama 使用本地模型，所以在 goose 中接入前，需要先[下载 Ollama 并启动一个本地模型](/zh-CN/docs/getting-started/providers)。如果模型还没拉取完成，常见报错如下：

> ExecutionError("error sending request for url (http://localhost:11434/v1/chat/completions)")

另外需要注意，DeepSeek 系列模型通常不支持 tool calling，所以如果你通过 Ollama 使用这类模型，需要先关闭所有 goose 扩展，相关说明见[使用扩展](/zh-CN/docs/getting-started/using-extensions)。在没有工具调用能力的情况下，goose 的自动化能力会明显受限。像 `qwen2.5` 这类支持 tool calling 的模型会更适合和 goose 搭配。

---

### 处理速率限制错误

当 goose 调用 LLM Provider 时，可能会遇到 `429 error`（超过速率限制）。更稳妥的做法是优先选择自带更好限流处理能力的 Provider。详细建议可查看[处理 LLM 速率限制](/zh-CN/docs/guides/handling-llm-rate-limits-with-goose)。

---

### Hermit 错误

如果你在 app 里安装扩展时看到 `hermit:fatal`，通常意味着本地 hermit 缓存需要清理。goose 会内置一份 hermit，用来确保 `npx` 和 `uvx` 可用且行为一致。如果你机器上存在旧版本缓存，可能需要手动删除。

在 macOS 上可以执行：

```sh
sudo rm -rf ~/Library/Caches/hermit
```

---

### API 错误

当 LLM Provider 的 API Token 配置不正确、额度用尽，或相关配置失效时，可能会看到类似下面的报错：

```sh
Traceback (most recent call last):
  File "/Users/admin/.local/pipx/venvs/goose-ai/lib/python3.13/site-packages/exchange/providers/utils.py",
line 30, in raise_for_status
    response.raise_for_status()
    ~~~~~~~~~~~~~~~~~~~~~~~~~^^
  File "/Users/admin/.local/pipx/venvs/goose-ai/lib/python3.13/site-packages/httpx/_models.py",
line 829, in raise_for_status
    raise HTTPStatusError(message, request=request, response=self)
httpx.HTTPStatusError: Client error '404 Not Found' for url
'https://api.openai.com/v1/chat/completions'
```

这类问题通常意味着额度耗尽，或者 API Key 无效。可以按下面顺序检查：

1. 登录对应 Provider 后台，确认额度是否足够
2. 重新配置 API Key：

   ```sh
   goose configure
   ```

完整 Provider 更新流程可参考[安装与配置指南](/zh-CN/docs/getting-started/installation)。

---

### GitHub Copilot Provider 配置问题

如果你在配置 GitHub Copilot 作为 Provider 时遇到报错，可以按下面几种常见场景排查。

#### Lead/Worker 模型导致的 OAuth 冲突

如果环境里已经配置了 [lead/worker model](/zh-CN/docs/tutorials/lead-worker)，在进行 GitHub Copilot OAuth 配置时，可能会看到：

```text
Failed to authenticate: Execution error: OAuth configuration not supported by this provider
```

处理方式：

1. 临时注释或删除主配置文件 `~/.config/goose/config.yaml` 中的 lead/worker 变量：

   ```yaml
   # GOOSE_LEAD_MODEL: your-model
   # GOOSE_WORKER_MODEL: your-model
   ```

2. 重新执行 `goose configure`
3. 完成 GitHub Copilot 的 OAuth 流程
4. 按需恢复 lead/worker 配置

#### 容器环境与 Keyring 问题

goose 会优先使用系统 keyring（例如 Linux 下通过 Secret Service/DBus）来安全存储 GitHub Copilot token。在容器或无桌面环境中，这些服务可能不可用，因此会导致 keyring 访问失败。

如果你在 Docker 或无 keyring 支持的 Linux 环境中运行 goose，goose 有时会自动回退到基于文件的密钥存储。若自动回退没有生效，并且你看到如下错误：

```text
Failed to save token: Failed to access keyring: <error message>
```

可以显式设置 `GOOSE_DISABLE_KEYRING`：

```bash
GOOSE_DISABLE_KEYRING=1 goose configure
```

如果你不希望把密钥写入 `secrets.yaml`，可以改用环境变量方式配置。

更多说明见下方的[Keychain / Keyring 错误](#keychain-keyring-errors)。

---

### 新 Recipe 警告

第一次在 goose Desktop 中执行某个 recipe 时，会出现 `New Recipe Warning` 对话框，让你先检查 recipe 的标题、描述和指令内容。如果你确认可信，可以点击 `Trust and Execute` 继续。同一个 recipe 只要内容不变，后续不会重复提示。

这项机制的目的是避免你在不了解内容的情况下执行潜在有风险的 recipe 代码。

---

### 卸载 goose 或清除缓存数据

在某些情况下，你可能需要彻底卸载 goose，或者在重装前清理本地数据。goose 在不同操作系统中会把日志、配置、会话和密钥保存在不同位置。

#### macOS

**数据位置**

- **日志与配置**：`~/.config/goose`
- **应用数据**：`~/Library/Application Support/Goose`
- **Secrets**：macOS Keychain（凭据名通常为 `goose`）

**清理步骤**

1. 关闭所有 goose 进程（CLI 和 GUI）
2. 打开 Keychain Access，删除名为 `goose` 的凭据
3. 删除数据目录：

   ```sh
   rm -rf ~/.config/goose
   rm -rf ~/Library/Application\\ Support/goose
   ```

4. 如果你使用的是 goose Desktop，再删除 Applications 里的 app

#### Linux

**数据位置**

- **数据 / 会话**：`~/.local/share/goose/`
- **日志**：`~/.local/state/goose/`
- **配置**：`~/.config/goose/`
- **Secrets**：系统 keyring（如果可用）

**清理步骤**

- 关闭所有 goose 进程（CLI 和 GUI）
- 从系统 keyring 中清除相关 secrets（如果有）
- 删除数据目录：

  ```sh
  rm -rf ~/.local/share/goose/
  rm -rf ~/.local/state/goose/
  rm -rf ~/.config/goose/
  ```

#### Windows

**数据位置**

- **配置与数据**：`%APPDATA%\\Block\\goose\\`
- **本地应用数据**：`%LOCALAPPDATA%\\Block\\goose\\`
- **Secrets**：Windows Credential Manager

**清理步骤**

1. 关闭所有 goose 进程（CLI 和 GUI）
2. 打开 Windows Credential Manager，删除与 `goose` 相关的凭据
3. 删除数据目录：

   ```cmd
   rmdir /s /q "%APPDATA%\\Block\\goose"
   rmdir /s /q "%LOCALAPPDATA%\\Block\\goose"
   ```

4. 如果使用的是 goose Desktop，可在系统设置的应用管理里卸载

完成清理后，如果你需要重新安装，可以回到标准安装流程重新开始。

---

### macOS 权限问题 {#macos-permission-issues}

如果你遇到 goose Desktop 启动后没有任何窗口，通常是文件或目录权限不正确导致的。最常见的情况是 goose 无法访问 `~/.config`，因而不能创建日志目录和日志文件。

如果你在使用过程中发现工具无法创建文件或目录，也可能是同一类权限问题。

#### 如何检查权限

1. 打开 Terminal
2. 运行：

   ```sh
   ls -ld ~/.config
   ```

**示例输出：**

```sh
drwx------  7 yourusername  staff  224 Jan 15 12:00 /Users/yourusername/.config
```

这里的 `rwx` 表示当前用户对该目录具备读（r）、写（w）和执行（x）权限。如果你自己的权限位里没有 `rwx`，就需要手动修正。

#### 如何修复权限

1. 给 `~/.config` 增加正确权限：

   ```sh
   chmod u+rw ~/.config
   ```

   如果 `~/.config` 还不存在，先创建再授权：

   ```sh
   mkdir -p ~/.config
   chmod u+rw ~/.config
   ```

2. 再次确认结果：

   ```sh
   ls -ld ~/.config
   ```

如果修完后问题仍然存在，可以尝试用管理员权限临时启动 goose，确认是否确实是权限导致：

```sh
sudo /Applications/Goose.app/Contents/MacOS/Goose
```

:::note
使用 `sudo` 启动 goose 可能会生成 root 拥有的文件，反而引出新的权限问题。这个方式只建议用于排查，不建议长期使用。
:::

#### 在系统设置里授予访问权限

1. 打开 `System Settings`
2. 进入 `Privacy & Security`
3. 打开 `Files & Folders`
4. 给 goose 授权访问相关目录

---

### Keychain / Keyring 错误 {#keychain-keyring-errors}

goose 默认会使用系统 keyring（macOS 上是 keychain）来保存密钥。如果运行环境无法访问 keyring，或者系统本身没有可用的 keyring 支持，可能会看到下面这种错误：

```bash
Error Failed to access secure storage (keyring): Platform secure storage failure: DBus error: The name org.freedesktop.secrets was not provided by any .service files
Please check your system keychain and run 'goose configure' again.
If your system is unable to use the keyring, please try setting secret key(s) via environment variables.
```

在部分环境里，goose 会自动回退到基于文件的密钥存储，详见下方的[自动回退说明](#keyring-cannot-be-accessed-automatic-fallback)。如果没有自动回退，你可以采用下面两种方式之一：

- 使用各 Provider 对应的环境变量。变量列表见[支持的 LLM Providers](/zh-CN/docs/getting-started/providers)。
  - 当前 shell 会话临时生效：
    ```bash
    export GOOGLE_API_KEY=$YOUR_KEY_HERE
    ```
  - 若要长期生效，把它写入 `~/.bashrc`、`~/.zshrc` 或对应 shell 配置文件
- 运行 `goose configure` 时，在保存到 keyring 的提示处选择 `No`

  ```bash
  $ goose configure

  Welcome to goose! Let's get you set up with a provider.
    you can rerun this command later to update your configuration

  ┌   goose-configure
  │
  ◇  Which model provider should we use?
  │  Google Gemini
  │
  ◇  GOOGLE_API_KEY is set via environment variable
  │
  ◇  Would you like to save this value to your keyring?
  │  No
  │
  ◇  Enter a model from that provider:
  │  gemini-2.0-flash-exp
  ```

- 如果你希望显式禁用 keyring，可以设置 `GOOSE_DISABLE_KEYRING` 为任意值，强制改用文件存储：

  ```bash
  GOOSE_DISABLE_KEYRING=1 goose configure
  ```

禁用 keyring 后，或者 goose 因无法访问 keyring 自动回退到文件存储后，secrets 会保存在：

- macOS / Linux：`~/.config/goose/secrets.yaml`
- Windows：`%APPDATA%\Block\goose\config\secrets.yaml`

更多细节见[配置文件](/zh-CN/docs/guides/config-files)。

---

### 无法访问 Keyring 时的自动回退 {#keyring-cannot-be-accessed-automatic-fallback}

在某些平台上，如果 goose 发现系统 keyring 明显不可用，会自动改用文件方式保存 secrets。

这种情况下，你可能会在日志中看到类似警告：

```text
Keyring unavailable. Using file storage for secrets.
```

自动回退只对当前 goose 进程生效。下一次启动新会话时，goose 仍会重新尝试使用 keyring。

如果你希望稳定地使用文件存储，而不是依赖自动判断，可以显式设置：

```bash
GOOSE_DISABLE_KEYRING=1 goose configure
```

`secrets.yaml` 的位置说明见[配置文件](/zh-CN/docs/guides/config-files)。

### Package Runner

许多外部扩展都依赖 package runner。如果你遇到下面这类错误：

```
Failed to start extension `{extension name}`: Could not run extension command (`{extension command}`): No such file or directory (os error 2)
Please check extension configuration for {extension name}.
```

通常意味着扩展本身还没有被正确安装，或者系统里缺少对应的 package runner。

例如 GitHub 扩展的命令是 `npx -y @modelcontextprotocol/server-github`。要运行这个命令，你的系统里需要先安装 [Node.js](https://nodejs.org/)。

---

### Windows 下 Node.js 扩展无法激活

如果你在 Windows 上激活基于 Node.js 的扩展时看到 `Node.js installer script not found`，通常是因为 goose 没能在预期路径中找到 Node.js。

#### 症状

- `node -v` 和 `npm -v` 正常
- 其它扩展（例如 Python 扩展）能正常工作
- 只有 Node.js 扩展激活失败

#### 解决方式

这通常出现在 Node.js 被装到了非标准路径，例如 `D:\Program Files\nodejs\`，而 goose 默认查找的是 `C:\Program Files\nodejs\`。

1. 先检查 Node.js 的安装路径：

   ```powershell
   where.exe node
   ```

2. 如果不在 `C:\Program Files\nodejs\`，可以创建符号链接：

   ```powershell
   mklink /D "C:\Program Files\nodejs" "D:\Program Files\nodejs"
   ```

   把 `D:\Program Files\nodejs` 替换成你真实的安装路径。

3. 重启 goose 后再试一次。

这样做的作用是让 goose 能在它预期的位置找到 Node.js，同时不影响你当前的实际安装目录。

---

### 检测到恶意软件包 {#malicious-package-detected}

如果你在使用扩展时看到 “blocked malicious package” 一类错误，表示某个扩展依赖的包被识别为恶意软件。错误消息通常会包含具体包名，例如：

```
Blocked malicious package: package-name@1.0.0 (npm). OSV MAL advisories: MAL-2024-1234
```

处理建议：

1. 优先寻找替代方案，可在[扩展目录](/extensions)或 [PulseMCP](https://www.pulsemcp.com/servers) 中查找同类扩展
2. 如有需要，可进一步核对扩展来源、包名和发布者
3. 如果你判断这是误报，可以到 [GitHub issue](https://github.com/block/goose/issues) 反馈

这项安全检查只针对本地执行、并使用 PyPI（`uvx`）或 NPM（`npx`）的外部扩展。它依赖 OSV 的实时数据；如果安全服务暂时不可用，扩展仍会按正常流程安装。

最佳实践仍然是：只安装来自可信官方来源的扩展。

---

### macOS 权限问题 {#macos-permission-issues}

如果你遇到 goose Desktop 启动后没有任何窗口，通常是文件或目录权限不正确导致的。最常见的情况是 goose 无法访问 `~/.config`，因而不能创建日志目录和日志文件。

如果你在使用过程中发现工具无法创建文件或目录，也可能是同一类权限问题。

#### 如何检查权限

1. 打开 Terminal
2. 运行：

   ```sh
   ls -ld ~/.config
   ```

**示例输出：**

```sh
drwx------  7 yourusername  staff  224 Jan 15 12:00 /Users/yourusername/.config
```

这里的 `rwx` 表示当前用户对该目录具备读（r）、写（w）和执行（x）权限。如果你自己的权限位里没有 `rwx`，就需要手动修正。

#### 如何修复权限

1. 给 `~/.config` 增加正确权限：

   ```sh
   chmod u+rw ~/.config
   ```

   如果 `~/.config` 还不存在，先创建再授权：

   ```sh
   mkdir -p ~/.config
   chmod u+rw ~/.config
   ```

2. 再次确认结果：

   ```sh
   ls -ld ~/.config
   ```

如果修完后问题仍然存在，可以尝试用管理员权限临时启动 goose，确认是否确实是权限导致：

```sh
sudo /Applications/Goose.app/Contents/MacOS/Goose
```

:::note
使用 `sudo` 启动 goose 可能会生成 root 拥有的文件，反而引出新的权限问题。这个方式只建议用于排查，不建议长期使用。
:::

#### 在系统设置里授予访问权限

1. 打开 `System Settings`
2. 进入 `Privacy & Security`
3. 打开 `Files & Folders`
4. 给 goose 授权访问相关目录

---

### WSL 中使用 Ollama Provider 的连接错误

如果你在 WSL 里把 Ollama 配成 Provider 时看到下面这类错误：

```
Execution error: error sending request for url (http://localhost:11434/v1/chat/completions)
```

通常意味着 WSL 里的 `localhost` 无法直接访问宿主机上的 Ollama。

1. 先检查服务是否可达：

   ```
   curl http://localhost:11434/api/tags
   ```

2. 如果返回 `failed to connect`，说明 WSL 可能需要改用宿主机 IP。可以运行：

   ```
   ip route show | grep -i default | awk '{ print $3 }'
   ```

3. 把得到的 IP 写进 goose 配置，替换 `localhost`。例如：

   ```
   http://172.24.80.1:11434
   ```

如果仍然连接失败，并且你使用的是 Windows 11 22H2 或以上版本，也可以尝试 WSL 的 [Mirrored Networking](https://learn.microsoft.com/en-us/windows/wsl/networking#mirrored-mode-networking)。

---

### 企业代理或防火墙问题 {#corporate-proxy-or-firewall-issues}

如果你处在企业代理或防火墙环境中，goose 无法连接到 LLM Provider 时，常见报错类似：

```
error sending request for url (https://api.openai.com/...)
failed to connect to api.openai.com
```

goose 支持通过标准环境变量和系统代理设置来配置 HTTP/HTTPS 代理；当两者同时存在时，环境变量优先。

处理方式：

1. 配置[代理环境变量](/zh-CN/docs/guides/environment-variables#network-configuration)：

   ```bash
   export HTTPS_PROXY="http://proxy.company.com:8080"
   export NO_PROXY="localhost,127.0.0.1,.internal"
   ```

2. 或者使用系统代理设置：
   - macOS：System Settings → Network → 选择连接 → Details → Proxies
   - Windows：Settings → Network & Internet → Proxy

3. 配置完成后重启 goose

如果问题依然存在，再检查：

- 代理地址和端口是否正确
- 是否需要认证信息，例如 `http://username:password@proxy:port`
- 代理是否允许访问你的 LLM Provider 域名

---

### Airgapped / 离线环境问题

如果你在 airgapped、完全离线、或网络严格受限的环境中使用 goose，可能会遇到 MCP Server 扩展无法激活、或运行时依赖下载失败的问题。

#### 常见症状

- 扩展激活失败，并提示缺少运行时环境
- 日志里出现 `hermit:fatal` 或下载失败
- 个人电脑上能正常工作，但公司网络中失败
- 错误消息类似：`Failed to start extension: Could not run extension command`

#### 原因

goose Desktop 使用打包好的 **shim**（例如 `npx` 和 `uvx` 的封装版本），它们会通过 Hermit 自动下载运行时。在受限网络中，这些下载通常会失败。

#### 解决方式：改用自定义命令名

1. 在系统上为 package runner 创建另外的命令名：

   ```bash
   # For uvx (Python packages)
   ln -s /usr/local/bin/uvx /usr/local/bin/runuv

   # For npx (Node.js packages)
   ln -s /usr/local/bin/npx /usr/local/bin/runnpx
   ```

2. 更新 MCP Server 配置，改用这些自定义命令名。

   原始写法：

   ```yaml
   extensions:
     example:
       cmd: uvx
       args: [mcp-server-example]
   ```

   替换为：

   ```yaml
   extensions:
     example:
       cmd: runuv  # This bypasses goose's shims
       args: [mcp-server-example]
   ```

3. 之所以有效，是因为 goose 只会把已知命令名（如 `npx`、`uvx`、`jbang`）替换成自己的 shim。自定义命令名会原样透传给系统里的真实可执行文件。

4. 如果在企业代理或完全离线环境中，上述方式仍不足以解决问题，通常就需要进一步定制并打包 goose Desktop，让它使用符合你网络约束的 shim、证书或代理配置。

---

### 还需要帮助？

如果问题还没解决，可以继续到 [Discord 社区](https://discord.gg/goose-oss) 求助。

:::tip
如果你能同时附上[诊断包](/zh-CN/docs/troubleshooting/diagnostics-and-reporting#diagnostics-system)，通常能更快定位问题。
:::
