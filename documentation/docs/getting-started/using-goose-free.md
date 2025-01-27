---
sidebar_position: 3
title: Using Goose for Free
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Using Goose for Free

Goose is a free and open source developer AI agent that you can start using right away, but not all supported [LLM Providers][providers] provide a free tier. 

Below, we outline a couple of free options and how to get started with them.


## Google Gemini
Google Gemini provides a free tier. To start using the Gemini API with Goose, you need an API Key from [Google AI studio](https://aistudio.google.com/app/apikey).

To set up Google Gemini with Goose, follow these steps:

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
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
  <TabItem value="ui" label="Goose Desktop">
  **To update your LLM provider and API key:** 

    1. Click on the three dots in the top-right corner.
    2. Select `Provider Settings` from the menu.
    2. Choose `Google Gemini` as provider from the list.
    3. Click Edit, enter your API key, and click `Set as Active`.

  </TabItem>
</Tabs>

## DeepSeek-R1

:::warning
Depending on the model's size, you'll need a relatively powerful device to smoothly run local LLMs.
:::

Ollama provides open source LLMs, such as `DeepSeek-r1`, that you can install and run locally.
Note that the native `DeepSeek-r1` model doesn't support tool calling, however, we have a [custom model](https://ollama.com/michaelneale/deepseek-r1-goose) you can use with Goose. 



1. Download and install Ollama from [ollama.com](https://ollama.com/download).
2. In a terminal window, run the following command to install the custom DeepSeek-r1 model:

```sh
ollama run michaelneale/deepseek-r1-goose
```

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
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

    6. Enter the installed deepseek-r1 model from above

    ```
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Configure Providers 
    │
    ◇  Which model provider should we use?
    │  Ollama 
    │
    ◇  Enter a model from that provider:
    │  michaelneale/deepseek-r1-goose
    │
    ◇  Welcome! You're all set to explore and utilize my capabilities. Let's get started on solving your problems together!
    │
    └  Configuration saved successfully
    ```
  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
    3. Click `...` in the top-right corner.
    4. Navigate to `Settings` -> `Browse Models` -> and select `Ollama` from the list.
    5. Enter `michaelneale/deepseek-r1-goose` for the model name.
  </TabItem>
</Tabs>

## Limitations

These free options are a great way to get started with Goose and explore its capabilities. However, if you need more advanced features or higher usage limits, you can upgrade to a paid plan with your LLM provider.

---

If you have any questions or need help with a specific provider, feel free to reach out to us on [Discord](https://discord.gg/block-opensource) or on the [Goose repo](https://github.com/block/goose).


[providers]: /docs/getting-started/providers