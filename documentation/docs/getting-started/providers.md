---
sidebar_position: 2
title: Configure LLM Provider
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Supported LLM Providers

You can use Goose with your preferred LLM. Goose supports a variety of LLM providers. To configure your chosen provider or see available options, run `goose configure` in the CLI or visit the `Provider Settings` page in the Goose Desktop.

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
   4. Enter you API key when prompted. Get more info on [how to obtain your API key](/docs/getting-started/providers#available-providers).

   
   
   ## Available Providers

   ### OpenAI

   OpenAI offers powerful language models that include GPT-4, GPT-3.5-turbo, and more.

   1. Run the following command and choose `Configure Providers`:
      ```sh
      goose configure
      ```
   2. Select `OpenAI` from the list of available providers.
   3. Enter your `OPENAI_API_KEY` when prompted, which you can obtain by registering at [OpenAI's platform](https://platform.openai.com/api-keys).

   ### Databricks

   Databricks is a data analytics and AI platform that provides access to various AI models and tools. They offer integration with popular models and custom model deployment.

   1. Run the following command and choose `Configure Providers`:
      ```sh
      goose configure
      ```
   2. Select `Databricks` as your provider.
   3. Enter your `DATABRICKS_HOST` and `DATABRICKS_TOKEN`, which can be generated in your [Databricks Account Settings](https://www.databricks.com/).

   ### Ollama

   Ollama is an open-source project that allows running large language models locally. It supports various open-source models and provides an API for integration.

   1. Run the following command and choose `Configure Providers`:
      ```sh
      goose configure
      ```
   2. Select `Ollama` and follow the steps to download and set up your models as detailed on [Ollama's site](https://ollama.com/). Requires `OLLAMA_HOST`.

   ### Anthropic

   Anthropic is an AI research company that offers advanced language models through its API. Their primary model is Claude, which comes in various versions.

   1. Run the following command and choose `Configure Providers`:
      ```sh
      goose configure
      ```
   2. Choose `Anthropic` and provide the `ANTHROPIC_API_KEY`, obtainable via [Anthropic's platform](https://www.anthropic.com/).

   ### Google Gemini

   Google Gemini is a suite of large language models developed by Google. It offers multimodal capabilities and can be accessed through the [Google AI Studio](https://ai.google.dev/gemini-api/docs).

   1. Run the following command and choose `Configure Providers`:
      ```sh
      goose configure
      ```
   2. Pick `Google Gemini` from the list of providers and input your `GOOGLE_API_KEY`.

   ### Groq

   Groq is an AI company that offers high-performance inference for large language models. They provide access to various models through their API.

   1. Run the following command and choose `Configure Providers`:
      ```sh
      goose configure
      ```
   2. Select `Groq` from the list of providers and input your `GROQ_API_KEY`, set up via the [Groq Console](https://groq.com/).

   ### OpenRouter

   OpenRouter is a platform that provides access to multiple AI models from various providers through a single API. It simplifies the process of using different AI models in applications.

   1. Run the following command and choose `Configure Providers`:
      ```sh
      goose configure
      ```

   2. Select `OpenRouter` from the list of providers and input your `OPENROUTER_API_KEY`, set up via the [OpenRouter Console](https://openrouter.ai/).

    
  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
  **To update your LLM provider and API key:** 

    1. Click the three dots (`...`) in the top-right corner.
    2. Select `Provider Settings` from the menu.
    3. Click Edit, enter your API key, and click `Set as Active`.

  </TabItem>
</Tabs>
