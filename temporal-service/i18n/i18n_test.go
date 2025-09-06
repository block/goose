package i18n

import (
	"testing"
)

func TestI18nInitialization(t *testing.T) {
	// Test English initialization
	err := Init("en")
	if err != nil {
		t.Errorf("Failed to initialize i18n with English: %v", err)
	}

	// Test that English messages work
	message := T("StartingTemporalService")
	if message != "Starting Temporal service..." {
		t.Errorf("Expected English message, got: %s", message)
	}

	// Test Portuguese initialization
	err = Init("pt-BR")
	if err != nil {
		t.Errorf("Failed to initialize i18n with Portuguese: %v", err)
	}

	// Test that Portuguese messages work
	message = T("StartingTemporalService")
	if message != "Iniciando serviço Temporal..." {
		t.Errorf("Expected Portuguese message, got: %s", message)
	}

	// Test fallback for unknown locale
	err = Init("fr")
	if err != nil {
		t.Errorf("Failed to initialize i18n with French (should fallback to English): %v", err)
	}

	// Should fallback to English
	message = T("StartingTemporalService")
	if message != "Starting Temporal service..." {
		t.Errorf("Expected English fallback message, got: %s", message)
	}
}

func TestMessageFormatting(t *testing.T) {
	err := Init("en")
	if err != nil {
		t.Fatalf("Failed to initialize i18n: %v", err)
	}

	// Test message with arguments
	message := Tf("RuntimeOS", "darwin")
	if message != "Runtime OS: darwin" {
		t.Errorf("Expected formatted message, got: %s", message)
	}

	// Test Portuguese formatting
	err = Init("pt-BR")
	if err != nil {
		t.Fatalf("Failed to initialize i18n: %v", err)
	}

	message = Tf("RuntimeOS", "darwin")
	if message != "Sistema Operacional: darwin" {
		t.Errorf("Expected formatted Portuguese message, got: %s", message)
	}
}

func TestLocaleValidation(t *testing.T) {
	// Test valid locales
	if !isValidLocale("en") {
		t.Error("English locale should be valid")
	}
	if !isValidLocale("pt-BR") {
		t.Error("Portuguese locale should be valid")
	}

	// Test invalid locales
	if isValidLocale("fr") {
		t.Error("French locale should not be valid")
	}
	if isValidLocale("invalid") {
		t.Error("Invalid locale should not be valid")
	}
}

func TestGetLocale(t *testing.T) {
	// Test default locale
	locale := GetLocale()
	if locale != DefaultLocale {
		t.Errorf("Expected default locale %s, got %s", DefaultLocale, locale)
	}
}
