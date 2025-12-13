---
sidebar_position: 2
title: Configure LLM Provider
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft } from 'lucide-react';
import { ModelSelectionTip } from '@site/src/components/ModelSelectionTip';

# Supported LLM Providers

goose is compatible with a wide range of LLM providers, allowing you to choose and integrate your preferred model.

:::tip Model Selection
<ModelSelectionTip/>
[Berkeley Function-Calling Leaderboard][function-calling-leaderboard] can be a good guide for selecting models.
:::

## Available Providers

| Provider                                                                    | Description                                                                                                                                                                                                               | Parameters                                                                                                                                                                          |
|-----------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| [Amazon Bedrock](https://aws.amazon.com/bedrock/)                           | Offers a variety of foundation models, including Claude, Jurassic-2, and others. **AWS environment variables must be set in advance, not configured through `goose configure`**                                           | `AWS_PROFILE`, or `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`                                                                                                 |
| [Amazon SageMaker TGI](https://docs.aws.amazon.com/sagemaker/latest/dg/realtime-endpoints.html) | Run Text Generation Inference models through Amazon SageMaker endpoints. **AWS credentials must be configured in advance.** | `SAGEMAKER_ENDPOINT_NAME`, `AWS_REGION` (optional), `AWS_PROFILE` (optional)  |
| [Anthropic](https://www.anthropic.com/)                                     | Offers Claude, an advanced AI model for natural language tasks.                                                                                                                                                           | `ANTHROPIC_API_KEY`, `ANTHROPIC_HOST` (optional)                                                                                                                                                                 |
| [Azure OpenAI](https://learn.microsoft.com/en-us/azure/ai-services/openai/) | Access Azure-hosted OpenAI models, including GPT-4 and GPT-3.5. Supports both API key and Azure credential chain authentication.                                                                                          | `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_DEPLOYMENT_NAME`, `AZURE_OPENAI_API_KEY` (optional)                                                                                           |
| [Databricks](https://www.databricks.com/)                                   | Unified data analytics and AI platform for building and deploying models.                                                                                                                                                 | `DATABRICKS_HOST`, `DATABRICKS_TOKEN` |
| [Docker Model Runner](https://docs.docker.com/ai/model-runner/)                             | Local models running in Docker Desktop or Docker CE with OpenAI-compatible API endpoints. **Because this provider runs locally, you must first [download a model](#local-llms).**                     | `OPENAI_HOST`, `OPENAI_BASE_PATH`   |
| [Gemini](https://ai.google.dev/gemini-api/docs)                             | Advanced LLMs by Google with multimodal capabilities (text, images).                                                                                                                                                      | `GOOGLE_API_KEY`                                                                                                                                                                    |
| [GCP Vertex AI](https://cloud.google.com/vertex-ai)                         | Google Cloud's Vertex AI platform, supporting Gemini and Claude models. **Credentials must be [configured in advance](https://cloud.google.com/vertex-ai/docs/authentication).**                 | `GCP_PROJECT_ID`, `GCP_LOCATION` and optionally `GCP_MAX_RATE_LIMIT_RETRIES` (5), `GCP_MAX_OVERLOADED_RETRIES` (5), `GCP_INITIAL_RETRY_INTERVAL_MS` (5000), `GCP_BACKOFF_MULTIPLIER` (2.0), `GCP_MAX_RETRY_INTERVAL_MS` (320_000). |
| [GitHub Copilot](https://docs.github.com/en/copilot/using-github-copilot/ai-models) | Access to AI models from OpenAI, Anthropic, Google, and other providers through GitHub's Copilot infrastructure. **GitHub account with Copilot access required.** | No manual key. Must configure through the CLI using the GitHub authentication flow to enable both CLI and Desktop access. |
| [Groq](https://groq.com/)                                                   | High-performance inference hardware and tools for LLMs.                                                                                                                                                                   | `GROQ_API_KEY`                                                                                                                                                                      |
| [LiteLLM](https://docs.litellm.ai/docs/) | LiteLLM proxy supporting multiple models with automatic prompt caching and unified API access. | `LITELLM_HOST`, `LITELLM_BASE_PATH` (optional), `LITELLM_API_KEY` (optional), `LITELLM_CUSTOM_HEADERS` (optional), `LITELLM_TIMEOUT` (optional) |
| [Mistral AI](https://mistral.ai/)                                           | Provides access to Mistral models including general-purpose models, specialized coding models (Codestral), and multimodal models (Pixtral).                                                                   | `MISTRAL_API_KEY`                                                                                                 |
| [Ollama](https://ollama.com/)                                               | Local model runner supporting Qwen, Llama, DeepSeek, and other open-source models. **Because this provider runs locally, you must first [download and run a model](#local-llms).**  | `OLLAMA_HOST`                                                                                                                                                                       |
| [Ramalama](https://ramalama.ai/)                                            | Local model using native [OCI](https://opencontainers.org/) container runtimes, [CNCF](https://www.cncf.io/) tools, and supporting models as OCI artifacts. Ramalama API an compatible alternative to Ollama and can be used with the goose Ollama provider. Supports Qwen, Llama, DeepSeek, and other open-source models. **Because this provider runs locally, you must first [download and run a model](#local-llms).**  | `OLLAMA_HOST`                                                                                                                                                                       |
| [OpenAI](https://platform.openai.com/api-keys)                              | Provides gpt-4o, o1, and other advanced language models. Also supports OpenAI-compatible endpoints (e.g., self-hosted LLaMA, vLLM, KServe). **o1-mini and o1-preview are not supported because goose uses tool calling.** | `OPENAI_API_KEY`, `OPENAI_HOST` (optional), `OPENAI_ORGANIZATION` (optional), `OPENAI_PROJECT` (optional), `OPENAI_CUSTOM_HEADERS` (optional)                                       |
| [OpenRouter](https://openrouter.ai/)                                        | API gateway for unified access to various models with features like rate-limiting management.                                                                                                                             | `OPENROUTER_API_KEY`                                                                                                                                                                |
| [Snowflake](https://docs.snowflake.com/user-guide/snowflake-cortex/aisql#choosing-a-model) | Access the latest models using Snowflake Cortex services, including Claude models. **Requires a Snowflake account and programmatic access token (PAT)**.                                                     | `SNOWFLAKE_HOST`, `SNOWFLAKE_TOKEN`                                                                                                                                                                 |
| [Tetrate Agent Router Service](https://router.tetrate.ai)                   | Unified API gateway for AI models including Claude, Gemini, GPT, open-weight models, and others. Supports PKCE authentication flow for secure API key generation.                                                                                | `TETRATE_API_KEY`, `TETRATE_HOST` (optional)                                                                                                                                        |
| [Venice AI](https://venice.ai/home)                                         | Provides access to open source models like Llama, Mistral, and Qwen while prioritizing user privacy. **Requires an account and an [API key](https://docs.venice.ai/overview/guides/generating-api-key)**.                 | `VENICE_API_KEY`, `VENICE_HOST` (optional), `VENICE_BASE_PATH` (optional), `VENICE_MODELS_PATH` (optional)                                                                          |
| [xAI](https://x.ai/)                                                        | Access to xAI's Grok models including grok-3, grok-3-mini, and grok-3-fast with 131,072 token context window.                                                                                                            | `XAI_API_KEY`, `XAI_HOST` (optional)                                                                                                                                                |

### CLI Providers

goose also supports special "pass-through" providers that work with existing CLI tools, allowing you to use your subscriptions instead of paying per token:

| Provider                                                                    | Description                                                                                                                                                                                                               | Requirements                                                                                                                                                                          |
|-----------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| [Claude Code](https://www.anthropic.com/claude-code) (`claude-code`)                       | Uses Anthropic's Claude CLI tool with your Claude Code subscription. Provides access to Claude with 200K context limit.                                                                                      | Claude CLI installed and authenticated, active Claude Code subscription                                                                                                              |
| [Cursor Agent](https://docs.cursor.com/en/cli/overview) (`cursor-agent`)   | Uses Cursor's AI CLI tool with your Cursor subscription. Provides access to GPT-5, Claude 4, and other models through the cursor-agent command-line interface.                                              | cursor-agent CLI installed and authenticated                                                                                                         |
| [Gemini CLI](https://ai.google.dev/gemini-api/docs) (`gemini-cli`)         | Uses Google's Gemini CLI tool with your Google AI subscription. Provides access to Gemini with 1M context limit.                                                                                               | Gemini CLI installed and authenticated                                                                                                                |

:::tip CLI Providers
CLI providers are cost-effective alternatives that use your existing subscriptions. They work differently from API providers as they execute CLI commands and integrate with the tools' native capabilities. See the [CLI Providers guide](/docs/guides/cli-providers) for detailed setup instructions.
:::

## Configure Provider and Model

To configure your chosen provider, see available options, or select a model, visit the `Models` tab in goose Desktop or run `goose configure` in the CLI.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  **First-time users:**
  
  On the welcome screen the first time you open goose, you have three options:
  - **Automatic setup with [Tetrate Agent Router](https://tetrate.io/products/tetrate-agent-router-service)**
  - **Automatic Setup with [OpenRouter](https://openrouter.ai/)**
  - **Other Providers**
  <Tabs groupId="setup">
    <TabItem value="tetrate" label="Tetrate Agent Router" default>
    We recommend starting with Tetrate Agent Router. Tetrate provides access to multiple AI models with built-in rate limiting and automatic failover. 

    :::info Free Credits Offer
    You'll receive $10 in free credits the first time you automatically authenticate with Tetrate through goose. This offer is available to both new and existing Tetrate users.
    :::
    1. Choose `Automatic setup with Tetrate Agent Router`. 
    2. goose will open a browser window for you to authenticate with Tetrate, or create a new account if you don't have one already.
    3. When you return to the goose desktop app, you're ready to begin your first session.
    </TabItem>

    <TabItem value="openrouter" label="OpenRouter">
    1. Choose `Automatic setup with OpenRouter`. 
    2. goose will open a browser window for you to authenticate with OpenRouter, or create a new account if you don't have one already.
    3. When you return to the goose desktop app, you're ready to begin your first session.
    </TabItem>

    <TabItem value="others" label="Other Providers">
    1. If you have a specific provider you want to use with goose, and an API key from that provider, choose `Other Providers`. 
    2. Find the provider of your choice and click its `Configure` button. If you don't see your provider in the list, click `Add Custom Provider` at the bottom of the window to [configure a custom provider](#configure-custom-provider). 
    3. Depending on your provider, you'll need to input your API Key, API Host, or other optional [parameters](#available-providers). Click the `Submit` button to authenticate and begin your first session.

    :::info Ollama Model Detection
    For Ollama users, all locally installed models display automatically in the model selection dropdown.
    :::
    
    </TabItem>
  </Tabs>
  **To update your LLM provider and API key:** 
  1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
  2. Click the `Settings` button on the sidebar
  3. Click the `Models` tab
  4. Click `Configure providers`
  5. Click your provider in the list
  6. Add your API key and other required configurations, then click `Submit`

  **To change your current model:**
  1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
  2. Click the `Settings` button on the sidebar
  3. Click the `Models` tab
  4. Click `Switch models`
  5. Choose from your configured providers in the dropdown, or select `Use other provider` to configure a new one
  6. Select a model from the available options, or choose `Use custom model` to enter a specific model name
  7. Click `Select model` to confirm your choice

  :::tip Shortcut
  For faster access, click your current model name at the bottom of the app and choose `Change Model`.
  :::

  **To start over with provider and model configuration:**
  1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
  2. Click the `Settings` button on the sidebar
  3. Click the `Models` tab
  4. Click `Reset Provider and Model` to clear your current settings and return to the welcome screen
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    1. In your terminal, run the following command: 

       ```sh
       goose configure
       ```

    2. Select `Configure Providers` from the menu and press `Enter`.

       ```
       ┌   goose-configure 
       │
       ◆  What would you like to configure?
       // highlight-start
       │  ● Configure Providers (Change provider or update credentials)
       // highlight-end
       │  ○ Custom Providers 
       │  ○ Add Extension 
       │  ○ Toggle Extensions 
       │  ○ Remove Extension 
       │  ○ goose Settings 
       └  
       ```
    3. Choose a model provider and press `Enter`. Use the arrow keys (↑/↓) to move through the options.

       ```
       ┌   goose-configure 
       │
       ◇  What would you like to configure?
       │  Configure Providers 
       │
       ◆  Which model provider should we use?
       │  ○ Amazon Bedrock 
       │  ○ Amazon SageMaker TGI 
       // highlight-start
       │  ● Anthropic (Claude and other models from Anthropic)
       // highlight-end
       │  ○ Azure OpenAI 
       │  ○ Claude Code CLI
       │  ○ ...
       └  
       ```
    4. Enter your API key (and any other configuration details) when prompted.

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
       // highlight-start
       │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
       // highlight-end
       └  
       ```
       
       If you're just changing models, skip any prompts to update the provider configuration.

    5. Enter your desired `ANTHROPIC_HOST` or press `Enter` to use the default. 

       ```
       ◆  Provider Anthropic requires ANTHROPIC_HOST, please enter a value
       // highlight-start
       │  https://api.anthropic.com (default)
       // highlight-end
       ```
    6. Choose the model you want to use. Depending on the provider, you can:
       - Select the model from a list
       - Search for the model by name
       - Enter the model name directly
       
       ```
       │
       ◇  Model fetch complete
       │
       ◇  Select a model:
       // highlight-start
       │  claude-sonnet-4-5 (default)
       // highlight-end
       │
       ◒  Checking your configuration...
       └  Configuration saved successfully
       ```
  
       This change takes effect the next time you start a session.

  :::note
  `goose configure` doesn't support entering custom model names. To use a model not in the provider's list, use goose Desktop or edit the `GOOSE_MODEL` variable in your [`config.yaml`](/docs/guides/config-files) directly.
  :::

  :::tip
  Set the model for an individual session using the [`run` command](/docs/guides/goose-cli-commands#run-options):

  ```bash
  goose run --model claude-sonnet-4-0 -t "initial prompt"
  ```
  :::

  </TabItem>
</Tabs>

### Using Custom OpenAI Endpoints

The built-in OpenAI provider can connect to OpenAI's official API (`api.openai.com`) or any OpenAI-compatible endpoint, such as:
- Self-hosted LLMs (e.g., LLaMA, Mistral) using vLLM or KServe
- Private OpenAI-compatible API servers
- Enterprise deployments requiring data governance and security compliance
- OpenAI API proxies or gateways

:::tip Custom Provider Option
Need to connect to multiple OpenAI-compatible endpoints? [Configure custom providers](#configure-custom-provider) instead for easier switching and better organization, as well as custom naming and shareable configurations.
:::

#### Configuration Parameters

| Parameter | Required | Description |
|-----------|----------|-------------|
| `OPENAI_API_KEY` | Yes | Authentication key for the API |
| `OPENAI_HOST` | No | Custom endpoint URL (defaults to api.openai.com) |
| `OPENAI_ORGANIZATION` | No | Organization ID for usage tracking and governance |
| `OPENAI_PROJECT` | No | Project identifier for resource management |
| `OPENAI_CUSTOM_HEADERS` | No | Additional headers to include in the request. Can be set via environment variable, configuration file, or CLI, in the format `HEADER_A=VALUE_A,HEADER_B=VALUE_B`. |

#### Example Configurations

<Tabs groupId="deployment">
  <TabItem value="vllm" label="vLLM Self-Hosted" default>
    If you're running LLaMA or other models using vLLM with OpenAI compatibility:
    ```sh
    OPENAI_HOST=https://your-vllm-endpoint.internal
    OPENAI_API_KEY=your-internal-api-key
    ```
  </TabItem>
  <TabItem value="kserve" label="KServe Deployment">
    For models deployed on Kubernetes using KServe:
    ```sh
    OPENAI_HOST=https://kserve-gateway.your-cluster
    OPENAI_API_KEY=your-kserve-api-key
    OPENAI_ORGANIZATION=your-org-id
    OPENAI_PROJECT=ml-serving
    ```
  </TabItem>
  <TabItem value="enterprise" label="Enterprise OpenAI">
    For enterprise OpenAI deployments with governance:
    ```sh
    OPENAI_API_KEY=your-api-key
    OPENAI_ORGANIZATION=org-id123
    OPENAI_PROJECT=compliance-approved
    ```
  </TabItem>
  <TabItem value="custom-headers" label="Custom Headers">
    For OpenAI-compatible endpoints that require custom headers:
    ```sh
    OPENAI_API_KEY=your-api-key
    OPENAI_ORGANIZATION=org-id123
    OPENAI_PROJECT=compliance-approved
    OPENAI_CUSTOM_HEADERS="X-Header-A=abc,X-Header-B=def"
    ```
  </TabItem>
</Tabs>

#### Setup Instructions

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click the `Settings` button on the sidebar
    3. Click the `Models` tab
    4. Click `Configure providers`
    5. Click `OpenAI` in the provider list
    6. Fill in your configuration details:
       - API Key (required)
       - Host URL (for custom endpoints)
       - Organization ID (for usage tracking)
       - Project (for resource management)
    7. Click `Submit`
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    1. Run `goose configure`
    2. Select `Configure Providers`
    3. Choose `OpenAI` as the provider
    4. Enter your configuration when prompted:
       - API key
       - Host URL (if using custom endpoint)
       - Organization ID (if using organization tracking)
       - Project identifier (if using project management)
  </TabItem>
</Tabs>

:::tip Enterprise Deployment
For enterprise deployments, you can pre-configure these values using environment variables or configuration files to ensure consistent governance across your organization.
:::

## Configure Custom Provider

Custom providers let you connect to services that aren't in the [available providers](#available-providers) list. They appear in goose's provider list and can be selected like any other provider.

**Benefits:**
- **Multiple endpoints**: Switch between different services (e.g., vLLM, corporate proxy, OpenAI)
- **Pre-configured models**: Store a list of preferred models
- **Shareable configuration**: JSON files can be shared across teams or checked into repos
- **Custom naming**: Show "Corporate API" instead of "OpenAI" in the UI
- **Separate credentials**: Assign each provider its own API key

Custom providers must use OpenAI, Anthropic, or Ollama compatible API formats. OpenAI-compatible providers can include custom headers for additional authentication, API keys, tokens, or tenant identifiers. Each custom provider maps to a JSON configuration file.

**To add a custom provider:**
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click the `Settings` button on the sidebar
    3. Click the `Models` tab
    4. Click `Configure providers`
    5. Click `Add Custom Provider` at the bottom of the window
    6. Fill in the provider details:
       - **Provider Type**: 
         - `OpenAI Compatible` (most common)
         - `Anthropic Compatible`
         - `Ollama Compatible`
       - **Display Name**: A friendly name for the provider
       - **API URL**: The base URL of the API endpoint
       - **API Key**: The API key, which is accessed using a custom environment variable and stored in the keychain (or `secrets.yaml` if the keyring is disabled)
         - For `Ollama Compatible` providers, click `This is a local model (no auth required)`
       - **Available Models**: Comma-separated list of available model names
       - **Streaming Support**: Whether the API supports streaming responses (click to toggle)
    7. Click `Create Provider`

    :::info Custom Headers
    Currently, custom headers for OpenAI compatible providers can't be defined in goose Desktop. As a workaround, configure the provider using goose CLI or edit the provider configuration file directly.
    :::

  </TabItem>
  <TabItem value="cli" label="goose CLI">
    1. In your terminal, run the following command: 

       ```sh
       goose configure
       ```

    2. Select `Custom Providers`. Use the arrow keys (↑/↓) to move through the options.

       ```sh
       ┌   goose-configure 
       │
       ◆  What would you like to configure?
       │  ○ Configure Providers
       // highlight-start
       │  ● Custom Providers (Add custom provider with compatible API)
       // highlight-end
       │  ○ Add Extension 
       │  ○ Toggle Extensions 
       │  ○ Remove Extension 
       │  ○ goose Settings 
       └  
       ```

    3. Select `Add A Custom Provider`

       ```sh
       ┌   goose-configure 
       │
       ◇  What would you like to configure?
       │  Custom Providers 
       │
       ◆  What would you like to do?
       // highlight-start
       │  ● Add A Custom Provider (Add a new OpenAI/Anthropic/Ollama compatible Provider)
       // highlight-end
       │  ○ Remove Custom Provider
       └  
       ```

    4. Follow the prompts to enter the provider details:
       - **API Type**: 
         - `OpenAI Compatible` (most common)
         - `Anthropic Compatible`
         - `Ollama Compatible`
       - **Name**: A friendly name for the provider
       - **API URL**: The base URL of the API endpoint
       - **API Key**: The API key, which is accessed using a custom environment variable and stored in the keychain (or `secrets.yaml` if the keyring is disabled)
         - For `Ollama Compatible` providers, press `Enter` to skip (or enter any value to be able to use the provider in goose Desktop)
       - **Available Models**: Comma-separated list of available model names
       - **Streaming Support**: Whether the API supports streaming responses
       - **Custom Headers**: Required header names and values (`OpenAI Compatible` providers only)

  </TabItem>
  <TabItem value="config" label="Config File">

    First create a JSON file in the `custom_providers` directory:
    - macOS/Linux: `~/.config/goose/custom_providers/`
    - Windows: `%APPDATA%\Block\goose\config\custom_providers\`

    Example `custom_corp_api.json` configuration file:
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
      "supports_streaming": true
    }
    ```

    Then use the `api_key_env` to set the key for your session. For example:
    ```bash
    export CUSTOM_CORP_API_API_KEY="your-api-key"
    goose session start --provider custom_corp_api
    ```

    :::tip Keychain Key Storage
    If you want to store the API key in the `goose` keychain, update the provider in goose Desktop and enter the key. This provides secure, persistent storage and allows goose to connect natively to the provider.
    :::

  </TabItem>
</Tabs>

**To update a custom provider:**

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click the `Settings` button on the sidebar
    3. Click the `Models` tab
    4. Click `Configure providers`
    5. Click on your custom provider in the list
    6. Update the fields you want to change
       <br/>**Important:** Verify that `Provider Type` shows the correct value before saving. Otherwise, it may default to `OpenAI Compatible` regardless of the original setting.
    7. Click `Update Provider`

  </TabItem>
  <TabItem value="cli" label="goose CLI">
    
    1. In your terminal, run the following command: 

       ```sh
       goose configure
       ```

    2. Select `Configure Providers` from the menu and press `Enter`.

       ```sh
       ┌   goose-configure 
       │
       ◆  What would you like to configure?
       // highlight-start
       │  ● Configure Providers (Change provider or update credentials)
       // highlight-end
       │  ○ Custom Providers 
       │  ○ Add Extension 
       │  ○ Toggle Extensions 
       │  ○ Remove Extension 
       │  ○ goose Settings 
       └  
       ```

    3. Select the custom provider you want to update and press `Enter`. Use the arrow keys (↑/↓) to move through the options.

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
       // highlight-start
       │  ● Corporate API (Custom Corporate API provider)
       // highlight-end
       │  ○ Cursor Agent 
       │  ○ ...
       └  
       ```

    4. Follow the prompts to update the fields.

  </TabItem>
  <TabItem value="config" label="Config File">

    Open the custom provider configuration file in the `custom_providers` directory:
    - macOS/Linux: `~/.config/goose/custom_providers/`
    - Windows: `%APPDATA%\Block\goose\config\custom_providers\`

    Update the fields you want to change and save your changes.
  </TabItem>
</Tabs>

Your changes are available in your next goose session.

**To remove a custom provider:**

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    Currently you cannot remove custom providers using goose Desktop.
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    
    1. In your terminal, run the following command: 

       ```sh
       goose configure
       ```

    2. Select `Custom Providers`. Use the arrow keys (↑/↓) to move through the options.

       ```sh
       ┌   goose-configure 
       │
       ◆  What would you like to configure?
       │  ○ Configure Providers
       // highlight-start
       │  ● Custom Providers (Add custom provider with compatible API)
       // highlight-end
       │  ○ Add Extension 
       │  ○ Toggle Extensions 
       │  ○ Remove Extension 
       │  ○ goose Settings 
       └  
       ```

    3. Select `Remove Custom Provider`.

       ```sh
       ┌   goose-configure 
       │
       ◇  What would you like to configure?
       │  Custom Providers 
       │
       ◆  What would you like to do?
       │  ○ Add A Custom Provider 
       // highlight-start
       │  ● Remove Custom Provider (Remove an existing custom provider)
       // highlight-end
       └  
       ```

    4. Select the custom provider you want to remove.

    The provider configuration file is removed from the `custom_providers` directory and the key is removed from the keychain.

  </TabItem>
  <TabItem value="config" label="Config File">

    :::tip
    If the provider's API key is stored in the keychain, use goose CLI to remove the custom provider. This also removes the stored API key.
    :::

    Delete the custom provider configuration file in the `custom_providers` directory:
    - macOS/Linux: `~/.config/goose/custom_providers/`
    - Windows: `%APPDATA%\Block\goose\config\custom_providers\`

  </TabItem>
</Tabs>

## Using goose for Free

goose is a free and open source AI agent that you can start using right away, but not all supported [LLM Providers][providers] provide a free tier. 

Below, we outline a couple of free options and how to get started with them.

:::warning Limitations
These free options are a great way to get started with goose and explore its capabilities. However, you may need to upgrade your LLM for better performance.
:::


### Groq
Groq provides free access to open source models with high-speed inference. To use Groq with goose, you need an API key from [Groq Console](https://console.groq.com/keys).

Groq offers several open source models that support tool calling:
- **moonshotai/kimi-k2-instruct** - Mixture-of-Experts model with 1 trillion parameters, optimized for agentic intelligence and tool use
- **qwen/qwen3-32b** - 32.8 billion parameter model with advanced reasoning and multilingual capabilities  
- **gemma2-9b-it** - Google's Gemma 2 model with instruction tuning
- **llama-3.3-70b-versatile** - Meta's Llama 3.3 model for versatile applications

To set up Groq with goose, follow these steps:

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  **To update your LLM provider and API key:** 

    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar.
    2. Click the `Settings` button on the sidebar.
    3. Click the `Models` tab.
    4. Click `Configure Providers`
    5. Choose `Groq` as provider from the list.
    6. Click `Configure`, enter your API key, and click `Submit`.

  </TabItem>
  <TabItem value="cli" label="goose CLI">
    1. Run: 
    ```sh
    goose configure
    ```
    2. Select `Configure Providers` from the menu.
    3. Follow the prompts to choose `Groq` as the provider.
    4. Enter your API key when prompted.
    5. Enter the Groq model of your choice (e.g., `moonshotai/kimi-k2-instruct`).
  </TabItem>
</Tabs>

### Google Gemini
Google Gemini provides a free tier. To start using the Gemini API with goose, you need an API Key from [Google AI studio](https://aistudio.google.com/app/apikey).

To set up Google Gemini with goose, follow these steps:

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  **To update your LLM provider and API key:** 

    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar.
    2. Click the `Settings` button on the sidebar.
    3. Click the `Models` tab.
    4. Click `Configure Providers`
    5. Choose `Google Gemini` as provider from the list.
    6. Click `Configure`, enter your API key, and click `Submit`.

  </TabItem>
  <TabItem value="cli" label="goose CLI">
    1. Run: 
    ```sh
    goose configure
    ```
    2. Select `Configure Providers` from the menu.
    3. Follow the prompts to choose `Google Gemini` as the provider.
    4. Enter your API key when prompted.
    5. Enter the Gemini model of your choice.

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
    │▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │    
    ◇ Enter a model from that provider:
    │ gemini-2.0-flash-exp
    │
    ◇ Hello! You're all set and ready to go, feel free to ask me anything!
    │
    └ Configuration saved successfully
    ```
  </TabItem>
</Tabs>


### Local LLMs

goose is a local AI agent, and by using a local LLM, you keep your data private, maintain full control over your environment, and can work entirely offline without relying on cloud access. However, please note that local LLMs require a bit more set up before you can use one of them with goose.

:::warning Limited Support for models without tool calling
goose extensively uses tool calling, so models without it can only do chat completion. If using models without tool calling, all goose [extensions must be disabled](/docs/getting-started/using-extensions#enablingdisabling-extensions).
:::

Here are some local providers we support:

<Tabs groupId="local-llms">
  <TabItem value="ollama" label="Ollama" default>
    <Tabs groupId="ollama-models">
      <TabItem value="ramalala" label="Ramalala">
        1. [Download Ramalama](https://github.com/containers/ramalama?tab=readme-ov-file#install).
        2. In a terminal, run any Ollama [model supporting tool-calling](https://ollama.com/search?c=tools) or [GGUF format HuggingFace Model](https://huggingface.co/search/full-text?q=%22tools+support%22+%2B+%22gguf%22&type=model):

          The `--runtime-args="--jinja"` flag is required for Ramalama to work with the goose Ollama provider.

          Example:

          ```sh
          ramalama serve --runtime-args="--jinja" ollama://qwen2.5
          ```

          3. In a separate terminal window, configure with goose:

          ```sh
          goose configure
          ```

          4. Choose to `Configure Providers`

          ```
          ┌   goose-configure
          │
          ◆  What would you like to configure?
          │  ● Configure Providers (Change provider or update credentials)
          │  ○ Toggle Extensions
          │  ○ Add Extension
          └
          ```

          5. Choose `Ollama` as the model provider since Ramalama is API compatible and can use the goose Ollama provider

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

          6. Enter the host where your model is running

          :::info Endpoint
          For the Ollama provider, if you don't provide a host, we set it to `localhost:11434`. When constructing the URL, we preprend `http://` if the scheme is not `http` or `https`. Since Ramalama's default port to serve on is 8080, we set `OLLAMA_HOST=http://0.0.0.0:8080`
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


          7. Enter the model you have running

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
          If you notice that goose is having trouble using extensions or is ignoring [.goosehints](/docs/guides/using-goosehints), it is likely that the model's default context length of 2048 tokens is too low. Use `ramalama serve` to set the `--ctx-size, -c` option to a [higher value](https://github.com/containers/ramalama/blob/main/docs/ramalama-serve.1.md#--ctx-size--c).
          :::

      </TabItem>
      <TabItem value="deepseek" label="DeepSeek-R1">
        The native `DeepSeek-r1` model doesn't support tool calling, however, we have a [custom model](https://ollama.com/michaelneale/deepseek-r1-goose) you can use with goose. 

        :::warning
        Note that this is a 70B model size and requires a powerful device to run smoothly.
        :::


        1. [Download Ollama](https://ollama.com/download). 
        2. In a terminal window, run the following command to install the custom DeepSeek-r1 model:

        ```sh
        ollama run michaelneale/deepseek-r1-goose
        ```

        3. In a separate terminal window, configure with goose:

        ```sh
        goose configure
        ```

        4. Choose to `Configure Providers`

        ```
        ┌   goose-configure 
        │
        ◆  What would you like to configure?
        │  ● Configure Providers (Change provider or update credentials)
        │  ○ Toggle Extensions 
        │  ○ Add Extension 
        └  
        ```

        5. Choose `Ollama` as the model provider

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

        6. Enter the host where your model is running

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
        │  http://localhost:11434
        └
        ```

        7. Enter the installed model from above

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
      <TabItem value="others" label="Other Models" default>
        1. [Download Ollama](https://ollama.com/download). 
        2. In a terminal, run any [model supporting tool-calling](https://ollama.com/search?c=tools)

          Example:

          ```sh
          ollama run qwen2.5
          ```

        3. In a separate terminal window, configure with goose:

          ```sh
          goose configure
          ```

        4. Choose to `Configure Providers`

        ```
        ┌   goose-configure 
        │
        ◆  What would you like to configure?
        │  ● Configure Providers (Change provider or update credentials)
        │  ○ Toggle Extensions 
        │  ○ Add Extension 
        └  
        ```

        5. Choose `Ollama` as the model provider

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

        6. Enter the host where your model is running

        :::info Endpoint
        For Ollama, if you don't provide a host, we set it to `localhost:11434`. 
        When constructing the URL, we prepend `http://` if the scheme is not `http` or `https`. 
        If you're running Ollama on a different server, you'll have to set `OLLAMA_HOST=http://{host}:{port}`.
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
        │  http://localhost:11434
        └
        ```


        7. Enter the model you have running

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
        If you notice that goose is having trouble using extensions or is ignoring [.goosehints](/docs/guides/using-goosehints), it is likely that the model's default context length of 4096 tokens is too low. Set the `OLLAMA_CONTEXT_LENGTH` environment variable to a [higher value](https://github.com/ollama/ollama/blob/main/docs/faq.mdx#how-can-i-specify-the-context-window-size).
        :::
        
      </TabItem>
    </Tabs>
  </TabItem>
  <TabItem value="docker" label="Docker Model Runner" default>
    1. [Get Docker](https://docs.docker.com/get-started/get-docker/)
    2. [Enable Docker Model Runner](https://docs.docker.com/ai/model-runner/#enable-dmr-in-docker-desktop)
    3. [Pull a model](https://docs.docker.com/ai/model-runner/#pull-a-model), for example, from Docker Hub [AI namespace](https://hub.docker.com/u/ai), [Unsloth](https://hub.docker.com/u/unsloth), or [from HuggingFace](https://www.docker.com/blog/docker-model-runner-on-hugging-face/)

    Example:

    ```sh
    docker model pull hf.co/unsloth/gemma-3n-e4b-it-gguf:q6_k
    ```

    4. Configure goose to use Docker Model Runner, using the OpenAI API compatible endpoint: 

    ```sh
    goose configure
    ```

    5. Choose to `Configure Providers`

    ```
    ┌   goose-configure 
    │
    ◆  What would you like to configure?
    │  ● Configure Providers (Change provider or update credentials)
    │  ○ Toggle Extensions 
    │  ○ Add Extension 
    └  
    ```

    6. Choose `OpenAI` as the model provider: 

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
    ```

    7. Configure Docker Model Runner endpoint as the `OPENAI_HOST`: 

    ```
    ┌   goose-configure
    │
    ◇  What would you like to configure?
    │  Configure Providers
    │
    ◇  Which model provider should we use?
    │  OpenAI
    │
    ◆  Provider OpenAI requires OPENAI_HOST, please enter a value
    │  https://api.openai.com (default)
    └
    ```

    The default value for the host-side port Docker Model Runner is 12434, so the `OPENAI_HOST` value could be: 
    `http://localhost:12434`. 

    8. Configure the base path: 

    ```
    ◆  Provider OpenAI requires OPENAI_BASE_PATH, please enter a value
    │  v1/chat/completions (default)
    └
    ```

    Docker model runner uses `/engines/llama.cpp/v1/chat/completions` for the base path.

    9. Finally configure the model available in Docker Model Runner to be used by goose: `hf.co/unsloth/gemma-3n-e4b-it-gguf:q6_k`

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



## Azure OpenAI Credential Chain

goose supports two authentication methods for Azure OpenAI:

1. **API Key Authentication** - Uses the `AZURE_OPENAI_API_KEY` for direct authentication
2. **Azure Credential Chain** - Uses Azure CLI credentials automatically without requiring an API key

To use the Azure Credential Chain:
- Ensure you're logged in with `az login`
- Have appropriate Azure role assignments for the Azure OpenAI service
- Configure with `goose configure` and select Azure OpenAI, leaving the API key field empty

This method simplifies authentication and enhances security for enterprise environments.

## Multi-Model Configuration

Beyond single-model setups, goose supports [multi-model configurations](/docs/guides/multi-model/) that can use different models and providers for specialized tasks:

- **Lead/Worker Model** - Automatic switching between a lead model for initial turns and a worker model for execution tasks
- **Planning Mode** - Manual planning phase using a dedicated model to create detailed project breakdowns before execution

---

If you have any questions or need help with a specific provider, feel free to reach out to us on [Discord](https://discord.gg/goose-oss) or on the [goose repo](https://github.com/block/goose).


[providers]: /docs/getting-started/providers
[function-calling-leaderboard]: https://gorilla.cs.berkeley.edu/leaderboard.html
