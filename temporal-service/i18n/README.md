# Internationalization (i18n) Support for Goose Temporal Service

This package provides internationalization support for the Goose Temporal Service, allowing the application to display messages in multiple languages.

## Supported Languages

- **English (en)** - Default language
- **Brazilian Portuguese (pt-BR)** - First supported locale

## Usage

### Command Line

Set the language using the `--lang` flag:

```bash
# Use English (default)
./temporal-service

# Use Brazilian Portuguese
./temporal-service --lang pt-BR

# Use environment variable
GOOSE_LANG=pt-BR ./temporal-service
```

### Environment Variable

Set the `GOOSE_LANG` environment variable:

```bash
export GOOSE_LANG=pt-BR
./temporal-service
```

### Programmatic Usage

```go
import "temporal-service/i18n"

// Initialize with specific locale
err := i18n.Init("pt-BR")
if err != nil {
    // Handle error
}

// Get localized message
message := i18n.T("StartingTemporalService")
// Returns: "Iniciando serviço Temporal..."

// Get localized message with formatting
message = i18n.Tf("RuntimeOS", "darwin")
// Returns: "Sistema Operacional: darwin"
```

## Adding New Languages

1. Create a new message file in `messages/` directory:
   ```
   messages/fr.json  # For French
   ```

2. Add the locale to the `SupportedLocales` slice in `i18n.go`:
   ```go
   var SupportedLocales = []string{"en", "pt-BR", "fr"}
   ```

3. Translate all message IDs in the new language file.

## Message Format

Messages use a simple JSON format:

```json
[
  {
    "id": "MessageID",
    "translation": "Translated message text"
  }
]
```

### Formatting

Messages support Go's `fmt.Sprintf` formatting:

```json
{
  "id": "RuntimeOS",
  "translation": "Runtime OS: %s"
}
```

Use `i18n.Tf()` to format messages with arguments:

```go
message := i18n.Tf("RuntimeOS", runtime.GOOS)
```

## Testing

Run the i18n tests:

```bash
go test ./i18n/...
```

## Implementation Details

- Uses `github.com/nicksnyder/go-i18n/v2` library
- Messages are embedded using Go's `embed` package
- Automatic fallback to English for unsupported locales
- Thread-safe message lookup
- Support for both simple messages and formatted messages

## Message IDs

The following message IDs are available for localization:

- `StartingTemporalService` - Service startup message
- `RuntimeOS` - Operating system information
- `RuntimeARCH` - Architecture information
- `CurrentWorkingDirectory` - Working directory
- `CreatingTemporalService` - Service creation message
- `TemporalServerRunningOnPort` - Server status
- `TemporalUIAvailableAt` - UI availability
- And many more...

See the message files for the complete list of translatable strings.
