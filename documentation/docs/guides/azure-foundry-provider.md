---
title: Azure AI Foundry
description: Use OpenAI, Anthropic, and partner model deployments from an Azure AI Foundry project
---

# Azure AI Foundry

The `azure_foundry` provider connects goose to Azure AI Foundry deployments. It supports two endpoint types:

| Endpoint | Inference surface |
|---|---|
| Foundry project: `https://<resource>.services.ai.azure.com/api/projects/<project>` | OpenAI models through Responses, Claude through Anthropic Messages, and partner models through Chat Completions |
| MaaS/serverless: `https://<deployment>.<region>.models.ai.azure.com` | Chat Completions for the model bound to the endpoint |

For project endpoints, goose discovers deployments with `GET /deployments`. Deployment names can be customized; goose uses the returned `modelPublisher` to select the protocol and `modelName` to resolve model metadata such as the context window.

## Configuration

| Variable | Required | Description |
|---|---:|---|
| `AZURE_FOUNDRY_ENDPOINT` | Yes | Full Foundry project or MaaS endpoint |
| `AZURE_FOUNDRY_API_KEY` | No | API key; omit it to use Azure CLI credentials |
| `AZURE_FOUNDRY_AD_TOKEN` | No | Pre-acquired Microsoft Entra access token; takes precedence over the API key |
| `AZURE_FOUNDRY_API_VERSION` | No | Deployment discovery API version; project endpoints default to `v1` |

Run `goose configure`, select **Configure Providers**, and choose **Azure AI Foundry**. You can also set the variables before starting goose:

```sh
export AZURE_FOUNDRY_ENDPOINT="https://my-resource.services.ai.azure.com/api/projects/my-project"
export AZURE_FOUNDRY_API_KEY="<key>"
goose session
```

For a MaaS endpoint:

```sh
export AZURE_FOUNDRY_ENDPOINT="https://my-deployment.eastus.models.ai.azure.com"
export AZURE_FOUNDRY_API_KEY="<key>"
goose session
```

## Authentication

Authentication is selected in this order:

1. `AZURE_FOUNDRY_AD_TOKEN`
2. `AZURE_FOUNDRY_API_KEY`
3. Azure CLI credentials

When neither token nor key is configured, sign in with Azure CLI before starting goose:

```sh
az login
```

Project endpoints request a token for `https://ai.azure.com`. MaaS endpoints request a token for `https://ml.azure.com`.

## Protocol routing

For a project endpoint, goose routes each deployment using metadata returned by Azure:

- publisher `OpenAI` → `/openai/v1/responses`
- publisher `Anthropic` → `/anthropic/v1/messages`
- all other publishers → `/openai/v1/chat/completions`

If deployment discovery is temporarily unavailable, recognizable `gpt-*`, `o1*`, `o3*`, `o4*`, and `claude-*` names use their native surfaces. Other names use Chat Completions.

MaaS endpoints always use `/chat/completions`.

## Model metadata and pricing

The deployments API provides the deployment name and underlying `modelName`, `modelVersion`, and `modelPublisher`. goose uses the underlying model name to look up a context window in its bundled model catalog. An explicit `GOOSE_CONTEXT_LIMIT` or session override still takes precedence.

Azure pricing depends on region, SKU, offer, deployment type, and contract. The deployments API does not provide a reliable per-token price, so this provider does not attach a price to discovered deployments.

## Troubleshooting

### 401 or 403

- Ensure the key belongs to the configured endpoint.
- For Entra authentication, run `az login` again and verify that your identity has access to the Foundry project.
- Do not use a project endpoint key with a MaaS endpoint, or the reverse.

### No deployments are listed

- Confirm that the endpoint includes `/api/projects/<project>`.
- Confirm that the project contains model deployments.
- If your project uses a non-default deployment API version, set `AZURE_FOUNDRY_API_VERSION`.

### Wrong protocol for a custom deployment name

Refresh the provider model list so goose can retrieve `modelPublisher`. Without deployment metadata, routing can only use recognizable model-name prefixes.
