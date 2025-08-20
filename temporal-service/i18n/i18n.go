package i18n

import (
	"embed"
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/nicksnyder/go-i18n/v2/i18n"
	"golang.org/x/text/language"
)

//go:embed messages/*.json
var messagesFS embed.FS

// Bundle holds the i18n bundle for message localization
var Bundle *i18n.Bundle

// Localizer holds the current localizer instance
var Localizer *i18n.Localizer

// DefaultLocale is the default language for the application
const DefaultLocale = "en"

// SupportedLocales lists all supported languages
var SupportedLocales = []string{"en", "pt-BR"}

// Init initializes the i18n system with the specified locale
func Init(locale string) error {
	// Validate locale
	if !isValidLocale(locale) {
		locale = DefaultLocale
	}

	// Create new bundle
	Bundle = i18n.NewBundle(language.English)

	// Load message files
	if err := loadMessageFiles(); err != nil {
		return fmt.Errorf("failed to load message files: %w", err)
	}

	// Create localizer
	Localizer = i18n.NewLocalizer(Bundle, locale)

	return nil
}

// loadMessageFiles loads all message files from the embedded filesystem
func loadMessageFiles() error {
	entries, err := messagesFS.ReadDir("messages")
	if err != nil {
		return fmt.Errorf("failed to read messages directory: %w", err)
	}

	for _, entry := range entries {
		if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".json") {
			continue
		}

		content, err := messagesFS.ReadFile("messages/" + entry.Name())
		if err != nil {
			return fmt.Errorf("failed to read message file %s: %w", entry.Name(), err)
		}

		// Parse JSON message file
		messageFile, err := i18n.ParseMessageFileBytes(content, entry.Name(), map[string]i18n.UnmarshalFunc{
			"json": json.Unmarshal,
		})
		if err != nil {
			return fmt.Errorf("failed to parse message file %s: %w", entry.Name(), err)
		}
		
		// Add messages to bundle
		for _, message := range messageFile.Messages {
			Bundle.AddMessages(messageFile.Tag, message)
		}
	}

	return nil
}

// isValidLocale checks if the given locale is supported
func isValidLocale(locale string) bool {
	for _, supported := range SupportedLocales {
		if supported == locale {
			return true
		}
	}
	return false
}

// GetLocale returns the current locale from environment variable or default
func GetLocale() string {
	if locale := os.Getenv("GOOSE_LANG"); locale != "" {
		if isValidLocale(locale) {
			return locale
		}
	}
	return DefaultLocale
}

// T returns a localized message for the given message ID
func T(messageID string, args ...interface{}) string {
	if Localizer == nil {
		// Fallback to English if i18n not initialized
		return messageID
	}

	message, err := Localizer.Localize(&i18n.LocalizeConfig{
		MessageID: messageID,
	})
	if err != nil {
		// Fallback to message ID if localization fails
		return messageID
	}

	if len(args) > 0 {
		message = fmt.Sprintf(message, args...)
	}

	return message
}

// Tf returns a localized message with formatting (like fmt.Sprintf)
func Tf(messageID string, args ...interface{}) string {
	return T(messageID, args...)
}
