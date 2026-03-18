---
sidebar_position: 1
title: 快速开始
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { RateLimits } from '@site/src/components/RateLimits';
import { ModelSelectionTip } from '@site/src/components/ModelSelectionTip';
import { OnboardingProviderSetup } from '@site/src/components/OnboardingProviderSetup';
import MacDesktopInstallButtons from '@site/src/components/MacDesktopInstallButtons';
import WindowsDesktopInstallButtons from '@site/src/components/WindowsDesktopInstallButtons';
import LinuxDesktopInstallButtons from '@site/src/components/LinuxDesktopInstallButtons';
import { PanelLeft } from 'lucide-react';

:::warning 中文文档为预览版
当前中文内容同步自英文 docs `main@f697e8d`（2026-03-18）。后续更新可能滞后，如有差异，请以英文原文为准。
:::

# 5 分钟上手 goose

goose 是一个可扩展的开源 AI agent，它会通过自动化编码任务来提升你的软件开发效率。

这份快速上手会带你完成：

- ✅ 安装 goose
- ✅ 配置你的 LLM
- ✅ 创建一个小应用
- ✅ 添加一个 MCP server

开始吧 🚀

## 安装 goose

<Tabs>
  <TabItem value="mac" label="macOS" default>
    你可以选择安装 goose Desktop 和 / 或 goose CLI：

    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        <MacDesktopInstallButtons
          introText="点击下方按钮下载 goose Desktop："
          siliconLabel="macOS Apple 芯片"
          intelLabel="macOS Intel"
        />
        <div style={{ marginTop: '1rem' }}>
          1. 解压下载的 zip 文件。
          2. 运行可执行文件启动 goose Desktop。
        </div>
      </TabItem>
      <TabItem value="cli" label="goose CLI">
        运行以下命令安装 goose：

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
        ```
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="linux" label="Linux">
    你可以选择安装 goose Desktop 和 / 或 goose CLI：

    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        <LinuxDesktopInstallButtons
          introText="点击下方按钮下载 goose Desktop："
          debLabel="DEB 安装包（Ubuntu / Debian）"
          rpmLabel="RPM 安装包（RHEL / Fedora）"
          flatpakLabel="Flatpak（通用）"
        />
        <div style={{ marginTop: '1rem' }}>
          **对于 Debian / Ubuntu 系发行版：**
          1. 下载 DEB 文件
          2. 在终端进入该文件所在目录
          3. 运行 `sudo dpkg -i (filename).deb`
          4. 从应用菜单启动 goose
        </div>
      </TabItem>
      <TabItem value="cli" label="goose CLI">
        在 Linux 上安装 goose CLI，请运行：

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
        ```
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="windows" label="Windows">
    你可以选择安装 goose Desktop 和 / 或 goose CLI：

    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        <WindowsDesktopInstallButtons
          introText="点击下方按钮下载 goose Desktop："
          windowsLabel="Windows"
        />
        <div style={{ marginTop: '1rem' }}>
          1. 解压下载的 zip 文件。
          2. 运行可执行文件启动 goose Desktop。
        </div>
      </TabItem>
      <TabItem value="cli" label="goose CLI">
        
        请在 **Git Bash**、**MSYS2** 或 **PowerShell** 中运行以下命令，以原生方式在 Windows 上安装 goose CLI：

        ```bash
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
        ```
        
        前置条件与环境要求请查看[安装指南](/zh-CN/docs/getting-started/installation)。

        :::info PATH 警告与 Keyring
        如果安装后看到 PATH 警告，需要先把 goose 加入 PATH，再执行 `goose configure`。详细步骤见 [Windows CLI 安装说明](/zh-CN/docs/getting-started/installation)。

        如果配置时提示你选择是否存储到 keyring，建议选择不存。如果遇到 keyring 报错，请继续参考 [Windows 设置说明](/zh-CN/docs/getting-started/installation#set-llm-provider)。
        :::

      </TabItem>
    </Tabs>
  </TabItem>
</Tabs>

## 配置 Provider

goose 需要依赖[支持的 LLM providers](/zh-CN/docs/getting-started/providers)来获得理解请求和执行任务所需的 AI 能力。首次使用时，系统会提示你配置一个 provider。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  在欢迎页中，你会看到这些选项：
  
  <OnboardingProviderSetup
    quickSetupDescription="goose 会根据你的 API Key 自动识别并配置 Provider。"
    chatGptDescription="使用 ChatGPT Plus / Pro 账户登录，访问 GPT-5 Codex 模型。"
    agentRouterDescription="通过自动配置快速接入多个 AI 模型。"
    openRouterDescription="通过单一 API 接入多个模型，按量计费。"
    otherProvidersDescription="在设置中手动配置其他 Provider。"
  />

  对于这份快速开始，建议先选 **Tetrate Agent Router**。Tetrate 提供多个 AI 模型入口，内置 rate limiting 与自动故障切换。更多 provider 信息见[配置 LLM Provider](/zh-CN/docs/getting-started/providers)。
  
  goose 会为你打开浏览器，完成 Tetrate 登录或注册。回到 goose Desktop 后，你就可以开始第一次会话了。
      
  :::info 免费额度
  第一次通过 goose 自动认证 Tetrate 时，你会获得 10 美元免费额度。新老 Tetrate 用户都可以使用这个活动。
  :::
    
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  1. 在终端里运行：

    ```sh
    goose configure
    ```

  2. 在菜单中选择 `Configure Providers` 并按 Enter。

    ```
   ┌   goose-configure 
   │
   ◆  What would you like to configure?
   │  ● Configure Providers (Change provider or update credentials)
   │  ○ Add Extension 
   │  ○ Toggle Extensions 
   │  ○ Remove Extension 
   │  ○ goose settings 
   └  
   ```
   3. 选择一个模型 provider。对于这份快速开始，建议选择 `Tetrate Agent Router Service`。Tetrate 提供多个 AI 模型入口，内置 rate limiting 与自动故障切换。其他 provider 说明见[配置 LLM Provider](/zh-CN/docs/getting-started/providers)。

   ```
   ┌   goose-configure 
   │
   ◇  What would you like to configure?
   │  Configure Providers 
   │
   ◆  Which model provider should we use?
   │  ○ Amazon Bedrock 
   │  ○ Amazon SageMaker TGI 
   │  ○ Anthropic 
   │  ○ Azure OpenAI 
   │  ○ ChatGPT Codex 
   │  ○ Claude Code CLI 
   │  ○ ...
   |  ● Tetrate Agent Router Service (Enterprise router for AI models)
   │  ○ ...
   └  
   ```
    :::info 免费额度
    第一次通过 goose 自动认证 Tetrate 时，你会获得 10 美元免费额度。新老 Tetrate 用户都适用。
    :::

   4. 按提示输入 API Key（以及其他需要的配置项）。

   ```
   ┌   goose-configure 
   │
   ◇  What would you like to configure?
   │  Configure Providers 
   │
   ◇  Which model provider should we use?
   │  Tetrate Agent Router Service 
   │
   ◆  Provider Tetrate Agent Router Service requires TETRATE_API_KEY, please enter a value
   │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
   └  
   ```
    :::tip GitHub Copilot 认证
    GitHub Copilot 不使用 API key。选择 `GitHub Copilot` 作为 provider 后，系统会生成认证码并复制到剪贴板，同时打开浏览器；你只需要把认证码粘贴进去即可完成登录。

    更完整说明见 [GitHub Copilot Authentication](/zh-CN/docs/getting-started/providers#github-copilot-authentication)。
    :::

   5. 选择或搜索你要使用的模型。
   ```
   │
   ◇  Model fetch complete
   │
   ◆  Select a model:
   │  ○ Search all models...
   │  ○ gemini-2.5-pro
   │  ○ gemini-2.0-flash
   |  ○ gemini-2.0-flash-lite
   │  ● gpt-5 (Recommended)
   |  ○ gpt-5-mini
   |  ○ gpt-5-nano
   |  ○ gpt-4.1
   │
   ◓  Checking your configuration...
   └  Configuration saved successfully
   ```
   <RateLimits
     title="Provider 速率限制"
     providerLinkText="不同 provider"
     providerLinkHref="/zh-CN/docs/getting-started/providers"
     providerText=" 的限流策略差异很大。开始时可以先选择你最熟悉、也最稳定的 provider。"
     rateLimitsText="如果你频繁遇到速率限制，可以参考"
     guideLinkText="处理速率限制"
     guideHref="/zh-CN/docs/guides/handling-llm-rate-limits-with-goose"
     guideText="这篇指南，或者切换模型，或改用支持自动故障切换的 provider。"
   />
   <ModelSelectionTip
     title="模型选择建议"
     description="如果你还不确定用哪个模型，先选一个平衡型默认模型即可。后续可以根据任务类型和成本再切换。"
   />
  </TabItem>
</Tabs>

## 开始会话

会话是你和 goose 之间的一段连续对话。现在开始创建一个。

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        选择好 LLM provider 后，点击侧边栏中的 `Home`。

        在输入框里直接输入问题、任务或指令，goose 就会立刻开始工作。
    </TabItem>
    <TabItem value="cli" label="goose CLI">
        1. 先创建一个空目录（例如 `goose-demo`），然后在终端进入该目录。
        2. 运行以下命令启动新会话：
        ```sh
        goose session
        ```

    </TabItem>
</Tabs>

## 编写 Prompt

你可以直接像对开发者说话一样，把任务交给 goose。

现在让 goose 帮你做一个井字棋小游戏：

```text
create an interactive browser-based tic-tac-toe game in javascript where a player competes against a bot
```

goose 会先生成计划，然后立即开始实现。完成后，你的目录里通常会出现一个 JavaScript 文件，以及一个可直接游玩的 HTML 页面。

## 启用一个扩展

虽然你也可以手动进入工作目录，再自己在浏览器中打开 HTML 文件，但如果 goose 能直接帮你打开浏览器，不是更方便吗？现在启用 [`Computer Controller` 扩展](/zh-CN/docs/mcp/computer-controller-mcp)，让 goose 获得浏览器操作能力。

<Tabs groupId="interface">

    <TabItem value="ui" label="goose Desktop" default>
        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏。
        2. 点击侧边栏中的 `Extensions`。
        3. 启用 `Computer Controller` 扩展。这个扩展会提供网页自动化、文件缓存和自动化流程能力。
        4. 回到当前会话。
        5. 现在 goose 已经有浏览器能力了，让它在浏览器里打开你的游戏：
    </TabItem>
    <TabItem value="cli" label="goose CLI">
        1. 先按 `Ctrl+C` 结束当前会话，回到终端。
        2. 运行配置命令：
        ```sh
        goose configure
        ```
        3. 依次选择 `Add Extension` > `Built-in Extension` > `Computer Controller`，并把 timeout 设为 300 秒。这个扩展会提供网页自动化、文件缓存和自动化流程能力。
        ```
        ┌   goose-configure
        │
        ◇  What would you like to configure?
        │  Add Extension
        │
        ◇  What type of extension would you like to add?
        │  Built-in Extension
        │
        ◇  Which built-in extension would you like to enable?
        │  Computer Controller
        │
        ◇  Please set the timeout for this tool (in secs):
        │  300
        │
        └  Enabled computercontroller extension
        ```
        4. 现在 goose 已经有浏览器能力了，恢复你上一轮会话：
        ```sh
         goose session -r
        ```
        5. 让 goose 在浏览器里打开你的游戏：
    </TabItem>
</Tabs>

```text
open the tic-tac-toe game in a browser
```

去玩一下你的小游戏吧，别装了，你肯定想点开试试 😂 祝你好运！

## 下一步

恭喜，你已经成功用 goose 做出了一个 Web 应用！🎉

你可以继续这样探索：
* 继续当前会话，把你的小游戏做得更完整（样式、功能、体验）
* 浏览更多可用的[扩展](/extensions)，继续增强 goose 的能力
* 给 goose 提供一组[提示规则](/zh-CN/docs/guides/context-engineering/using-goosehints)，让它在会话里更贴合你的习惯
* 如果你不希望 goose 默认自治运行，可以先了解一下[Developer 扩展说明](/zh-CN/docs/mcp/developer-mcp)
