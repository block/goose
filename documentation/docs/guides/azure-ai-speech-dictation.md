---
title: Azure AI Speech Dictation
description: Configure Azure AI Services Fast Transcription for voice dictation in goose
---

# Azure AI Speech Dictation

The `azure_foundry` dictation provider uses the Azure AI Services Fast Transcription API. This is separate from the Azure AI Foundry LLM inference provider: you can configure either feature independently.

## Required endpoint

Set `AZURE_SPEECH_ENDPOINT` to the endpoint of the Azure AI Services resource that provides Speech:

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
2. `AZURE_FOUNDRY_AD_TOKEN`
3. `AZURE_SPEECH_KEY`
4. `AZURE_FOUNDRY_API_KEY`
5. Azure CLI credentials

A unified Azure AI Foundry resource can use its Foundry key. A separate Azure AI Services resource requires that resource's key in `AZURE_SPEECH_KEY`.

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
3. Enter the Speech endpoint.
4. Enter the Azure AI Services key, or leave it empty to use Microsoft Entra ID.

The provider calls:

```text
POST /speechtotext/transcriptions:transcribe?api-version=2024-11-15
```

Audio is sent as multipart form data using the `audio` part, with optional locale configuration in the `definition` part.
