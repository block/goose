---
title: LLM Rate Limits
sidebar_position: 4
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Handling LLM Rate Limits

Rate limiting is the process of restricting the number of requests a user or application can send to an LLM API within a specific timeframe. LLM providers enforce this with the purpose of managing resources and preventing abuse. 

Since Goose is working very quickly to implement your tasks, you may need to manage rate limits imposed by the provider. If you frequently hit rate limits, consider upgrading your LLM plan to access higher tier limits or using OpenRouter.

## Using OpenRouter

OpenRouter provides a unified interface for LLMs that allows you to select and switch between different providers automatically - all under a single billing plan. With OpenRouter, you can utilize free models or purchase credits for paid models.

1. Go to [openrouter.ai](https://openrouter.ai) and create an account. 
2. Once verified, create your [API key](https://openrouter.ai/settings/keys).
<!-- 3. Add your API key and OpenRouter configuration to your environment variables: -->


<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
    1. Run the Goose configuration command:
    ```sh
    goose configure
    ```
    2. Select `Configure Providers` from the menu.
    3. Follow the prompts to choose OpenRouter as your provider and enter your OpenRouter API key when prompted.
  </TabItem>
  <TabItem value="ui" label="Goose Desktop">

    1. Click on the three dots in the top-right corner.
    2. Select `Settings` from the menu.
    3. Click on "Browse" in the `Models` section.
    4. Click on `Configure`
    5. Select `OpenRouter` from the list of available providers.
    6. Enter your OpenRouter API key in the dialog that appears.
  </TabItem>
</Tabs>


Now Goose will send your requests through OpenRouter which will automatically switch models when necessary to avoid interruptions due to rate limiting.

