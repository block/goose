---
title: Set LLM Rate Limits
sidebar_label: LLM Rate Limits
sidebar_position: 8
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft } from 'lucide-react';


Rate limiting is the process of restricting the number of requests a user or application can send to an LLM API within a specific timeframe. LLM providers enforce this with the purpose of managing resources and preventing abuse. 

Since Goose is working very quickly to implement your tasks, you may need to manage rate limits imposed by the provider. If you frequently hit rate limits, consider upgrading your LLM plan to access higher tier limits or using OpenRouter.

## Using OpenRouter

OpenRouter provides a unified interface for LLMs that allows you to select and switch between different providers automatically - all under a single billing plan. With OpenRouter, you can utilize free models or purchase credits for paid models.

1. Go to [openrouter.ai](https://openrouter.ai) and create an account. 
2. Once verified, create your [API key](https://openrouter.ai/settings/keys).

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar.
    2. Click the `Settings` button on the sidebar.
    3. Click the `Models` tab.
    3. Click `Configure Providers`.
    5. Click `Configure` under `OpenRouter` to edit your OpenRouter settings.
    6. Enter your OpenRouter API key.
    7. Click `Submit`.
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
    1. Run the Goose configuration command:
    ```sh
    goose configure
    ```
    2. Select `Configure Providers` from the menu.
    3. Follow the prompts to choose OpenRouter as your provider and enter your OpenRouter API key when prompted.
  </TabItem>
</Tabs>


Now Goose will send your requests through OpenRouter which will automatically switch models when necessary to avoid interruptions due to rate limiting.