---
title: Azure AI Speech Dictation
description: Configure Azure AI Speech Fast Transcription for voice dictation in goose
---

# Azure AI Speech Dictation

The `azure_foundry` dictation provider uses the Azure AI Speech Fast Transcription API. This is separate from the Azure AI Foundry LLM inference provider: you can configure either feature independently.

## Required endpoint

Set `AZURE_SPEECH_ENDPOINT` to the HTTPS origin of your Azure AI Speech resource. Do not include an API path, query string, fragment, or credentials:

```sh
export AZURE_SPEECH_ENDPOINT="https://<resource>.cognitiveservices.azure.com"
```

For a unified Foundry resource, goose can derive this endpoint from a project endpoint such as:

```text
https://<resource>.services.ai.azure.com/api/projects/<project>
```

A MaaS endpoint ending in `.models.ai.azure.com` cannot be used to derive a Speech endpoint.

## Authentication

goose selects credentials in this order:

1. `AZURE_SPEECH_AD_TOKEN`
2. `AZURE_SPEECH_KEY`
3. `AZURE_FOUNDRY_AD_TOKEN`, when the Speech endpoint is derived from Foundry or explicitly matches the derived resource
4. `AZURE_FOUNDRY_API_KEY`, with the same compatibility requirement
5. Azure CLI credentials

A unified Azure AI Foundry resource can reuse its Foundry credentials. A separate Azure AI Speech resource requires Speech-specific credentials or Azure CLI authentication; Goose never sends Foundry credentials to an unrelated Speech endpoint.

To use Azure CLI credentials instead of a key:

```sh
az login
```

The access token is requested for `https://cognitiveservices.azure.com`.

## Optional locale

Set a locale to improve recognition for a known language:

```sh
export AZURE_SPEECH_LOCALE="fr-FR"
```

If it is omitted, Azure detects the locale according to the Speech service behavior.

## Desktop setup

1. Open **Settings** → **Chat** → **Voice Dictation Provider**.
2. Select **Azure_foundry**.
3. Enter the Speech endpoint, or keep the endpoint derived from a compatible Foundry resource.
4. Optionally enter a Speech-specific key. Otherwise Goose uses compatible unified Foundry credentials or Azure CLI authentication.

The provider calls:

```text
POST /speechtotext/transcriptions:transcribe?api-version=2024-11-15
```

Audio is sent as multipart form data using the `audio` part, with optional locale configuration in the `definition` part.
