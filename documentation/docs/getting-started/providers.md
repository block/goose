---
sidebar_position: 2
title: Configure LLM Provider
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Supported LLM Providers


Goose is compatible with a wide range of LLM providers, allowing you to choose and integrate your preferred model.

## Available Providers

| Provider                                      | Description                                                                                                                                                                                                              | Parameters                            |
|-----------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------|
| [Anthropic](https://www.anthropic.com/)       | Offers Claude, an advanced AI model for natural language tasks.                                                                                                                                                          | `ANTHROPIC_API_KEY`                   |
| [Databricks](https://www.databricks.com/)     | Unified data analytics and AI platform for building and deploying models.                                                                                                                                                | `DATABRICKS_HOST`, `DATABRICKS_TOKEN` |
| [Gemini](https://ai.google.dev/gemini-api/docs) | Advanced LLMs by Google with multimodal capabilities (text, images).                                                                                                                                                   | `GOOGLE_API_KEY`                      |
| [Groq](https://groq.com/)                     | High-performance inference hardware and tools for LLMs.                                                                                                                                                                  | `GROQ_API_KEY`                        |
| [Ollama](https://ollama.com/)                 | Local model runner supporting Qwen, Llama, DeepSeek, and other open-source models. **Because this provider runs locally, you must first [download and run a model](/docs/getting-started/providers#local-llms-ollama).** | N/A                                   |
| [OpenAI](https://platform.openai.com/api-keys) | Provides gpt-4o, o1, and other advanced language models. **o1-mini and o1-preview are not supported because Goose uses tool calling.**                                                                                  | `OPENAI_API_KEY`                      |
| [OpenRouter](https://openrouter.ai/)          | API gateway for unified access to various models with features like rate-limiting management.                                                                                                                            | `OPENROUTER_API_KEY`                  |

:::tip Model Recommendation
Goose currently works best with Anthropic's Claude 3.5 Sonnet and OpenAI's o1 model. 
:::
   
## Configure Provider

To configure your chosen provider or see available options, run `goose configure` in the CLI or visit the `Provider Settings` page in the Goose Desktop.

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
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
   4. Enter you API key (and any other configuration details) when prompted

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
  <TabItem value="ui" label="Goose Desktop">
  **To update your LLM provider and API key:** 

    1. Click the three dots (`...`) in the top-right corner.
    2. Select `Provider Settings` from the menu.
    3. Click Edit, enter your API key, and click `Set as Active`.

  </TabItem>
</Tabs>

## Local LLMs (Ollama)

Ollama provides local LLMs, which requires a bit more set up before you can use it with Goose.

1. [Download Ollama](https://ollama.com/download). 
2. Run any [model supporting tool-calling](https://ollama.com/search?c=tools):

:::warning Limited Support for models without tool calling
Goose extensively uses tool calling, so models without it (e.g. `DeepSeek-r1`) can only do chat completion. If using models without tool calling, all Goose [extensions must be disabled](/docs/getting-started/using-extensions#enablingdisabling-extensions). As an alternative, you can use a [custom DeepSeek-r1 model](/docs/getting-started/using-goose-free#deepseek-r1) we've made specifically for Goose.
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