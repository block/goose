---
title: Handling LLM Rate Limits with Goose
---

# Handling LLM Rate Limits with Goose

Rate limiting is the process of restricting the number of requests a user or application can send to an LLM API within a specific timeframe with the purpose of managing resources and preventing abuse. 

Since goose requires you to bring your own LLM provider, you may need to manage rate limits imposed by the provider. This guide provides you with some best practices to efficiently handle rate limits while using Goose.

## Example Rate Limits

| **Model**               | **Provider**        | **Rate Limit**                                                                 |
|--------------------------|---------------------|--------------------------------------------------------------------------------|
| GPT-4          | OpenAI             | Rate limits vary by subscription tier. For example, GPT-4o offers up to 10 million tokens per minute, which is 5 times higher than GPT-4 Turbo. |
| Claude                | Anthropic          | Rate limits are set at the organization level and defined by usage tiers. For instance, a rate of 60 requests per minute may be enforced as 1 request per second.          |

### Tips to Manage Rate Limits

1. **Monitor Your Usage**: Keep track of your API usage through provider dashboards or logging tools.
2. **Upgrade your plan**: If you frequently hit rate limits, consider upgrading your plan to access higher tier limits.

## Using openrouter.ai as an Alternative

OpenRouter provides a unified interface for LLMs that allows you to select and switch between different providers seamlessly and automatically - all under a single billing plan. 

1. Go to [openrouter.ai](https://openrouter.ai) and create an account. OpenRouter allows you to use some models for free, or purchase credits to use the paid models.
2. Once verified, create your [API key](https://openrouter.ai/settings/keys).

[Insert image]

3. Add your API key and OpenRouter configuration to your environment variables:

```bash
export GOOSE_PROVIDER="openrouter"
export OPENROUTER_API_KEY="..."
```

Now, you can send your requests through OpenRouter, which would automatically switch models for you to make sure your work is not interrupted with goose.
