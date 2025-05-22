---
sidebar_position: 2
title: Configure LLM Provider
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Supported LLM Providers

Goose is compatible with a wide range of LLM providers, allowing you to choose and integrate your preferred model.

:::tip Model Selection
Goose relies heavily on tool calling capabilities and currently works best with Anthropic's Claude 3.5 Sonnet and OpenAI's GPT-4o (2024-11-20) model.
[Berkeley Function-Calling Leaderboard][function-calling-leaderboard] can be a good guide for selecting models.
:::

## Available Providers

| Provider                                                                    | Description                                                                                                                                                                                                               | Parameters                                                                                                                                                                          |
|-----------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| [Amazon Bedrock](https://aws.amazon.com/bedrock/)                           | Offers a variety of foundation models, including Claude, Jurassic-2, and others. **AWS environment variables must be set in advance, not configured through `goose configure`**                                           | `AWS_PROFILE`, or `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`, ...                                                                                                   |
| [Anthropic](https://www.anthropic.com/)                                     | Offers Claude, an advanced AI model for natural language tasks.                                                                                                                                                           | `ANTHROPIC_API_KEY`, `ANTHROPIC_HOST` (optional)                                                                                                                                                                 |
| [Azure OpenAI](https://learn.microsoft.com/en-us/azure/ai-services/openai/) | Access Azure-hosted OpenAI models, including GPT-4 and GPT-3.5. Supports both API key and Azure credential chain authentication.                                                                                          | `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_DEPLOYMENT_NAME`, `AZURE_OPENAI_API_KEY` (optional)                                                                                           |
| [Databricks](https://www.databricks.com/)                                   | Unified data analytics and AI platform for building and deploying models.                                                                                                                                                 | `DATABRICKS_HOST`, `DATABRICKS_TOKEN`                                                                                                                                               |
| [Gemini](https://ai.google.dev/gemini-api/docs)                             | Advanced LLMs by Google with multimodal capabilities (text, images).                                                                                                                                                      | `GOOGLE_API_KEY`                                                                                                                                                                    |
| [GCP Vertex AI](https://cloud.google.com/vertex-ai)                         | Google Cloud's Vertex AI platform, supporting Gemini and Claude models. **Credentials must be configured in advance. Follow the instructions at https://cloud.google.com/vertex-ai/docs/authentication.**                 | `GCP_PROJECT_ID`, `GCP_LOCATION` and optional `GCP_MAX_RETRIES` (6), `GCP_INITIAL_RETRY_INTERVAL_MS` (5000), `GCP_BACKOFF_MULTIPLIER` (2.0), `GCP_MAX_RETRY_INTERVAL_MS` (320_000). |
| [GitHub Copilot](https://docs.github.com/en/copilot/using-github-copilot/ai-models) | Access to GitHub Copilot's chat models including gpt-4o, o1, o3-mini, and Claude models. Uses device code authentication flow for secure access. | Uses GitHub device code authentication flow (no API key needed) |
| [Groq](https://groq.com/)                                                   | High-performance inference hardware and tools for LLMs.                                                                                                                                                                   | `GROQ_API_KEY`                                                                                                                                                                      |
| [Ollama](https://ollama.com/)                                               | Local model runner supporting Qwen, Llama, DeepSeek, and other open-source models. **Because this provider runs locally, you must first [download and run a model](/docs/getting-started/providers#local-llms-ollama).**  | `OLLAMA_HOST`                                                                                                                                                                       |
| [OpenAI](https://platform.openai.com/api-keys)                              | Provides gpt-4o, o1, and other advanced language models. Also supports OpenAI-compatible endpoints (e.g., self-hosted LLaMA, vLLM, KServe). **o1-mini and o1-preview are not supported because Goose uses tool calling.** | `OPENAI_API_KEY`, `OPENAI_HOST` (optional), `OPENAI_ORGANIZATION` (optional), `OPENAI_PROJECT` (optional), `OPENAI_CUSTOM_HEADERS` (optional)                                       |
| [OpenRouter](https://openrouter.ai/)                                        | API gateway for unified access to various models with features like rate-limiting management.                                                                                                                             | `OPENROUTER_API_KEY`                                                                                                                                                                |


## Supported Models by Provider

Below is a list of models officially supported by Goose for each provider. You can use these model names when configuring Goose through the CLI or Desktop interface.

:::tip Google Gemini vs. GCP Vertex AI
Google offers two separate providers with different models and configuration requirements:

- **Google Gemini** - Direct API with simple API key setup, focused on Gemini models only
- **GCP Vertex AI** - Enterprise platform requiring GCP project setup, offering both Google models AND third-party models like Claude

Choose Google Gemini for simpler setup or GCP Vertex AI for enterprise features and additional model options.
:::

:::tip Anthropic vs. AWS Bedrock
You can access Claude models in two ways:

- **Anthropic** - Direct API from Anthropic, requires an Anthropic API key
- **AWS Bedrock** - Access through AWS, requires AWS credentials and uses different model names

Choose based on your existing infrastructure and API key availability.
:::

:::tip OpenAI vs. Azure OpenAI
You can access OpenAI models in two ways:

- **OpenAI** - Direct API from OpenAI, requires an OpenAI API key
- **Azure OpenAI** - Access through Microsoft Azure, requires Azure setup and supports Azure credential chain

Use Azure OpenAI for enterprise compliance, data residency requirements, or if you already have Azure infrastructure.
:::

### OpenAI
- **Default Model:** `gpt-4o`
- **Supported Models:**
  - `gpt-4o`
  - `gpt-4o-mini`
  - `gpt-4-turbo`
  - `gpt-3.5-turbo`
  - `o1`
  - `o3`
  - `o4-mini`
  - `gpt-4.1`
  - `gpt-4-1`

### Anthropic
- **Default Model:** `claude-3-5-sonnet-latest`
- **Supported Models:**
  - `claude-3.5-sonnet-2` (Same as `claude-3-5-sonnet-latest`)
  - `claude-3-5-haiku-latest`
  - `claude-3-opus-latest`
  - `claude-3-7-sonnet-20250219`
  - `claude-3-7-sonnet-latest`

:::note Model Naming
Anthropic models may use either hyphen format (`claude-3-5-sonnet-latest`) or dot format (`claude-3.5-sonnet-2`). Both formats work but the exact name might vary based on API version.
:::

### Google Gemini
- **Default Model:** `gemini-1.5-flash`
- **Supported Models:**
  - `gemini-1.5-flash` (Recommended for general use)
  - `gemini-2.0-flash`
  - `gemini-2.0-flash-lite-preview-02-05`
  - `gemini-2.0-flash-thinking-exp-01-21`
  - `gemini-2.0-pro-exp-02-05`
  - `gemini-2.5-pro-exp-03-25`
  - `gemini-2.5-flash-preview-04-17`

:::note Gemini Versions
Google frequently updates Gemini models. The version numbers may change (1.5, 2.0, 2.5), and newer versions may become available after this documentation was written.
:::

### GitHub Copilot
- **Default Model:** `gpt-4o`
- **Supported Models:**
  - `gpt-4o`
  - `o1`
  - `o3-mini`
  - `claude-3.7-sonnet`
  - `claude-3.5-sonnet`

### Azure OpenAI
- **Default Model:** `gpt-4o`
- **Supported Models:**
  - `gpt-4o`
  - `gpt-4o-mini`
  - `gpt-4`

### Ollama
- **Default Model:** You need to specify the model you have installed locally.
- **Recommended Models with Tool Support:**
  - `qwen2.5` (Recommended for best performance)
  - `llama3.2`
  - `phi3:mini`
  - `michaelneale/deepseek-r1-goose` (Custom model with Goose-specific tool support)

### AWS Bedrock
- **Default Model:** You need to specify the model ID for AWS Bedrock.
- **Supported Models:**
  - `us.anthropic.claude-3-7-sonnet-20250219-v1:0` (Recommended)
  - `anthropic.claude-3-haiku-20240307-v1:0`
  - `anthropic.claude-3-sonnet-20240229-v1:0`
  - `anthropic.claude-3-opus-20240229-v1:0`
  - `anthropic.claude-v2:1`
  - `anthropic.claude-instant-v1`

:::note Model ID Format
AWS Bedrock model IDs may include a region prefix (`us.` or other region code) or omit it depending on your configuration. Check the AWS Bedrock console for the exact model IDs available in your region.
:::

### Databricks
- **Default Model:** `goose`
- **Supported Models:**
  - `goose` (Recommended for Goose)
  - `databricks-dbrx-instruct`
  - `databricks-claude-3-sonnet-20240229`
  - `databricks-mpt-30b-instruct`
  - `o1`, `o3`, `o3-mini` (with optional reasoning suffixes)

:::note Databricks O1/O3 Models
For O1 and O3 models, you can specify reasoning effort by adding `-low`, `-medium`, or `-high` to the model name. For example:
- `o1-medium` (medium reasoning effort, default if not specified)
- `o3-high` (high reasoning effort)
- `o3-mini-low` (low reasoning effort)

For Claude 3.7 models, you can enable thinking capabilities with the `CLAUDE_THINKING_ENABLED=1` environment variable.
:::

### GCP Vertex AI
- **Default Model:** `gemini-1.5-pro-002`
- **Supported Models:**
  - `gemini-1.5-pro-002`
  - `gemini-1.5-flash`
  - `gemini-2.0-flash-001`
  - `gemini-2.0-pro-exp-02-05`
  - `gemini-2.5-pro-exp-03-25`
  - `claude-3-5-haiku@20241022`
  - `claude-3-5-sonnet@20240620`
  - `claude-3-5-sonnet-v2@20241022`
  - `claude-3-7-sonnet@20250219`

:::note Vertex AI Model Names
Vertex AI model names often include date stamps or version numbers. The available models may change as Google and Anthropic release updates.

Vertex AI models are also region-specific:
- Claude models default to the Ohio region (us-east5)
- Gemini models default to the Iowa region (us-central1)
:::

### Groq
- **Default Model:** `llama-3.3-70b-versatile`
- **Supported Models:**
  - `llama-3.3-70b-versatile` (Recommended for general use)
  - `llama3-8b-8192`
  - `llama3-70b-8192`
  - `mixtral-8x7b-32768`
  - `gemma-7b-it`

### OpenRouter
- **Default Model:** `openai/gpt-4o`
- **Supported Models:**
  - `openai/gpt-4o`
  - `anthropic/claude-3-opus`
  - `anthropic/claude-3-sonnet`
  - `meta-llama/llama-3-70b-instruct`
  - Many more models are available through OpenRouter

:::info More Models
For each provider, you can use additional models beyond those listed here but this list should get you started. Model availability and naming may change over time as providers update their offerings.
:::


## Configure Provider

To configure your chosen provider or see available options, run `goose configure` in the CLI or visit the `Settings` page in the Goose Desktop.

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  **To update your LLM provider and API key:** 
  1. Click the gear on the Goose Desktop toolbar
  1. Click `Advanced Settings`
  1. Under `Models`, click `Configure provider`
  1. Click `Configure` on the LLM provider to update
  1. Add additional configurations (API key, host, etc) then press `submit`

  **To change provider model**
  1. Click the gear on the Goose Desktop toolbar
  2. Click `Advanced Settings`
  3. Under `Models`, click `Switch models`
  5. Select a Provider from drop down menu
  6. Select a model from drop down menu
  7. Press `Select Model`

  You can explore more models by selecting a `provider` name under `Browse by Provider`. A link will appear, directing you to the provider's website. Once you've found the model you want, return to step 6 and paste the model name.
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
    1. Run the following command: 

    ```sh
    goose configure
    ```

    2. Select `Configure Providers` from the menu and press Enter.

    ```
   ┌   goose-configure 
   │
   ◆  What would you like to configure?
   │  ● Configure Providers (Change provider or update credentials)
   │  ○ Toggle Extensions 
   │  ○ Add Extension 
   └  
   ```
   3. Choose a model provider and press Enter.

   ```
   ┌   goose-configure 
   │
   ◇  What would you like to configure?
   │  Configure Providers 
   │
   ◆  Which model provider should we use?
   │  ● Anthropic (Claude and other models from Anthropic)
   │  ○ Databricks 
   │  ○ Google Gemini 
   │  ○ Groq 
   │  ○ Ollama 
   │  ○ OpenAI 
   │  ○ OpenRouter 
   └  
   ```
   4. Enter your API key (and any other configuration details) when prompted

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
   │   
   └  
```
  </TabItem>
</Tabs>

## Using Custom OpenAI Endpoints

Goose supports using custom OpenAI-compatible endpoints, which is particularly useful for:
- Self-hosted LLMs (e.g., LLaMA, Mistral) using vLLM or KServe
- Private OpenAI-compatible API servers
- Enterprise deployments requiring data governance and security compliance
- OpenAI API proxies or gateways

### Configuration Parameters

| Parameter | Required | Description |
|-----------|----------|-------------|
| `OPENAI_API_KEY` | Yes | Authentication key for the API |
| `OPENAI_HOST` | No | Custom endpoint URL (defaults to api.openai.com) |
| `OPENAI_ORGANIZATION` | No | Organization ID for usage tracking and governance |
| `OPENAI_PROJECT` | No | Project identifier for resource management |
| `OPENAI_CUSTOM_HEADERS` | No | Additional headers to include in the request. Can be set via environment variable, configuration file, or CLI, in the format `HEADER_A=VALUE_A,HEADER_B=VALUE_B`. |

### Example Configurations

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

### Setup Instructions

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    1. Click `...` in the upper right corner
    2. Click `Advanced Settings`
    3. Next to `Models`, click the `browse` link
    4. Click the `configure` link in the upper right corner
    5. Press the `+` button next to OpenAI
    6. Fill in your configuration details:
       - API Key (required)
       - Host URL (for custom endpoints)
       - Organization ID (for usage tracking)
       - Project (for resource management)
    7. Press `submit`
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
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

## Using Goose for Free

Goose is a free and open source AI agent that you can start using right away, but not all supported [LLM Providers][providers] provide a free tier. 

Below, we outline a couple of free options and how to get started with them.

:::warning Limitations
These free options are a great way to get started with Goose and explore its capabilities. However, you may need to upgrade your LLM for better performance.
:::


### Google Gemini
Google Gemini provides a free tier. To start using the Gemini API with Goose, you need an API Key from [Google AI studio](https://aistudio.google.com/app/apikey).

To set up Google Gemini with Goose, follow these steps:

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  **To update your LLM provider and API key:** 

    1. Click on the three dots in the top-right corner.
    2. Select `Provider Settings` from the menu.
    2. Choose `Google Gemini` as provider from the list.
    3. Click Edit, enter your API key, and click `Set as Active`.

  </TabItem>
  <TabItem value="cli" label="Goose CLI">
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


### Local LLMs (Ollama)

Ollama provides local LLMs, which requires a bit more set up before you can use it with Goose.

1. [Download Ollama](https://ollama.com/download). 
2. Run any [model supporting tool-calling](https://ollama.com/search?c=tools):

:::warning Limited Support for models without tool calling
Goose extensively uses tool calling, so models without it (e.g. `DeepSeek-r1`) can only do chat completion. If using models without tool calling, all Goose [extensions must be disabled](/docs/getting-started/using-extensions#enablingdisabling-extensions). As an alternative, you can use a [custom DeepSeek-r1 model](/docs/getting-started/providers#deepseek-r1) we've made specifically for Goose.
:::

Example:

```sh
ollama run qwen2.5
```

3. In a separate terminal window, configure with Goose:

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

5. Enter the host where your model is running

:::info Endpoint
For Ollama, if you don't provide a host, we set it to `localhost:11434`. When constructing the URL, we preprend `http://` if the scheme is not `http` or `https`. If you're running Ollama on port 80 or 443, you'll have to set `OLLMA_HOST=http://host:{port}`
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


6. Enter the model you have running

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

### DeepSeek-R1

Ollama provides open source LLMs, such as `DeepSeek-r1`, that you can install and run locally.
Note that the native `DeepSeek-r1` model doesn't support tool calling, however, we have a [custom model](https://ollama.com/michaelneale/deepseek-r1-goose) you can use with Goose. 

:::warning
Note that this is a 70B model size and requires a powerful device to run smoothly.
:::


1. Download and install Ollama from [ollama.com](https://ollama.com/download).
2. In a terminal window, run the following command to install the custom DeepSeek-r1 model:

```sh
ollama run michaelneale/deepseek-r1-goose
```

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    3. Click `...` in the top-right corner.
    4. Navigate to `Advanced Settings` -> `Browse Models` -> and select `Ollama` from the list.
    5. Enter `michaelneale/deepseek-r1-goose` for the model name.
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
    3. In a separate terminal window, configure with Goose:

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

    5. Enter the host where your model is running

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

    6. Enter the installed model from above

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
</Tabs>

## Azure OpenAI Credential Chain

Goose supports two authentication methods for Azure OpenAI:

1. **API Key Authentication** - Uses the `AZURE_OPENAI_API_KEY` for direct authentication
2. **Azure Credential Chain** - Uses Azure CLI credentials automatically without requiring an API key

To use the Azure Credential Chain:
- Ensure you're logged in with `az login`
- Have appropriate Azure role assignments for the Azure OpenAI service
- Configure with `goose configure` and select Azure OpenAI, leaving the API key field empty

This method simplifies authentication and enhances security for enterprise environments.

---

If you have any questions or need help with a specific provider, feel free to reach out to us on [Discord](https://discord.gg/block-opensource) or on the [Goose repo](https://github.com/block/goose).


[providers]: /docs/getting-started/providers
[function-calling-leaderboard]: https://gorilla.cs.berkeley.edu/leaderboard.html