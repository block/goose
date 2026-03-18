---
sidebar_position: 1
title: 安装 goose
---

import Tabs from "@theme/Tabs";
import TabItem from "@theme/TabItem";
import { RateLimits } from "@site/src/components/RateLimits";
import { OnboardingProviderSetup } from "@site/src/components/OnboardingProviderSetup";
import { ModelSelectionTip } from "@site/src/components/ModelSelectionTip";
import MacDesktopInstallButtons from "@site/src/components/MacDesktopInstallButtons";
import WindowsDesktopInstallButtons from "@site/src/components/WindowsDesktopInstallButtons";
import LinuxDesktopInstallButtons from "@site/src/components/LinuxDesktopInstallButtons";
import { PanelLeft } from "lucide-react";

# 安装 goose

这页汇总 goose Desktop 和 goose CLI 的安装方式。想先快速跑通流程，可以先看 [快速开始](../quickstart.md)。

<Tabs>
  <TabItem value="mac" label="macOS" default>
    你可以选择安装 goose Desktop、goose CLI，或两者都装。

    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        你可以直接从浏览器下载 goose Desktop，或者通过 [Homebrew](https://brew.sh/) 安装。

        <h3 style={{ marginTop: "1rem" }}>方案 1：下载安装包</h3>
        <MacDesktopInstallButtons
          introText="点击下面的按钮下载 macOS 版本的 goose Desktop："
          siliconLabel="macOS Apple 芯片"
          intelLabel="macOS Intel"
        />

        <div style={{ marginTop: "1rem" }}>
          1. 解压下载的压缩包。
          2. 运行应用启动 goose Desktop。
        </div>

        :::tip 更新
        建议定期[更新 goose](/zh-CN/docs/guides/updating-goose)。
        :::

        <h3>方案 2：通过 Homebrew 安装</h3>
        Homebrew 安装的也是[同一个桌面应用](https://github.com/Homebrew/homebrew-cask/blob/master/Casks/b/block-goose.rb)，但它还能帮你统一处理后续更新：

        ```bash
        brew install --cask block-goose
        ```

        ---

        <div style={{ marginTop: "1rem" }}>
          :::info 权限
          如果你使用的是 Apple Mac M3，且 goose Desktop 启动后看不到窗口，请检查 `~/.config` 目录是否具有读写权限。

          goose 需要在这里创建日志目录和日志文件。权限修正后，应用一般就能正常加载。操作细节可参考[已知问题指南](/zh-CN/docs/troubleshooting/known-issues#macos-permission-issues)。
          :::
        </div>
      </TabItem>
      <TabItem value="cli" label="goose CLI">
        你可以直接下载安装脚本，或者通过 [Homebrew](https://brew.sh/) 安装。

        <h3 style={{ marginTop: "1rem" }}>方案 1：下载安装脚本</h3>
        在 macOS 上运行下面的命令安装最新版 goose CLI：

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
        ```

        这段脚本会自动下载最新版 goose 并把它安装到系统里。

        如果你想跳过交互式配置：

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```

        :::tip 更新
        建议保持 goose 为最新版本。更新时运行：

        ```sh
        goose update
        ```
        :::

        <h3>方案 2：通过 Homebrew 安装</h3>
        Homebrew 安装的是[预编译 CLI 二进制](https://github.com/Homebrew/homebrew-core/blob/master/Formula/b/block-goose-cli.rb)，也能帮你处理更新：

        ```bash
        brew install block-goose-cli
        ```
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="linux" label="Linux">
    你可以选择安装 goose Desktop、goose CLI，或两者都装。

    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        你可以直接从浏览器下载 goose Desktop。

        <h3 style={{ marginTop: "1rem" }}>下载安装包</h3>
        <LinuxDesktopInstallButtons
          introText="点击下面的按钮下载 Linux 版本的 goose Desktop："
          debLabel="DEB 安装包（Ubuntu / Debian）"
          rpmLabel="RPM 安装包（RHEL / Fedora）"
          flatpakLabel="Flatpak（通用）"
        />

        <div style={{ marginTop: "1rem" }}>
          **对于 Debian / Ubuntu 系发行版：**
          1. 下载 `.deb` 文件。
          2. 在终端进入下载目录。
          3. 运行 `sudo dpkg -i 文件名.deb`。
          4. 从应用菜单启动 goose。
        </div>

        :::tip 更新
        建议定期[更新 goose](/zh-CN/docs/guides/updating-goose)。
        :::
      </TabItem>
      <TabItem value="cli" label="goose CLI">
        在 Linux 上安装 goose CLI，运行：

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
        ```

        这段脚本会自动获取最新版本并安装到本机。

        跳过交互式配置：

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```

        :::tip 更新
        建议保持 goose 为最新版本。更新时运行：

        ```sh
        goose update
        ```
        :::
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="windows" label="Windows">
    你可以选择安装 goose Desktop、goose CLI，或两者都装。

    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        你可以直接从浏览器下载 goose Desktop。

        <h3 style={{ marginTop: "1rem" }}>下载安装包</h3>
        <WindowsDesktopInstallButtons
          introText="点击下面的按钮下载 Windows 版本的 goose Desktop："
          windowsLabel="Windows"
        />

        <div style={{ marginTop: "1rem" }}>
          1. 解压下载的压缩包。
          2. 运行应用启动 goose Desktop。
        </div>

        :::tip 更新
        建议定期[更新 goose](/zh-CN/docs/guides/updating-goose)。
        :::
      </TabItem>
      <TabItem value="cli" label="goose CLI">
        要在原生 Windows 上安装 goose CLI，建议使用以下任一环境：

        - **Git Bash**（推荐）：随 [Git for Windows](https://git-scm.com/download/win) 提供
        - **MSYS2**：可从 [msys2.org](https://www.msys2.org/) 获取
        - **PowerShell**：Windows 10/11 默认提供

        在你选择的环境里运行：

        ```bash
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
        ```

        如果你想跳过交互式配置：

        ```bash
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```

        **PowerShell 安装方式：**
        先把 PowerShell 安装脚本下载到当前目录：

        ```powershell
        Invoke-WebRequest -Uri "https://raw.githubusercontent.com/block/goose/main/download_cli.ps1" -OutFile "download_cli.ps1";
        ```

        然后执行安装：

        ```powershell
        .\download_cli.ps1
        ```

        :::info Windows PATH 设置
        如果安装后提示 `goose` 不在 PATH 中，需要手动把它加入 PATH。

        <details>
          <summary>适用于 Git Bash / MSYS2</summary>

          ```bash
          echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
          source ~/.bashrc
          ```
        </details>

        <details>
          <summary>适用于 PowerShell</summary>

          ```powershell
          # Add to your PowerShell profile
          $profilePath = $PROFILE
          if (!(Test-Path $profilePath)) { New-Item -Path $profilePath -ItemType File -Force }
          Add-Content -Path $profilePath -Value '$env:PATH = "$env:USERPROFILE\.local\bin;$env:PATH"'
          # Reload profile or restart PowerShell
          . $PROFILE
          ```
        </details>

        配置好 PATH 之后，你就可以在任意目录直接运行 `goose`。
        :::

        <details>
          <summary>通过 Windows Subsystem for Linux (WSL) 安装</summary>

          我们更推荐原生 Windows 安装，但如果你更习惯 Linux 风格环境，也可以通过 WSL 使用 goose CLI。

          1. 以管理员身份打开 [PowerShell](https://learn.microsoft.com/en-us/powershell/scripting/install/installing-powershell-on-windows)，安装 WSL 和默认 Ubuntu 发行版：

          ```bash
          wsl --install
          ```

          2. 如果系统提示重启，请重启电脑。之后，或者如果 WSL 已经安装好，运行：

          ```bash
          wsl -d Ubuntu
          ```

          3. 在 Ubuntu shell 中执行 goose 安装脚本：

          ```bash
          curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
          ```

          :::tip
          如果下载或解压时遇到问题，可能需要先安装 `bzip2`：

          ```bash
          sudo apt update && sudo apt install bzip2 -y
          ```
          :::

          如果你想跳过交互式配置：

          ```sh
          curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
          ```

          如果需要，也可以把 goose 加入 PATH，并顺手写入 API Key：

          ```bash
          echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
          echo 'export OPENAI_API_KEY=your_api_key' >> ~/.bashrc
          source ~/.bashrc
          ```
        </details>
      </TabItem>
    </Tabs>
  </TabItem>
</Tabs>

## 设置 LLM Provider {#set-llm-provider}

goose 需要通过一个可用的 [LLM Provider][providers] 获得理解请求和执行任务所需的 AI 能力。首次使用时，它会引导你完成 Provider 配置。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    第一次打开 goose Desktop 时，欢迎页通常会提供以下入口：

    <OnboardingProviderSetup
      quickSetupDescription="goose 会根据你的 API Key 自动识别并配置 Provider。"
      chatGptDescription="使用 ChatGPT Plus / Pro 订阅登录，访问 GPT-5 Codex 模型。"
      agentRouterDescription="通过自动配置快速接入多个 AI 模型。"
      openRouterDescription="通过单一 API 使用多个模型，按量付费。"
      otherProvidersDescription="在设置里手动配置其它 Provider。"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    CLI 会自动进入配置流程，你可以选择以下方式完成 Provider 配置：

    - **OpenRouter Login**：登录 OpenRouter，自动配置模型
    - **Tetrate Agent Router Service Login**：登录 Tetrate，自动配置模型
    - **Manual Configuration**：手动选择 Provider 并填写凭据

    典型流程如下：

    ```text
    ┌   goose-configure
    │
    ◇ How would you like to set up your provider?
    │ Tetrate Agent Router Service Login
    │
    Opening browser for Tetrate Agent Router Service authentication...
    [goose opens the browser and prints details]

    Authentication complete!

    Configuring Tetrate Agent Router Service...
    ✓ Tetrate Agent Router Service configuration complete
    ✓ Models configured successfully

    Testing configuration...
    ✓ Configuration test passed!
    ✓ Developer extension enabled!
    └ Tetrate Agent Router Service setup complete! You can now use goose.
    ```

    如果你选择手动配置：

    ```sh
    goose configure
    ```

    然后依次完成：

    1. 选择 `Configure Providers`
    2. 选择 Provider
    3. 输入 API Key 或其他配置项
    4. 选择模型
    5. 保存设置

    :::info Windows 用户
    如果你在手动配置 Provider 时遇到 keyring 相关错误，建议在提示时选择不要写入 keyring，改用环境变量手动配置，例如：

    ```bash
    export OPENAI_API_KEY={your_api_key}
    ```

    然后重新运行 `goose configure`。goose 会检测到该环境变量，并显示：

    ```text
    ● OPENAI_API_KEY is set via environment variable
    ```

    若要跨会话持久保存，可把它写进 shell profile：

    ```bash
    echo 'export OPENAI_API_KEY=your_api_key' >> ~/.bashrc
    source ~/.bashrc
    ```
    :::
  </TabItem>
</Tabs>

:::tip 模型选择
<ModelSelectionTip text="goose 很依赖 tool calling 能力。对大多数开发任务，优先选择 tool calling 表现稳定的模型。" />
:::

:::info 免费额度
第一次通过 goose 自动完成 Tetrate 认证时，你会获得 10 美元免费额度；新老 Tetrate 用户都适用。
:::

## 更新 Provider

你可以随时更换 LLM Provider、切换模型或更新 API Key。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏。
    2. 点击 `Settings`。
    3. 进入 `Models` 标签。
    4. 选择更新 Provider、切换模型，或者点击 `Reset Provider and Model` 重新开始配置。更细的说明见 [配置 LLM Provider](./providers.md#configure-provider-and-model)。
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    1. 运行：

       ```sh
       goose configure
       ```

    2. 选择 `Configure Providers`。
    3. 按提示更新 Provider、凭据或模型。

    **示例：**

    在配置时，使用上下方向键高亮目标项，再按 Enter：

    ```text
    ┌   goose-configure
    │
    ◇ What would you like to configure?
    │ Configure Providers
    │
    ◇ Which model provider should we use?
    │ Google Gemini
    │
    ◇ Provider Google Gemini requires GOOGLE_API_KEY, please enter a value
    │▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇ Enter a model from that provider:
    │ gemini-2.0-flash-exp
    │
    ◇  Hello there! You're all set to use me, so please ask away!
    │
    └  Configuration saved successfully
    ```
  </TabItem>
</Tabs>

<RateLimits
  title="计费与速率限制"
  providerLinkText="Google Gemini"
  providerText=" 提供可直接开始使用的免费额度。除此之外，你需要确保在所选 LLM Provider 账户中有可用余额，才能正常发起请求。"
  rateLimitsText="部分 Provider 还会限制 API 请求频率，这会直接影响使用体验。你可以继续阅读 "
  guideLinkText="设置 LLM 速率限制"
  guideHref="/zh-CN/docs/guides/handling-llm-rate-limits-with-goose"
  guideText="，了解如何在使用 goose 时更高效地管理这些限制。"
/>

## 运行 goose

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    选择好 Provider 后，Desktop 会直接进入可用的会话界面。你只需要在输入框里输入任务即可开始。
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    在终端进入目标目录后运行：

    ```sh
    goose session
    ```
  </TabItem>
</Tabs>

## 共享配置

goose CLI 和 Desktop 共享核心配置，包括：

- LLM Provider 设置
- 当前模型
- 扩展配置

这意味着你可以在 Desktop 里完成配置，再切回 CLI 继续使用；反之亦然。更完整的说明见 [Config Files](/zh-CN/docs/guides/config-files)。

:::info
虽然核心配置在两个界面之间是共享的，但扩展可以自行决定如何存储认证信息。有些扩展会直接使用共享配置文件，有些则会使用自己的存储逻辑。
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    你可以通过以下方式进入共享配置：

    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏。
    2. 点击 `Settings`。
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    使用下面的命令管理共享配置：

    ```sh
    goose configure
    ```
  </TabItem>
</Tabs>

## 在 CI/CD 中固定 goose 版本

在 CI/CD 或其它自动化、非交互环境中，建议通过 `GOOSE_VERSION` 固定特定版本，以保证安装可复现，并避免在 `stable` tag 不包含对应 CLI 二进制资源时出现下载 404。

完整示例与用法说明见 [CI/CD Environments](/zh-CN/docs/tutorials/cicd)。

## 为 Linux 发行版生成 manpages

如果你正在为某个 Linux 发行版打包 goose，或者要做自定义构建，可以从 CLI 命令定义自动生成 Unix manpages：

```bash
just generate-manpages
```

这个命令会在 `target/man/` 下生成 ROFF 格式的 manpage（例如 `goose.1`、`goose-session.1`），你可以把它们安装到 `/usr/share/man/man1/`，从而通过 `man` 命令提供离线文档。

manpage 生成需要 goose 源码仓库，主要面向 Fedora、Debian 等发行版的打包维护者。实现细节可参考 [generate_manpages.rs](https://github.com/block/goose/blob/main/crates/goose-cli/src/bin/generate_manpages.rs)。

## 额外资源

- 想先跑通第一轮体验：看 [快速开始](../quickstart.md)
- 想专门配置 Provider：看 [配置 LLM Provider](./providers.md)
- 想启用扩展：看 [Using Extensions](./using-extensions.md)

[using-extensions]: /zh-CN/docs/getting-started/using-extensions
[providers]: /zh-CN/docs/getting-started/providers
[handling-rate-limits]: /zh-CN/docs/guides/handling-llm-rate-limits-with-goose
[config-files]: /zh-CN/docs/guides/config-files
