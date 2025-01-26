---
sidebar_position: 2
title: Configure LLM Provider
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Supported LLM Providers


Goose is compatible with a wide range of LLM providers, allowing you to choose and integrate your preferred model.

## Available Providers

| Provider                                      | Description                                         | Parameters                            |
|-----------------------------------------------|---------------------------------------------------|---------------------------------------|
| [OpenAI](https://platform.openai.com/api-keys) | Provides GPT-4, GPT-3.5-turbo, and other advanced language models. | `OPENAI_API_KEY`                      |
| [Databricks](https://www.databricks.com/)     | Unified data analytics and AI platform for building and deploying models. | `DATABRICKS_HOST`, `DATABRICKS_TOKEN` |
| [Ollama](https://ollama.com/)                 | Local model runner supporting DeepSeek, Llama, Mistral, and other open-source models. | N/A         |
| [Anthropic](https://www.anthropic.com/)       | Offers Claude, an advanced AI model for natural language tasks. | `ANTHROPIC_API_KEY`                   |
| [Gemini](https://ai.google.dev/gemini-api/docs) | Advanced LLMs by Google with multimodal capabilities (text, images). | `GOOGLE_API_KEY`                      |
| [Groq](https://groq.com/)                     | High-performance inference hardware and tools for LLMs. | `GROQ_API_KEY`                        |
| [OpenRouter](https://openrouter.ai/) | API gateway for unified access to various models with features like rate-limiting management | `OPENROUTER_API_KEY`        |
   
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
