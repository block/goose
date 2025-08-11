# AIML API Provider - Vollständige Goose Integration

## Überblick

Die AIML API wurde als vollständig integrierter Provider in Goose implementiert, sowohl für die CLI als auch für die Desktop GUI. Der Provider bietet Zugang zu 300+ AI-Modellen über eine einheitliche API.

## Implementierte Komponenten

### 1. Backend Provider (Rust)
**Datei:** `crates/goose/src/providers/aimlapi.rs`
- Vollständige OpenAI-kompatible Provider-Implementierung  
- Support für Chat Completions, Streaming und Embeddings
- Unterstützung für alle bekannten AIML API Modelle:
  - OpenAI: gpt-4o, gpt-4o-mini, o1, etc.
  - Anthropic: claude-3-5-sonnet, claude-3-5-haiku, etc.
  - Google: gemini-2.0-flash-exp, gemini-1.5-pro, etc.
  - DeepSeek: deepseek-r1, deepseek-chat, etc.
  - Meta Llama, Qwen, Mistral und mehr

### 2. Backend Integration
**Dateien geändert:**
- `crates/goose/src/providers/mod.rs` - Modul hinzugefügt
- `crates/goose/src/providers/factory.rs` - Provider registriert

### 3. Server API Konfiguration
**Datei:** `crates/goose-server/src/routes/providers_and_keys.json`
```json
"aimlapi": {
    "name": "AIML API",
    "description": "Access 300+ AI models through a single unified API including GPT, Claude, Gemini, DeepSeek, Llama, and more",
    "models": ["gpt-4o", "claude-3-5-sonnet-20241022", "gemini-2.0-flash-exp", ...],
    "required_keys": ["AIMLAPI_API_KEY"]
}
```

### 4. Frontend Provider Registry
**Datei:** `ui/desktop/src/components/settings/providers/ProviderRegistry.tsx`
```typescript
{
  name: 'AIML API',
  details: {
    id: 'aimlapi',
    name: 'AIML API',
    description: 'Access 300+ AI models through a single unified API...',
    parameters: [
      { name: 'AIMLAPI_API_KEY', is_secret: true },
      { name: 'AIMLAPI_HOST', is_secret: false, default: 'https://api.aimlapi.com' },
      { name: 'AIMLAPI_BASE_PATH', is_secret: false, default: 'v1/chat/completions' },
      { name: 'AIMLAPI_TIMEOUT', is_secret: false, default: '600' }
    ]
  }
}
```

### 5. Provider Icons
**Dateien erstellt:**
- `ui/desktop/src/components/settings/providers/modal/subcomponents/icons/aimlapi.svg`
- `ui/desktop/src/components/settings/providers/modal/subcomponents/icons/aimlapi.png` (32x32)
- `ui/desktop/src/components/settings/providers/modal/subcomponents/icons/aimlapi@2x.png` (64x64)
- `ui/desktop/src/components/settings/providers/modal/subcomponents/icons/aimlapi@3x.png` (96x96)

### 6. Logo Integration
**Datei:** `ui/desktop/src/components/settings/providers/modal/subcomponents/ProviderLogo.tsx`
- Import von AIML API Logo hinzugefügt
- Mapping im `providerLogos` Record hinzugefügt

## Konfiguration und Verwendung

### CLI Verwendung
```bash
# API Key setzen
export AIMLAPI_API_KEY="your-api-key-from-aimlapi.com"

# Mit verschiedenen Modellen verwenden
goose session --provider aimlapi --model gpt-4o
goose session --provider aimlapi --model "deepseek/deepseek-r1"
goose session --provider aimlapi --model "claude-3-5-sonnet-20241022"
```

### GUI Konfiguration
1. Goose Desktop App öffnen
2. Settings > Providers navigieren
3. "AIML API" Provider finden
4. "Configure" klicken
5. API Key eingeben (erhalten von https://aimlapi.com/app/keys)
6. Optional: Host, Base Path und Timeout anpassen
7. "Save" klicken

### Verfügbare Umgebungsvariablen
- `AIMLAPI_API_KEY` (erforderlich) - API Key von aimlapi.com
- `AIMLAPI_HOST` (optional) - Standard: https://api.aimlapi.com
- `AIMLAPI_BASE_PATH` (optional) - Standard: v1/chat/completions  
- `AIMLAPI_TIMEOUT` (optional) - Standard: 600 Sekunden
- `AIMLAPI_CUSTOM_HEADERS` (optional) - Zusätzliche Headers

## Unterstützte Features

✅ **Chat Completions** - Vollständige Unterstützung für Text-Generierung  
✅ **Streaming** - Real-time Token-Streaming  
✅ **Embeddings** - Text-zu-Vektor Konvertierung  
✅ **Tool Calling** - Funktionsaufrufe  
✅ **Multiple Models** - Zugang zu 300+ Modellen  
✅ **GUI Integration** - Vollständige Desktop-App Integration  
✅ **Icon Support** - Professionelles AIML API Logo  

## Verfügbare Top-Modelle

### OpenAI Familie
- gpt-4o, gpt-4o-mini
- o1, o1-mini, o1-preview
- gpt-4-turbo, gpt-3.5-turbo

### Anthropic Familie  
- claude-3-5-sonnet-20241022
- claude-3-5-haiku-20241022
- claude-3-opus-20240229

### Google Familie
- gemini-2.0-flash-exp
- gemini-1.5-pro, gemini-1.5-flash

### DeepSeek Familie
- deepseek/deepseek-r1
- deepseek/deepseek-r1-distill-llama-70b
- deepseek/deepseek-chat

### Open Source
- meta-llama/Meta-Llama-3.1-405B-Instruct
- Qwen/Qwen2.5-72B-Instruct  
- mistral-large-latest
- databricks/dbrx-instruct

## Testing

```bash
# CLI Test
echo "What is 2+2?" | goose session --provider aimlapi --model gpt-4o

# Provider Liste anzeigen
goose providers

# Mit verschiedenen Modellen testen  
goose session --provider aimlapi --model "deepseek/deepseek-chat"
goose session --provider aimlapi --model "claude-3-5-sonnet-20241022"
```

## Architektur-Details

Die Implementierung folgt Goose's Provider-Pattern:
1. **Provider Trait Implementation** - Definiert die Standard-Interface
2. **OpenAI Format Compatibility** - Nutzt das bewährte OpenAI-Format
3. **Error Handling** - Vollständiges Error-Mapping und Retry-Logik
4. **Configuration Management** - Sicheres Speichern von API-Keys
5. **GUI Integration** - Nahtlose Integration in die Desktop-App

## Besonderheiten

- **Unified API**: Ein einziger Provider für hunderte von Modellen
- **OpenAI Compatibility**: Vollständig kompatibel mit OpenAI API Standards
- **Enterprise Ready**: Support für Custom Headers und Timeouts
- **Production Tested**: Basiert auf bewährten Goose Provider-Patterns
- **Secure Storage**: API Keys werden sicher im System-Keychain gespeichert

Die Integration ist vollständig und produktionsreif!