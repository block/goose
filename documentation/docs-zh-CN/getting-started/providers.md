---
sidebar_position: 2
title: 配置 LLM Provider
---

import Tabs from "@theme/Tabs";
import TabItem from "@theme/TabItem";
import { PanelLeft } from "lucide-react";
import { ModelSelectionTip } from "@site/src/components/ModelSelectionTip";
import { OnboardingProviderSetup } from "@site/src/components/OnboardingProviderSetup";

# 支持的 LLM Providers

goose 兼容很多 LLM Providers，你可以按自己的成本、模型偏好、部署方式和安全要求来选择。

:::tip 模型选择
<ModelSelectionTip text="goose 很依赖 tool calling。优先选择在函数调用、长上下文和多轮任务里表现稳定的模型。" />
[Berkeley Function-Calling Leaderboard][function-calling-leaderboard] 也可以作为选型参考。
:::

## 可用 Providers {#available-providers}

| Provider | 说明 | 主要参数 |
|---|---|---|
| [Amazon Bedrock](https://aws.amazon.com/bedrock/) | 提供 Claude、Jurassic-2 等多种基础模型。**AWS 环境变量需要提前配置，不能完全通过 `goose configure` 完成。** | 凭证认证：`AWS_PROFILE`，或 `AWS_ACCESS_KEY_ID`、`AWS_SECRET_ACCESS_KEY`、`AWS_REGION`<br /><br />Bearer Token 认证：`AWS_BEARER_TOKEN_BEDROCK` 与 `AWS_REGION`、`AWS_DEFAULT_REGION` 或 `AWS_PROFILE` |
| [Amazon SageMaker TGI](https://docs.aws.amazon.com/sagemaker/latest/dg/realtime-endpoints.html) | 通过 SageMaker endpoint 运行 Text Generation Inference 模型。**AWS 凭证需要预先配置。** | `SAGEMAKER_ENDPOINT_NAME`，`AWS_REGION`（可选），`AWS_PROFILE`（可选） |
| [Anthropic](https://www.anthropic.com/) | 使用 Claude 等模型。 | `ANTHROPIC_API_KEY`，`ANTHROPIC_HOST`（可选） |
| [Avian](https://avian.io/) | 面向 DeepSeek、Kimi、GLM、MiniMax 等模型的低成本推理 API，兼容 OpenAI 风格接口。 | `AVIAN_API_KEY`，`AVIAN_HOST`（可选） |
| [Azure OpenAI](https://learn.microsoft.com/en-us/azure/ai-services/openai/) | 使用托管在 Azure 上的 OpenAI 模型，支持 API Key 和 Azure Credential Chain。 | `AZURE_OPENAI_ENDPOINT`，`AZURE_OPENAI_DEPLOYMENT_NAME`，`AZURE_OPENAI_API_KEY`（可选） |
| [ChatGPT Codex](https://chatgpt.com/codex) | 接入面向代码生成与理解优化的 GPT-5 Codex 模型。**需要 ChatGPT Plus / Pro 订阅。** | 无需手工 API Key。CLI 和 Desktop 都通过浏览器 OAuth 完成认证。 |
| [Databricks](https://www.databricks.com/) | 数据与 AI 一体平台，可统一托管并调用模型。 | `DATABRICKS_HOST`，`DATABRICKS_TOKEN` |
| [Docker Model Runner](https://docs.docker.com/ai/model-runner/) | 在 Docker Desktop 或 Docker CE 中运行本地模型，并通过 OpenAI 兼容接口接入。**因为是本地模型，需要先拉模型。** | `OPENAI_HOST`，`OPENAI_BASE_PATH` |
| [Gemini](https://ai.google.dev/gemini-api/docs) | Google 的多模态模型。Gemini 3 支持可配置的 [thinking level](#gemini-3-thinking-levels)。 | `GOOGLE_API_KEY`，`GEMINI3_THINKING_LEVEL`（可选） |
| [GCP Vertex AI](https://cloud.google.com/vertex-ai) | 通过 Google Cloud Vertex AI 使用 Gemini、Claude 等模型。**认证需要提前按 Google Cloud 要求配置。** | `GCP_PROJECT_ID`，`GCP_LOCATION`，以及可选的重试参数：`GCP_MAX_RATE_LIMIT_RETRIES`、`GCP_MAX_OVERLOADED_RETRIES`、`GCP_INITIAL_RETRY_INTERVAL_MS`、`GCP_BACKOFF_MULTIPLIER`、`GCP_MAX_RETRY_INTERVAL_MS` |
| [GitHub Copilot](https://docs.github.com/en/copilot/using-github-copilot/ai-models) | 通过 GitHub Copilot 基础设施访问 OpenAI、Anthropic、Google 等模型。**需要有 Copilot 权限的 GitHub 账号。** | 无需手工 API Key。CLI 和 Desktop 都使用 [device flow](#github-copilot-authentication) 认证。 |
| [Groq](https://groq.com/) | 高性能推理硬件与模型服务。 | `GROQ_API_KEY` |
| [LiteLLM](https://docs.litellm.ai/docs/) | 多模型代理层，统一 API，支持 prompt caching 和多后端代理。 | `LITELLM_HOST`，`LITELLM_BASE_PATH`（可选），`LITELLM_API_KEY`（可选），`LITELLM_CUSTOM_HEADERS`（可选），`LITELLM_TIMEOUT`（可选） |
| [LM Studio](https://lmstudio.ai/) | 通过 LM Studio 提供的 OpenAI 兼容本地服务运行模型。**先下载模型。** | 通常无需额外参数，默认连接 `localhost:1234` |
| [Mistral AI](https://mistral.ai/) | 提供通用模型、代码模型（Codestral）和多模态模型（Pixtral）。 | `MISTRAL_API_KEY` |
| [Ollama](https://ollama.com/) | 本地模型运行器，支持 Qwen、Llama、DeepSeek 等开源模型。**先下载并启动模型。** | `OLLAMA_HOST` |
| [OpenAI](https://platform.openai.com/api-keys) | 使用 GPT-4o、o 系列等模型，也可接入 OpenAI 兼容 endpoint。**`o1-mini` 和 `o1-preview` 不支持，因为 goose 依赖 tool calling。** | `OPENAI_API_KEY`，`OPENAI_HOST`（可选），`OPENAI_ORGANIZATION`（可选），`OPENAI_PROJECT`（可选），`OPENAI_CUSTOM_HEADERS`（可选） |
| [OpenRouter](https://openrouter.ai/) | 聚合多个模型供应商，便于统一调用与限流。 | `OPENROUTER_API_KEY` |
| [OVHcloud AI](https://www.ovhcloud.com/en/public-cloud/ai-endpoints/) | 通过 AI Endpoints 访问 Qwen、Llama、Mistral、DeepSeek 等开源模型。 | `OVHCLOUD_API_KEY` |
| [Ramalama](https://ramalama.ai/) | 基于 OCI 容器运行时的本地模型运行器，可通过 goose 的 Ollama provider 兼容接入。**先下载并运行模型。** | `OLLAMA_HOST` |
| [Snowflake](https://docs.snowflake.com/user-guide/snowflake-cortex/aisql#choosing-a-model) | 通过 Snowflake Cortex 使用最新模型，包括 Claude。**需要 Snowflake 账号和 PAT。** | `SNOWFLAKE_HOST`，`SNOWFLAKE_TOKEN` |
| [Tanzu AI Services](https://techdocs.broadcom.com/us/en/vmware-tanzu/platform/ai-services/10-3/ai/index.html) | 通过 VMware Tanzu 平台统一访问企业管理的 LLM。模型从 endpoint 动态获取。 | `TANZU_AI_API_KEY`，`TANZU_AI_ENDPOINT` |
| [Tetrate Agent Router Service](https://router.tetrate.ai) | 统一 AI 网关，可接入 Claude、Gemini、GPT 和开源模型，支持 PKCE 流程。 | `TETRATE_API_KEY`，`TETRATE_HOST`（可选） |
| [Venice AI](https://venice.ai/home) | 提供以隐私为重点的开源模型访问能力，如 Llama、Mistral、Qwen。 | `VENICE_API_KEY`，`VENICE_HOST`（可选），`VENICE_BASE_PATH`（可选），`VENICE_MODELS_PATH`（可选） |
| [xAI](https://x.ai/) | 使用 xAI 的 Grok 系列模型。 | `XAI_API_KEY`，`XAI_HOST`（可选） |

:::tip Claude Prompt Caching
通过 Anthropic、Databricks、OpenRouter、LiteLLM 使用 Claude 模型时，goose 会自动启用 Anthropic 的 [prompt caching](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)。这会在请求中加入 `cache_control` 标记，能降低长会话的成本。实现细节可参考 [provider implementations](https://github.com/block/goose/tree/main/crates/goose/src/providers)。
:::

### CLI Providers

goose 还支持一类特殊的“透传式” CLI providers。它们直接复用你已有的 CLI 工具和订阅，而不是单独按 token 付费。

| Provider | 说明 | 要求 |
|---|---|---|
| [Claude Code](https://www.anthropic.com/claude-code) (`claude-code`) | 使用 Anthropic 的 Claude CLI 工具，并复用你的 Claude Code 订阅。上下文窗口可到 200K。 | 已安装并登录 Claude CLI，且有有效 Claude Code 订阅 |
| [OpenAI Codex](https://developers.openai.com/codex/cli) (`codex`) | 使用 OpenAI Codex CLI，并复用 ChatGPT Plus / Pro 订阅。 | 已安装并登录 Codex CLI，且有有效 ChatGPT Plus / Pro 订阅 |
| [Cursor Agent](https://docs.cursor.com/en/cli/overview) (`cursor-agent`) | 使用 Cursor 的 CLI 工具，接入 GPT-5、Claude 4 等模型。 | 已安装并登录 `cursor-agent` |
| [Gemini CLI](https://ai.google.dev/gemini-api/docs) (`gemini-cli`) | 使用 Google Gemini CLI 工具，并复用 Google AI 订阅。 | 已安装并登录 Gemini CLI |

:::tip CLI Providers
CLI provider 的成本通常更可控，因为它们复用你现有订阅；但工作方式和 API provider 不同，本质上是执行 CLI 命令并接入工具原生能力。详细配置见 [CLI Providers 指南](/zh-CN/docs/guides/cli-providers)。
:::

### ACP Providers

goose 支持把 [Agent Client Protocol (ACP)](https://agentclientprotocol.com/) agent 当作 provider 使用。ACP provider 会把 goose 扩展透传给对应 agent，作为 MCP server 暴露出去。

| Provider | 说明 | 要求 |
|---|---|---|
| [Claude ACP](https://github.com/zed-industries/claude-agent-acp) (`claude-acp`) | 通过 ACP 使用 Claude Code，同时把 goose 扩展透传给 agent。 | `npm install -g @zed-industries/claude-agent-acp`，有效 Claude Code 订阅 |
| [Codex ACP](https://github.com/zed-industries/codex-acp) (`codex-acp`) | 通过 ACP 使用 OpenAI Codex，并把 goose 扩展透传给 agent。 | `npm install -g @zed-industries/codex-acp`，有效 ChatGPT Plus / Pro 订阅 |

:::tip ACP Providers
详细接入方式见 [ACP Providers 指南](/zh-CN/docs/guides/acp-providers)。
:::

## 配置 Provider 和模型 {#configure-provider-and-model}

无论你选择哪一种 provider，都可以在 goose Desktop 的 `Models` 标签页里操作，或者在 CLI 里执行 `goose configure`。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    **首次配置**

    第一次打开 goose Desktop 时，你通常会看到这些入口：

    <OnboardingProviderSetup
      quickSetupDescription="goose 会根据你的 API Key 自动识别并配置 Provider。"
      chatGptDescription="使用 ChatGPT Plus / Pro 订阅登录，访问 GPT-5 Codex 模型。"
      agentRouterDescription="通过自动配置快速接入多个 AI 模型。"
      openRouterDescription="通过单一 API 使用多个模型，按量付费。"
      otherProvidersDescription="在设置里手动配置其它 Provider。"
    />

    <Tabs groupId="setup">
      <TabItem value="apikey" label="Quick Setup" default>
        1. 选择 `Quick Setup with API Key`
        2. 输入你的 API Key（例如 OpenAI、Anthropic 或 Google）
        3. goose 会自动识别并配置对应 Provider
        4. 配置完成后即可开始第一个会话
      </TabItem>

      <TabItem value="chatgpt" label="ChatGPT Subscription">
        1. 选择 `ChatGPT Subscription`
        2. goose 会打开浏览器，让你用有效的 ChatGPT Plus / Pro 订阅账号登录
        3. 授权 goose 访问这个订阅
        4. 返回 goose Desktop 后即可开始使用
      </TabItem>

      <TabItem value="tetrate" label="Agent Router">
        对新用户来说，`Agent Router by Tetrate` 是一个很好的起点，因为它可以用内置限流和自动 failover 提供多个模型。

        :::info 免费额度
        第一次通过 goose 自动完成 Tetrate 认证时，你会拿到 10 美元免费额度。新老 Tetrate 用户都适用。
        :::

        1. 选择 `Agent Router by Tetrate`
        2. goose 会打开浏览器，让你登录或创建 Tetrate 账号
        3. 返回 goose Desktop 后即可开始使用
      </TabItem>

      <TabItem value="openrouter" label="OpenRouter">
        1. 选择 `Automatic setup with OpenRouter`
        2. goose 会打开浏览器，让你登录或创建 OpenRouter 账号
        3. 返回 goose Desktop 后即可开始使用
      </TabItem>

      <TabItem value="others" label="Other Providers">
        1. 如果你已经有想用的 Provider 和对应 API Key，选择 `Other Providers`
        2. 在列表里找到目标 Provider 并点击 `Configure`
        3. 如果列表里没有，点击底部的 `Add Custom Provider` 来[配置自定义 Provider](#configure-custom-provider)
        4. 按要求输入 API Key、Host 或其它[参数](#available-providers)，然后点击 `Submit`

        :::info Ollama Model Detection
        对 Ollama 来说，所有本地已安装模型都会自动出现在模型选择下拉里。
        :::
      </TabItem>
    </Tabs>

    **更新 Provider 和 API Key**

    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 点击 `Settings`
    3. 进入 `Models`
    4. 点击 `Configure providers`
    5. 选择你的 Provider
    6. 填写 API Key 和其它必需参数，然后点击 `Submit`

    **切换当前模型**

    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 点击 `Settings`
    3. 进入 `Models`
    4. 点击 `Switch models`
    5. 从已配置 Provider 中选择，或选择 `Use other provider` 新增 Provider
    6. 从列表中选模型，或者使用 `Use custom model` 直接输入模型名
    7. 点击 `Select model`

    :::tip 快速入口
    你也可以点击应用底部当前模型名称，然后选择 `Change Model`。
    :::

    **重置 Provider 和模型配置**

    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 点击 `Settings`
    3. 进入 `Models`
    4. 点击 `Reset Provider and Model`
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    1. 在终端运行：

       ```sh
       goose configure
       ```

    2. 选择 `Configure Providers`

       ```
       ┌   goose-configure
       │
       ◆  What would you like to configure?
       │  ● Configure Providers (Change provider or update credentials)
       │  ○ Custom Providers
       │  ○ Add Extension
       │  ○ Toggle Extensions
       │  ○ Remove Extension
       │  ○ goose Settings
       └
       ```

    3. 选择目标 model provider。可以用方向键移动，也可以直接输入关键词过滤。

       ```
       ┌   goose-configure
       │
       ◇  What would you like to configure?
       │  Configure Providers
       │
       ◆  Which model provider should we use?
       │  ○ Amazon Bedrock
       │  ○ Amazon SageMaker TGI
       │  ● Anthropic (Claude and other models from Anthropic)
       │  ○ Azure OpenAI
       │  ○ Claude Code CLI
       └
       ```

    4. 按提示输入 API Key 及其它配置。

       ```
       ┌   goose-configure
       │
       ◇  What would you like to configure?
       │  Configure Providers
       │
       ◇  Which model provider should we use?
       │  Anthropic
       │
       ◆  Provider Anthropic requires ANTHROPIC_API_KEY, please enter a value
       │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
       └
       ```

       如果你只是切换模型，可以跳过无需更新的配置项。

    5. 输入 `ANTHROPIC_HOST`，或者直接按 `Enter` 使用默认值。

       ```
       ◆  Provider Anthropic requires ANTHROPIC_HOST, please enter a value
       │  https://api.anthropic.com (default)
       ```

    6. 选择你要使用的模型。根据 Provider 不同，可能是：
       - 从列表中选择
       - 输入名称搜索
       - 直接手动输入模型名

       ```
       │
       ◇  Model fetch complete
       │
       ◇  Select a model:
       │  claude-sonnet-4-5 (default)
       │
       ◒  Checking your configuration...
       └  Configuration saved successfully
       ```

    :::note
    `goose configure` 本身不支持直接输入任意自定义模型名。如果目标模型不在列表里，可以改用 goose Desktop，或者直接编辑 [`config.yaml`](/zh-CN/docs/guides/config-files) 中的 `GOOSE_MODEL`。
    :::

    :::tip
    如果只想对单个会话临时切换模型，也可以直接使用 [`run` 命令](/zh-CN/docs/guides/goose-cli-commands) 传入对应参数。
    :::
  </TabItem>
</Tabs>

### 使用自定义 OpenAI Endpoints {#using-custom-openai-endpoints}

内置 `OpenAI` provider 不只支持 OpenAI 官方 API（`api.openai.com`），也能接入任何兼容 OpenAI 协议的 endpoint，例如：

- 通过 vLLM、KServe 暴露的自托管模型
- 企业内部的 OpenAI 兼容 API 服务
- 出于数据治理、安全审计或私有部署要求而搭建的代理层
- 像 LiteLLM、Docker Model Runner 这样的统一网关或本地运行器

:::tip 自定义 Provider 选项
如果你需要同时连接多个 OpenAI 兼容 endpoint，或者想给它们起更清晰的显示名，通常更适合继续往下看 [配置自定义 Provider](#configure-custom-provider)。
:::

#### 配置参数 {#configuration-parameters}

| 参数 | 必填 | 说明 |
|---|---|---|
| `OPENAI_API_KEY` | 是 | 用于认证的 API Key。某些本地服务虽然不校验，但很多代理层仍要求填写一个值 |
| `OPENAI_HOST` | 否 | 自定义 endpoint 地址；留空时默认走 `api.openai.com` |
| `OPENAI_ORGANIZATION` | 否 | 用于 OpenAI 组织级使用追踪与治理 |
| `OPENAI_PROJECT` | 否 | 用于项目级资源管理与审计 |
| `OPENAI_CUSTOM_HEADERS` | 否 | 附加请求头。可通过环境变量、配置文件或 CLI 传入，格式如 `HEADER_A=VALUE_A,HEADER_B=VALUE_B` |

#### 配置示例 {#example-configurations}

<Tabs groupId="deployment">
  <TabItem value="vllm" label="vLLM 自托管" default>
    如果你使用 vLLM 暴露 LLaMA、Mistral 等模型，可按这种方式接入：

    ```sh
    OPENAI_HOST=https://your-vllm-endpoint.internal
    OPENAI_API_KEY=your-internal-api-key
    ```
  </TabItem>

  <TabItem value="kserve" label="KServe 部署">
    对部署在 Kubernetes / KServe 上的模型，可以这样配置：

    ```sh
    OPENAI_HOST=https://kserve-gateway.your-cluster
    OPENAI_API_KEY=your-kserve-api-key
    OPENAI_ORGANIZATION=your-org-id
    OPENAI_PROJECT=ml-serving
    ```
  </TabItem>

  <TabItem value="enterprise" label="企业 OpenAI 网关">
    如果你的企业环境对组织与项目做了统一治理：

    ```sh
    OPENAI_API_KEY=your-api-key
    OPENAI_ORGANIZATION=org-id123
    OPENAI_PROJECT=compliance-approved
    ```
  </TabItem>

  <TabItem value="custom-headers" label="自定义请求头">
    某些 OpenAI 兼容 endpoint 还要求额外请求头：

    ```sh
    OPENAI_API_KEY=your-api-key
    OPENAI_ORGANIZATION=org-id123
    OPENAI_PROJECT=compliance-approved
    OPENAI_CUSTOM_HEADERS="X-Header-A=abc,X-Header-B=def"
    ```
  </TabItem>
</Tabs>

#### 配置步骤 {#setup-instructions}

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 点击 `Settings`
    3. 进入 `Models`
    4. 点击 `Configure providers`
    5. 在 Provider 列表中选择 `OpenAI`
    6. 填入所需配置：
       - API Key（必填）
       - Host URL（自定义 endpoint 时填写）
       - Organization ID（需要组织级追踪时填写）
       - Project（需要项目级治理时填写）
    7. 点击 `Submit`
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    1. 运行 `goose configure`
    2. 选择 `Configure Providers`
    3. 选择 `OpenAI`
    4. 按提示输入：
       - API Key
       - Host URL（如果使用自定义 endpoint）
       - Organization ID（如果需要组织追踪）
       - Project（如果需要项目管理）
  </TabItem>
</Tabs>

:::tip 企业部署
在团队环境里，通常可以通过环境变量或共享配置文件预置这些参数，确保所有 session 都遵循同一套安全和治理要求。
:::

## 配置自定义 Provider {#configure-custom-provider}

如果官方列表里没有你要接的服务，或者你想定制连接方式，可以创建自定义 Provider。创建完成后，它会像内置 Provider 一样出现在 provider 列表里。

**典型收益：**

- **多 endpoint 管理**：例如在 vLLM、企业代理和官方 OpenAI 之间快速切换
- **预设模型列表**：把常用模型名固化下来，减少手输
- **可共享配置**：JSON 文件可以放进仓库，方便团队复用
- **自定义展示名**：例如在界面里显示成 “Corporate API”
- **隔离凭据**：每个 provider 单独使用自己的 key

自定义 provider 目前要求底层 API 兼容 OpenAI、Anthropic 或 Ollama。你也可以附加自定义请求头，用于租户标识、额外 token 或内部认证字段。

### 添加自定义 Provider

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 点击 `Settings`
    3. 进入 `Models`
    4. 点击 `Configure providers`
    5. 点击窗口底部的 `Add Custom Provider`
    6. 逐项填写：
       - **Provider Type**
         - `OpenAI Compatible`（最常见）
         - `Anthropic Compatible`
         - `Ollama Compatible`
       - **Display Name**：显示给用户看的名称
       - **API URL**：服务的基础地址
       - **Authentication**
         - 默认通过自定义环境变量读取 API Key，并存入 keychain
         - 如果目标服务不要求授权（例如本地模型或内网服务），取消勾选 **This provider requires an API key**
       - **Available Models**：逗号分隔的模型列表
       - **Streaming Support**：是否支持流式返回
    7. 点击 `Create Provider`

    :::info 自定义请求头
    目前 goose Desktop 还不能直接填写 custom headers。需要的话，先创建 provider，再手动编辑配置文件补上。
    :::
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    1. 运行：

       ```sh
       goose configure
       ```

    2. 选择 `Custom Providers`

       ```sh
       ┌   goose-configure
       │
       ◆  What would you like to configure?
       │  ○ Configure Providers
       │  ● Custom Providers (Add custom provider with compatible API)
       │  ○ Add Extension
       │  ○ Toggle Extensions
       │  ○ Remove Extension
       │  ○ goose Settings
       └
       ```

    3. 选择 `Add A Custom Provider`
    4. 按提示填写：
       - **API Type**
         - `OpenAI Compatible`
         - `Anthropic Compatible`
         - `Ollama Compatible`
       - **Name**
       - **API URL**
       - **Authentication Required**
         - 选 `Yes` 时继续输入 API Key
         - 选 `No` 时跳过 key
       - **Available Models**
       - **Streaming Support**
       - **Custom Headers**

    :::info 自定义请求头
    CLI 目前只支持给 OpenAI-compatible provider 直接配置 custom headers。Anthropic / Ollama compatible provider 仍建议创建后手工编辑配置文件。
    :::
  </TabItem>

  <TabItem value="config" label="配置文件">
    先在 `custom_providers` 目录下创建一个 JSON 文件：

    - macOS / Linux：`~/.config/goose/custom_providers/`
    - Windows：`%APPDATA%\\Block\\goose\\config\\custom_providers\\`

    示例 `custom_corp_api.json`：

    ```json
    {
      "name": "custom_corp_api",
      "engine": "openai",
      "display_name": "Corporate API",
      "description": "Custom Corporate API provider",
      "api_key_env": "CUSTOM_CORP_API_API_KEY",
      "base_url": "https://api.company.com/v1/chat/completions",
      "models": [
        {
          "name": "gpt-4o",
          "context_limit": 128000
        },
        {
          "name": "gpt-3.5-turbo",
          "context_limit": 16385
        }
      ],
      "headers": {
        "x-origin-client-id": "YOUR_CLIENT_ID",
        "x-origin-secret": "YOUR_SECRET_VALUE"
      },
      "supports_streaming": true,
      "requires_auth": true
    }
    ```

    然后通过 `api_key_env` 为当前 session 注入密钥，例如：

    ```bash
    export CUSTOM_CORP_API_API_KEY="your-api-key"
    goose session start --provider custom_corp_api
    ```

    :::tip Keychain 存储
    如果你希望 key 落到 goose keychain 里，可以先在 Desktop 中创建或更新这个 provider，并在界面里输入 API Key。
    :::
  </TabItem>
</Tabs>

### 更新自定义 Provider

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 点击 `Settings`
    3. 进入 `Models`
    4. 点击 `Configure providers`
    5. 在列表中点击你的自定义 provider
    6. 修改要更新的字段
    7. 点击 `Update Provider`
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    1. 运行：

       ```sh
       goose configure
       ```

    2. 选择 `Configure Providers`

       ```sh
       ┌   goose-configure
       │
       ◆  What would you like to configure?
       │  ● Configure Providers (Change provider or update credentials)
       │  ○ Custom Providers
       │  ○ Add Extension
       │  ○ Toggle Extensions
       │  ○ Remove Extension
       │  ○ goose Settings
       └
       ```

    3. 从 provider 列表里选中你的自定义 provider。可以用方向键移动，也可以直接输入关键词过滤。

       ```sh
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
       │  ○ Claude Code CLI
       │  ● Corporate API (Custom Corporate API provider)
       │  ○ Cursor Agent
       │  ○ ...
       └
       ```

    4. 按提示更新需要修改的字段
  </TabItem>

  <TabItem value="config" label="配置文件">
    打开 `custom_providers` 目录中的对应 JSON 文件并直接修改：

    - macOS / Linux：`~/.config/goose/custom_providers/`
    - Windows：`%APPDATA%\\Block\\goose\\config\\custom_providers\\`

    保存后，新配置会在下一次 goose session 中生效。
  </TabItem>
</Tabs>

### 删除自定义 Provider

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 点击 `Settings`
    3. 进入 `Models`
    4. 点击 `Configure providers`
    5. 选择要删除的自定义 provider
    6. 点击 `Delete Provider`
    7. 确认删除 provider 及其关联的 API Key
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    1. 运行：

       ```sh
       goose configure
       ```

    2. 选择 `Custom Providers`

       ```sh
       ┌   goose-configure
       │
       ◆  What would you like to configure?
       │  ○ Configure Providers
       │  ● Custom Providers (Add custom provider with compatible API)
       │  ○ Add Extension
       │  ○ Toggle Extensions
       │  ○ Remove Extension
       │  ○ goose Settings
       └
       ```

    3. 选择 `Remove Custom Provider`

       ```sh
       ┌   goose-configure
       │
       ◇  What would you like to configure?
       │  Custom Providers
       │
       ◆  What would you like to do?
       │  ○ Add A Custom Provider
       │  ● Remove Custom Provider (Remove an existing custom provider)
       └
       ```

    4. 选择要移除的 provider

    对应的配置文件会从 `custom_providers` 目录删除；如果 API Key 存在 keychain 中，也会一并清理。
  </TabItem>

  <TabItem value="config" label="配置文件">
    :::tip
    如果 API Key 已存入 keychain，更推荐用 goose CLI 删除，这样可以把配置文件和密钥一起清干净。
    :::

    直接删除 `custom_providers` 目录下对应的 JSON 文件：

    - macOS / Linux：`~/.config/goose/custom_providers/`
    - Windows：`%APPDATA%\\Block\\goose\\config\\custom_providers\\`
  </TabItem>
</Tabs>

## 免费开始使用 goose {#using-goose-for-free}

goose 本身是免费开源的，但不是所有 [LLM Providers][providers] 都有免费额度。下面列出几个常见的免费起步方案。

:::warning 限制
这些免费方案很适合先上手，但如果你要处理更复杂、更长上下文或更多工具调用任务，后续通常还是需要升级到更强的模型。
:::

### Groq

Groq 提供了若干开源模型的免费高速推理能力。要在 goose 中使用 Groq，需要先到 [Groq Console](https://console.groq.com/keys) 获取 API Key。

Groq 当前常见可用模型包括：

- **`moonshotai/kimi-k2-instruct-0905`**：MoE 架构，参数规模非常大，适合更强的 agentic reasoning 和工具使用
- **`qwen/qwen3-32b`**：32.8B 参数，多语言和推理能力比较均衡
- **`llama-3.3-70b-versatile`**：Meta 的 Llama 3.3 通用模型，适合覆盖较广的任务场景
- **`llama-3.1-8b-instant`**：更偏向低延迟推理，适合快速响应

完整支持列表可参考 [groq.json](https://github.com/block/goose/blob/main/crates/goose/src/providers/declarative/groq.json)。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 点击 `Settings`
    3. 进入 `Models`
    4. 点击 `Configure Providers`
    5. 选择 `Groq`
    6. 点击 `Configure`，输入 API Key 并提交
    7. 选择想用的 Groq 模型
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    1. 运行：

       ```sh
       goose configure
       ```

    2. 选择 `Configure Providers`
    3. 按提示选择 `Groq`
    4. 输入 API Key
    5. 选择模型
  </TabItem>
</Tabs>

### Google Gemini

Google Gemini 提供免费额度。你需要先到 [Google AI Studio](https://aistudio.google.com/app/apikey) 获取 API Key。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 点击 `Settings`
    3. 进入 `Models`
    4. 点击 `Configure Providers`
    5. 选择 `Google Gemini`
    6. 点击 `Configure`，输入 API Key 并提交
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    1. 运行：

       ```sh
       goose configure
       ```

    2. 选择 `Configure Providers`
    3. 选择 `Google Gemini`
    4. 输入 `GOOGLE_API_KEY`
    5. 输入你要使用的 Gemini 模型名

       ```
       ┌   goose-configure
       │
       ◇ What would you like to configure?
       │ Configure Providers
       │
       ◇ Which model provider should we use?
       │ Google Gemini
       │
       ◇ Provider Google Gemini requires GOOGLE_API_KEY, please enter a value
       │ ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
       │
       ◇ Enter a model from that provider:
       │ gemini-2.0-flash-exp
       │
       └ Configuration saved successfully
       ```
  </TabItem>
</Tabs>

### 本地 LLMs {#local-llms}

goose 本身就是本地 AI agent。配合本地 LLM，你可以把数据保留在本机，完全离线工作，并保留对运行环境的完全控制。但相应地，前置准备也会更多一些。

:::warning 不支持 tool calling 的模型能力有限
goose 高度依赖 tool calling。如果模型不支持 tool calling，它基本只能做聊天补全。这种情况下，你需要关闭所有 goose [扩展](/zh-CN/docs/getting-started/using-extensions)。
:::

当前常见的本地 provider 路线包括：

<Tabs groupId="local-llms">
  <TabItem value="ollama" label="Ollama" default>
    <Tabs groupId="ollama-models">
      <TabItem value="ramalama" label="Ramalama" default>
        1. [安装 Ramalama](https://github.com/containers/ramalama?tab=readme-ov-file#install)
        2. 在终端中启动一个支持 tool calling 的 Ollama 模型，或者一个支持对应格式的 GGUF 模型

           `--runtime-args="--jinja"` 是 goose 的 Ollama provider 能正确配合 Ramalama 所必需的。

           示例：

           ```sh
           ramalama serve --runtime-args="--jinja" ollama://qwen2.5
           ```

        3. 在另一个终端里运行：

           ```sh
           goose configure
           ```

        4. 选择 `Configure Providers`

           ```
           ┌   goose-configure
           │
           ◆  What would you like to configure?
           │  ● Configure Providers (Change provider or update credentials)
           │  ○ Toggle Extensions
           │  ○ Add Extension
           └
           ```

        5. 选择 `Ollama` 作为 model provider，因为 Ramalama 的接口兼容 goose 的 Ollama provider

           ```
           ┌   goose-configure
           │
           ◇  What would you like to configure?
           │  Configure Providers
           │
           ◆  Which model provider should we use?
           │  ○ Anthropic
           │  ○ Databricks
           │  ○ Google Gemini
           │  ○ Groq
           │  ● Ollama (Local open source models)
           │  ○ OpenAI
           │  ○ OpenRouter
           └
           ```

        6. 输入模型运行地址

           :::info Endpoint
           如果你不提供 host，Ollama provider 默认使用 `localhost:11434`。如果没有写 `http://` 或 `https://`，goose 会自动补 `http://`。Ramalama 默认端口通常是 `8080`，因此常见值是 `OLLAMA_HOST=http://0.0.0.0:8080`。
           :::

           ```
           ┌   goose-configure
           │
           ◇  What would you like to configure?
           │  Configure Providers
           │
           ◇  Which model provider should we use?
           │  Ollama
           │
           ◆  Provider Ollama requires OLLAMA_HOST, please enter a value
           │  http://0.0.0.0:8080
           └
           ```

        7. 输入正在运行的模型名，例如 `qwen2.5`

           ```
           ┌   goose-configure
           │
           ◇  What would you like to configure?
           │  Configure Providers
           │
           ◇  Which model provider should we use?
           │  Ollama
           │
           ◇  Provider Ollama requires OLLAMA_HOST, please enter a value
           │  http://0.0.0.0:8080
           │
           ◇  Enter a model from that provider:
           │  qwen2.5
           │
           ◇  Welcome! You're all set to explore and utilize my capabilities. Let's get started on solving your problems together!
           │
           └  Configuration saved successfully
           ```

           :::tip Context Length
           如果 goose 开始忽略 [.goosehints](/zh-CN/docs/guides/context-engineering/using-goosehints) 或扩展调用异常，往往是上下文长度太小。Ramalama 可以通过 `--ctx-size` / `-c` 提高上下文窗口。
           :::
      </TabItem>

      <TabItem value="deepseek" label="DeepSeek-R1">
        原生 `DeepSeek-r1` 不支持 tool calling，但你可以使用专门适配 goose 的[自定义模型](https://ollama.com/michaelneale/deepseek-r1-goose)。

        :::warning
        这是一个 70B 级别模型，对硬件要求较高。
        :::

        1. [安装 Ollama](https://ollama.com/download)
        2. 在终端运行：

           ```sh
           ollama run michaelneale/deepseek-r1-goose
           ```

        3. 在另一个终端运行：

           ```sh
           goose configure
           ```

        4. 选择 `Configure Providers`

           ```
           ┌   goose-configure
           │
           ◆  What would you like to configure?
           │  ● Configure Providers (Change provider or update credentials)
           │  ○ Toggle Extensions
           │  ○ Add Extension
           └
           ```

        5. 选择 `Ollama`

           ```sh
           ┌   goose-configure
           │
           ◇  What would you like to configure?
           │  Configure Providers
           │
           ◆  Which model provider should we use?
           │  ○ Anthropic
           │  ○ Databricks
           │  ○ Google Gemini
           │  ○ Groq
           │  ● Ollama (Local open source models)
           │  ○ OpenAI
           │  ○ OpenRouter
           └
           ```

        6. 输入 `OLLAMA_HOST`，通常是 `http://localhost:11434`

           ```sh
           ┌   goose-configure
           │
           ◇  What would you like to configure?
           │  Configure Providers
           │
           ◇  Which model provider should we use?
           │  Ollama
           │
           ◆  Provider Ollama requires OLLAMA_HOST, please enter a value
           │  http://localhost:11434
           └
           ```

        7. 输入模型名：`michaelneale/deepseek-r1-goose`

           ```
           ┌   goose-configure
           │
           ◇  What would you like to configure?
           │  Configure Providers
           │
           ◇  Which model provider should we use?
           │  Ollama
           │
           ◇   Provider Ollama requires OLLAMA_HOST, please enter a value
           │  http://localhost:11434
           │
           ◇  Enter a model from that provider:
           │  michaelneale/deepseek-r1-goose
           │
           ◇  Welcome! You're all set to explore and utilize my capabilities. Let's get started on solving your problems together!
           │
           └  Configuration saved successfully
           ```
      </TabItem>

      <TabItem value="others" label="Other Models">
        1. [安装 Ollama](https://ollama.com/download)
        2. 在终端运行任意支持 tool calling 的模型，例如：

           ```sh
           ollama run qwen2.5
           ```

        3. 在另一个终端运行：

           ```sh
           goose configure
           ```

        4. 选择 `Configure Providers`

           ```
           ┌   goose-configure
           │
           ◆  What would you like to configure?
           │  ● Configure Providers (Change provider or update credentials)
           │  ○ Toggle Extensions
           │  ○ Add Extension
           └
           ```

        5. 选择 `Ollama`

           ```sh
           ┌   goose-configure
           │
           ◇  What would you like to configure?
           │  Configure Providers
           │
           ◆  Which model provider should we use?
           │  ○ Anthropic
           │  ○ Databricks
           │  ○ Google Gemini
           │  ○ Groq
           │  ● Ollama (Local open source models)
           │  ○ OpenAI
           │  ○ OpenRouter
           └
           ```

        6. 输入模型地址，默认通常是 `http://localhost:11434`

           :::info Endpoint
           对 Ollama 来说，如果你不提供 host，默认会使用 `localhost:11434`。如果你把 Ollama 跑在别的机器上，则需要把 `OLLAMA_HOST` 改成 `http://{host}:{port}`。
           :::

           ```sh
           ┌   goose-configure
           │
           ◇  What would you like to configure?
           │  Configure Providers
           │
           ◇  Which model provider should we use?
           │  Ollama
           │
           ◆  Provider Ollama requires OLLAMA_HOST, please enter a value
           │  http://localhost:11434
           └
           ```

        7. 输入当前运行的模型名，例如 `qwen2.5`

           ```sh
           ┌   goose-configure
           │
           ◇  What would you like to configure?
           │  Configure Providers
           │
           ◇  Which model provider should we use?
           │  Ollama
           │
           ◇  Provider Ollama requires OLLAMA_HOST, please enter a value
           │  http://localhost:11434
           │
           ◇  Enter a model from that provider:
           │  qwen2.5
           │
           ◇  Welcome! You're all set to explore and utilize my capabilities. Let's get started on solving your problems together!
           │
           └  Configuration saved successfully
           ```

        :::tip Context Length
        如果模型默认上下文太小，goose 可能无法稳定使用扩展或读取 `.goosehints`。对 Ollama，可以用 `OLLAMA_CONTEXT_LENGTH` 提高上下文窗口。
        :::
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="lmstudio" label="LM Studio">
    [LM Studio](https://lmstudio.ai/) 可以在本地运行开源模型，并通过 OpenAI 兼容接口提供服务。

    1. 安装 LM Studio
    2. 下载一个支持 tool calling 的模型（例如 Qwen、Llama 或 Mistral 变体）
    3. 启动 LM Studio 的本地服务，默认地址通常是 `http://localhost:1234`

    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
        2. 点击 `Settings`
        3. 进入 `Models`
        4. 点击 `Configure providers`
        5. 选择 `LM Studio`
        6. 点击 `Submit`（通常不需要 API Key）
        7. 选择你在 LM Studio 中加载的模型
      </TabItem>

      <TabItem value="cli" label="goose CLI">
        1. 运行：

           ```sh
           goose configure
           ```

        2. 选择 `Configure Providers`
        3. 选择 `LM Studio`
        4. 输入与你在 LM Studio server 面板里看到的一致模型名，例如：

           ```
           ┌   goose-configure
           │
           ◇  What would you like to configure?
           │  Configure Providers
           │
           ◇  Which model provider should we use?
           │  LM Studio
           │
           ◇  Enter a model from that provider:
           │  qwen2.5-7b-instruct
           │
           └  Configuration saved successfully
           ```
      </TabItem>
    </Tabs>

    :::tip 模型名
    在 goose 里输入的模型名，要和 LM Studio server 面板中显示的标识一致。
    :::
  </TabItem>

  <TabItem value="docker" label="Docker Model Runner">
    1. [安装 Docker](https://docs.docker.com/get-started/get-docker/)
    2. [启用 Docker Model Runner](https://docs.docker.com/ai/model-runner/#enable-dmr-in-docker-desktop)
    3. [拉取模型](https://docs.docker.com/ai/model-runner/#pull-a-model)，例如：

       ```sh
       docker model pull hf.co/unsloth/gemma-3n-e4b-it-gguf:q6_k
       ```

    4. 运行：

       ```sh
       goose configure
       ```

    5. 选择 `Configure Providers`
    6. 选择 `OpenAI`，因为 Docker Model Runner 提供的是 OpenAI 兼容 endpoint

       ```
       ┌   goose-configure
       │
       ◇  What would you like to configure?
       │  Configure Providers
       │
       ◆  Which model provider should we use?
       │  ○ Anthropic
       │  ○ Amazon Bedrock
       │  ○ Claude Code
       │  ● OpenAI (GPT-4 and other OpenAI models, including OpenAI compatible ones)
       │  ○ OpenRouter
       └
       ```

    7. 把 `OPENAI_HOST` 改成 Docker Model Runner 的地址，例如 `http://localhost:12434`

       ```
       ◆  Provider OpenAI requires OPENAI_HOST, please enter a value
       │  https://api.openai.com (default)
       └
       ```

       Docker Model Runner 在宿主机上的默认端口通常是 `12434`，所以常见值就是 `http://localhost:12434`。

    8. 把 `OPENAI_BASE_PATH` 设置成对应路径。Docker Model Runner 使用 `/engines/llama.cpp/v1/chat/completions`

       ```
       ◆  Provider OpenAI requires OPENAI_BASE_PATH, please enter a value
       │  v1/chat/completions (default)
       └
       ```

    9. 最后输入要使用的模型名，例如 `hf.co/unsloth/gemma-3n-e4b-it-gguf:q6_k`

       ```
       │
       ◇  Enter a model from that provider:
       │  gpt-4o
       │
       ◒  Checking your configuration...
       └  Configuration saved successfully
       ```
  </TabItem>
</Tabs>

## GitHub Copilot 认证 {#github-copilot-authentication}

GitHub Copilot 使用 device flow，不需要手工 API Key：

1. 运行 [配置 Provider 和模型](#configure-provider-and-model) 中的 `goose configure`
2. 选择 **GitHub Copilot**
3. 一个 8 位验证码会自动复制到剪贴板
4. 浏览器会打开 GitHub 的设备激活页面
5. 粘贴验证码完成授权
6. 返回 goose 后，GitHub Copilot 就会在 CLI 和 Desktop 中都可用

## Azure OpenAI Credential Chain {#azure-openai-credential-chain}

goose 对 Azure OpenAI 支持两种认证方式：

1. **API Key Authentication**：使用 `AZURE_OPENAI_API_KEY`
2. **Azure Credential Chain**：自动使用 Azure CLI 登录态，不必显式提供 API Key

如果你要使用 Azure Credential Chain：

- 先执行 `az login`
- 确认当前账号对 Azure OpenAI 服务有足够权限
- 在 `goose configure` 中选择 Azure OpenAI，并把 API Key 留空

对企业环境来说，这种方式通常更安全，也更容易统一管理。

## 多模型配置 {#multi-model-configuration}

除了单模型方案，goose 还支持[多模型配置](/zh-CN/docs/guides/multi-model)，让不同模型承担不同角色：

- **Lead/Worker Model**：前几轮用更强模型做规划，后续用更快或更便宜的模型做执行
- **Planning Mode**：先用专门模型做任务拆解，再进入执行阶段

## Gemini 3 Thinking Levels {#gemini-3-thinking-levels}

Gemini 3 模型支持不同的 thinking level，用来平衡延迟与推理深度：

- **Low**（默认）：更快，推理更轻量
- **High**：推理更深，但延迟更高

:::tip
开启 thinking 后，你可以查看模型推理过程。详见 [Viewing Model Reasoning](#viewing-model-reasoning)。
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    在 Desktop 中选择 Gemini 3 模型时，会自动出现 “Thinking Level” 下拉框。你选定后，该设置会跨会话保留。
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    当你在 `goose configure` 里选择 Gemini 3 模型时，会看到：

    ```
    ◆  Select thinking level for Gemini 3:
    │  ● Low - Better latency, lighter reasoning
    │  ○ High - Deeper reasoning, higher latency
    ```
  </TabItem>
</Tabs>

:::info 优先级
thinking level 的优先级从高到低依次是：
1. 模型配置中的 `request_params.thinking_level`（通过 `GOOSE_PREDEFINED_MODELS`）
2. `GEMINI3_THINKING_LEVEL` 环境变量
3. 默认值 `low`
:::

## 查看模型推理过程 {#viewing-model-reasoning}

有些模型会把内部推理或 reasoning 作为响应的一部分暴露出来。goose 会自动捕获这些内容，并在适当位置展示。

当前常见支持情况：

| Provider / Model | 工作方式 |
|---|---|
| **DeepSeek-R1**（通过 OpenAI、Ollama、OpenRouter、OVHcloud 等） | 从响应里的 `reasoning_content` 字段提取 |
| **Kimi**（通过 Groq 或其它 OpenAI 兼容端点） | 从 `reasoning_content` 字段提取 |
| **Gemini CLI**（开启 thinking 的 Gemini） | 从流式响应中的 thinking block 捕获 |
| **Claude**（Anthropic，并开启 [extended thinking](/zh-CN/docs/guides/environment-variables)） | 从 API 响应中的 thinking block 捕获 |

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    在 Desktop 中，reasoning 会出现在模型回答上方的可折叠区域，通常显示为 **Show reasoning**。
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    CLI 默认隐藏 reasoning。若要显示，请设置：

    ```bash
    export GOOSE_CLI_SHOW_THINKING=1
    ```

    开启后，reasoning 会在主回答前以较暗文本显示。

    :::note
    这要求 stdout 必须是终端。如果你把输出 pipe 到文件或其它命令，reasoning 通常不会显示。
    :::
  </TabItem>
</Tabs>

:::tip
查看 reasoning 有助于理解模型为什么给出某个答案，也适合用于调试异常行为或学习模型的拆题方式。但它通常比较冗长，只在需要时开启即可。
:::

---

如果你在某个 Provider 上遇到问题，可以到 [Discord](https://discord.gg/goose-oss) 或 [goose repo](https://github.com/block/goose) 继续反馈。

[providers]: /zh-CN/docs/getting-started/providers
[function-calling-leaderboard]: https://gorilla.cs.berkeley.edu/leaderboard.html
