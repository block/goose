package config

import (
	"fmt"
	"reflect"
	"strconv"

	"github.com/openlit/openlit/operator/internal/schema"
)

// LoadFromEnvironment loads operator configuration from environment variables
// using the centralized schema for validation, defaults, and type conversion
// Returns error immediately for any invalid configuration to fail fast
func LoadFromEnvironment() (*OperatorConfig, error) {
	// Get the schema
	configSchema := schema.GetOperatorConfigSchema()

	// Load raw values from environment with validation
	rawConfig, errors := configSchema.LoadFromEnvironment()
	if len(errors) > 0 {
		// Combine all validation errors and fail immediately
		var errorMsg string
		for i, err := range errors {
			if i > 0 {
				errorMsg += "; "
			}
			errorMsg += err.Error()
		}
		return nil, fmt.Errorf("operator configuration validation failed - operator cannot start: %s", errorMsg)
	}

	// Convert to typed OperatorConfig struct
	config, err := mapToOperatorConfig(rawConfig)
	if err != nil {
		return nil, fmt.Errorf("failed to map operator configuration - operator cannot start: %w", err)
	}

	// Final validation of the complete config - fail fast on any issues
	if err := config.Validate(); err != nil {
		return nil, fmt.Errorf("operator configuration validation failed - operator cannot start: %w", err)
	}

	return config, nil
}

// mapToOperatorConfig converts the raw config map to a typed OperatorConfig struct
func mapToOperatorConfig(rawConfig map[string]interface{}) (*OperatorConfig, error) {
	config := &OperatorConfig{}

	// Use reflection to set fields based on the raw config
	configValue := reflect.ValueOf(config).Elem()
	configType := reflect.TypeOf(config).Elem()

	for i := 0; i < configValue.NumField(); i++ {
		fieldType := configType.Field(i)
		fieldValue := configValue.Field(i)

		// Get the corresponding value from raw config
		// Convert PascalCase field name to schema field name
		schemaFieldName := fieldType.Name
		rawValue, exists := rawConfig[schemaFieldName]

		if !exists {
			continue // Skip if not found in raw config
		}

		// Set the field value with type checking
		if !fieldValue.CanSet() {
			continue
		}

		switch fieldValue.Kind() {
		case reflect.String:
			if str, ok := rawValue.(string); ok {
				fieldValue.SetString(str)
			} else {
				return nil, fmt.Errorf("field %s expected string, got %T", fieldType.Name, rawValue)
			}

		case reflect.Int:
			if intVal, ok := rawValue.(int); ok {
				fieldValue.SetInt(int64(intVal))
			} else {
				return nil, fmt.Errorf("field %s expected int, got %T", fieldType.Name, rawValue)
			}

		case reflect.Bool:
			if boolVal, ok := rawValue.(bool); ok {
				fieldValue.SetBool(boolVal)
			} else {
				return nil, fmt.Errorf("field %s expected bool, got %T", fieldType.Name, rawValue)
			}

		default:
			return nil, fmt.Errorf("unsupported field type %s for field %s", fieldValue.Kind(), fieldType.Name)
		}
	}

	return config, nil
}

// GetDefaultConfig returns an OperatorConfig with all default values
// This is useful for testing and documentation
func GetDefaultConfig() *OperatorConfig {
	// Load the schema to get defaults
	configSchema := schema.GetOperatorConfigSchema()
	fields := configSchema.GetSchemaFields()

	config := &OperatorConfig{
		// Set defaults based on schema
		WebhookPort:    getIntDefault(fields["WebhookPort"].Default, 9443),
		WebhookPath:    getStringDefault(fields["WebhookPath"].Default, "/mutate"),
		WebhookCertDir: getStringDefault(fields["WebhookCertDir"].Default, "/tmp/k8s-webhook-server/serving-certs"),

		CertValidityDays: getIntDefault(fields["CertValidityDays"].Default, 365),
		CertRefreshDays:  getIntDefault(fields["CertRefreshDays"].Default, 30),

		Namespace:   getStringDefault(fields["Namespace"].Default, "openlit"),
		ServiceName: getStringDefault(fields["ServiceName"].Default, "openlit-webhook-service"),
		SecretName:  getStringDefault(fields["SecretName"].Default, "webhook-server-certs"),
		ConfigName:  getStringDefault(fields["ConfigName"].Default, "openlit-mutating-webhook-configuration"),

		// MetricsPort removed - no metrics server implemented
		HealthPort:            getIntDefault(fields["HealthPort"].Default, 8081),
		SelfMonitoringEnabled: getBoolDefault(fields["SelfMonitoringEnabled"].Default, false),
		LogLevel:              getStringDefault(fields["LogLevel"].Default, "info"),

		OTLPEndpoint:        getStringDefault(fields["OTLPEndpoint"].Default, ""),
		OTLPHeaders:         getStringDefault(fields["OTLPHeaders"].Default, ""),
		OTLPLogsEndpoint:    getStringDefault(fields["OTLPLogsEndpoint"].Default, ""),
		OTLPMetricsEndpoint: getStringDefault(fields["OTLPMetricsEndpoint"].Default, ""),

		WatchNamespace: getStringDefault(fields["WatchNamespace"].Default, ""),

		FailurePolicy:      getStringDefault(fields["FailurePolicy"].Default, "Fail"),
		ReinvocationPolicy: getStringDefault(fields["ReinvocationPolicy"].Default, "Never"),
	}

	return config
}

// Helper functions for safe default value extraction
func getStringDefault(defaultValue, fallback string) string {
	if defaultValue == "" {
		return fallback
	}
	return defaultValue
}

func getIntDefault(defaultValue string, fallback int) int {
	if defaultValue == "" {
		return fallback
	}
	// Convert string to int (should be validated by schema)
	if intVal, err := strconv.Atoi(defaultValue); err == nil {
		return intVal
	}
	return fallback
}

func getBoolDefault(defaultValue string, fallback bool) bool {
	if defaultValue == "" {
		return fallback
	}
	// Convert string to bool (should be validated by schema)
	if boolVal, err := strconv.ParseBool(defaultValue); err == nil {
		return boolVal
	}
	return fallback
}

// ValidateConfig validates an OperatorConfig against the schema
func ValidateConfig(config *OperatorConfig) error {
	if config == nil {
		return fmt.Errorf("config cannot be nil")
	}

	return config.Validate()
}
