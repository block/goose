---
title: Set LLM Rate Limits
sidebar_label: LLM Rate Limits
sidebar_position: 60
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft } from 'lucide-react';


Rate limiting is the process of restricting the number of requests a user or application can send to an LLM API within a specific timeframe. LLM providers enforce this with the purpose of managing resources and preventing abuse. 

Since goose is working very quickly to implement your tasks, you may need to manage rate limits imposed by the provider. If you frequently hit rate limits, consider upgrading your LLM plan to access higher tier limits or [configure a provider](/docs/getting-started/providers#configure-provider-and-model) that has built-in rate limiting:

:::info
goose supports automatic setup for both providers that takes you through OAuth account creation and secure API key generation.
:::

- **Tetrate Agent Router**: Unified API gateway for AI models including Claude, Gemini, GPT, open-weight models, and others. The developer's shortest path to models with enterprise-grade routing, built-in rate limiting, and automatic failover.

  Manage your account at [router.tetrate.ai](https://router.tetrate.ai/dashboard).

- **OpenRouter**: Provides a unified interface for LLMs that allows you to select and switch between different providers automatically - all under a single billing plan. With OpenRouter, you can utilize free models or purchase credits for paid models. 

  Manage your account at [openrouter.ai](https://openrouter.ai).

When goose sends your requests through one of these providers, the provider will automatically switch models when necessary to avoid interruptions due to rate limiting.
