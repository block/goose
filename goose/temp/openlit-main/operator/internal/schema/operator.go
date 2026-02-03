/*
OpenLIT Operator Configuration Schema

This file defines the centralized schema for the OpenLIT operator's own
infrastructure configuration. It provides a single source of truth for
all operator-specific settings including webhook configuration, TLS
certificates, observability, and multi-operator support.

The schema includes validation rules, default values, and environment
variable mappings to ensure consistent configuration management across
the entire operator lifecycle.

Key components:
- SchemaField: Defines validation and metadata for configuration fields
- OperatorConfigSchema: Complete schema definition with all operator settings
- Environment variable loading and validation
- Type conversion and default value handling

This schema is separate from the AutoInstrumentation CR schema and only
deals with operator infrastructure concerns.
*/
package schema

import (
	"fmt"
	"os"
	"reflect"
	"strconv"
	"strings"
)

// SchemaField defines validation and metadata for a configuration field
type SchemaField struct {
	// Environment variable name
	EnvVar string `json:"env"`

	// Default value as string (will be converted to appropriate type)
	Default string `json:"default,omitempty"`

	// Field type: string, integer, boolean
	Type string `json:"type"`

	// Format for string types: uri, email, etc.
	Format string `json:"format,omitempty"`

	// Validation constraints
	Min      *int64 `json:"min,omitempty"`
	Max      *int64 `json:"max,omitempty"`
	Enum     string `json:"enum,omitempty"` // comma-separated values
	Pattern  string `json:"pattern,omitempty"`
	Required bool   `json:"required,omitempty"`

	// Description for documentation
	Description string `json:"description,omitempty"`
}

// OperatorConfigSchema defines the complete schema for operator configuration
// This is the single source of truth for all operator config fields
type OperatorConfigSchema struct {
	// Webhook Configuration
	WebhookPort    SchemaField
	WebhookPath    SchemaField
	WebhookCertDir SchemaField

	// TLS Certificate Configuration
	CertValidityDays SchemaField
	CertRefreshDays  SchemaField

	// Kubernetes Configuration
	Namespace   SchemaField
	ServiceName SchemaField
	SecretName  SchemaField
	ConfigName  SchemaField

	// Observability Configuration
	// MetricsPort removed - no metrics server implemented
	HealthPort            SchemaField
	SelfMonitoringEnabled SchemaField
	LogLevel              SchemaField

	// OpenTelemetry Configuration (for operator self-monitoring)
	OTLPEndpoint        SchemaField
	OTLPHeaders         SchemaField
	OTLPLogsEndpoint    SchemaField
	OTLPMetricsEndpoint SchemaField

	// Multi-Operator Support
	WatchNamespace SchemaField

	// Webhook Behavior Configuration
	FailurePolicy      SchemaField
	ReinvocationPolicy SchemaField
}

// GetOperatorConfigSchema returns the complete operator configuration schema
// This centralizes all operator config validation, defaults, and metadata
func GetOperatorConfigSchema() OperatorConfigSchema {
	return OperatorConfigSchema{
		// Webhook Configuration
		WebhookPort: SchemaField{
			EnvVar:      "WEBHOOK_PORT",
			Default:     "9443",
			Type:        "integer",
			Min:         int64Ptr(1),
			Max:         int64Ptr(65535),
			Required:    false,
			Description: "Port for the admission webhook server",
		},
		WebhookPath: SchemaField{
			EnvVar:      "WEBHOOK_PATH",
			Default:     "/mutate",
			Type:        "string",
			Pattern:     "^/.*",
			Required:    false,
			Description: "Path for the webhook endpoint",
		},
		WebhookCertDir: SchemaField{
			EnvVar:      "WEBHOOK_CERT_DIR",
			Default:     "/tmp/k8s-webhook-server/serving-certs",
			Type:        "string",
			Required:    false,
			Description: "Directory where webhook certificates are stored",
		},

		// TLS Certificate Configuration
		CertValidityDays: SchemaField{
			EnvVar:      "CERT_VALIDITY_DAYS",
			Default:     "365",
			Type:        "integer",
			Min:         int64Ptr(1),
			Max:         int64Ptr(3650), // Max 10 years
			Required:    false,
			Description: "Validity period for generated certificates in days",
		},
		CertRefreshDays: SchemaField{
			EnvVar:      "CERT_REFRESH_DAYS",
			Default:     "30",
			Type:        "integer",
			Min:         int64Ptr(1),
			Max:         int64Ptr(365),
			Required:    false,
			Description: "Days before certificate expiry to trigger refresh",
		},

		// Kubernetes Configuration
		Namespace: SchemaField{
			EnvVar:      "POD_NAMESPACE",
			Default:     "openlit",
			Type:        "string",
			Pattern:     "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$",
			Required:    false,
			Description: "Kubernetes namespace where the operator is deployed",
		},
		ServiceName: SchemaField{
			EnvVar:      "WEBHOOK_SERVICE_NAME",
			Default:     "openlit-webhook-service",
			Type:        "string",
			Pattern:     "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$",
			Required:    false,
			Description: "Name of the webhook service",
		},
		SecretName: SchemaField{
			EnvVar:      "WEBHOOK_SECRET_NAME",
			Default:     "webhook-server-certs",
			Type:        "string",
			Pattern:     "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$",
			Required:    false,
			Description: "Name of the secret containing webhook certificates",
		},
		ConfigName: SchemaField{
			EnvVar:      "WEBHOOK_CONFIG_NAME",
			Default:     "openlit-mutating-webhook-configuration",
			Type:        "string",
			Pattern:     "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$",
			Required:    false,
			Description: "Name of the mutating webhook configuration",
		},

		// Observability Configuration
		// MetricsPort removed - no metrics server is implemented
		HealthPort: SchemaField{
			EnvVar:      "HEALTH_PORT",
			Default:     "8081",
			Type:        "integer",
			Min:         int64Ptr(1),
			Max:         int64Ptr(65535),
			Required:    false,
			Description: "Port for health check endpoints",
		},
		SelfMonitoringEnabled: SchemaField{
			EnvVar:      "SELF_MONITORING_ENABLED",
			Default:     "false",
			Type:        "boolean",
			Required:    false,
			Description: "Enable self-monitoring with OpenTelemetry",
		},
		LogLevel: SchemaField{
			EnvVar:      "LOG_LEVEL",
			Default:     "info",
			Type:        "string",
			Enum:        "debug,info,warn,error",
			Required:    false,
			Description: "Logging level for the operator",
		},

		// OpenTelemetry Configuration (using official OTEL env vars)
		OTLPEndpoint: SchemaField{
			EnvVar:      "OTEL_EXPORTER_OTLP_ENDPOINT",
			Default:     "",
			Type:        "string",
			Format:      "uri",
			Required:    false,
			Description: "OTLP endpoint for operator telemetry (logs, metrics, traces)",
		},
		OTLPHeaders: SchemaField{
			EnvVar:      "OTEL_EXPORTER_OTLP_HEADERS",
			Default:     "",
			Type:        "string",
			Required:    false,
			Description: "OTLP headers (key=value,key2=value2 format)",
		},
		OTLPLogsEndpoint: SchemaField{
			EnvVar:      "OTEL_EXPORTER_OTLP_LOGS_ENDPOINT",
			Default:     "",
			Type:        "string",
			Format:      "uri",
			Required:    false,
			Description: "Separate OTLP endpoint for logs (if different from main endpoint)",
		},
		OTLPMetricsEndpoint: SchemaField{
			EnvVar:      "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
			Default:     "",
			Type:        "string",
			Format:      "uri",
			Required:    false,
			Description: "Separate OTLP endpoint for metrics (if different from main endpoint)",
		},

		// Multi-Operator Support
		WatchNamespace: SchemaField{
			EnvVar:      "WATCH_NAMESPACE",
			Default:     "",
			Type:        "string",
			Pattern:     "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$",
			Required:    false,
			Description: "Namespace to watch for AutoInstrumentation resources (empty = all namespaces)",
		},

		// Webhook Behavior Configuration
		FailurePolicy: SchemaField{
			EnvVar:      "WEBHOOK_FAILURE_POLICY",
			Default:     "Fail",
			Type:        "string",
			Enum:        "Fail,Ignore",
			Required:    false,
			Description: "Webhook failure policy (Fail or Ignore)",
		},
		ReinvocationPolicy: SchemaField{
			EnvVar:      "WEBHOOK_REINVOCATION_POLICY",
			Default:     "Never",
			Type:        "string",
			Enum:        "Never,IfNeeded",
			Required:    false,
			Description: "Webhook reinvocation policy (Never or IfNeeded)",
		},
	}
}

// ValidateField validates a field value according to its schema
func (sf SchemaField) ValidateField(value interface{}) error {
	// Convert value to string for validation
	strValue := fmt.Sprintf("%v", value)

	// Check required fields
	if sf.Required && (value == nil || strValue == "") {
		return fmt.Errorf("field is required but not provided")
	}

	// Skip validation for empty optional fields
	if strValue == "" && !sf.Required {
		return nil
	}

	// Type-specific validation
	switch sf.Type {
	case "integer":
		intValue, err := strconv.ParseInt(strValue, 10, 64)
		if err != nil {
			return fmt.Errorf("invalid integer value: %s", strValue)
		}

		if sf.Min != nil && intValue < *sf.Min {
			return fmt.Errorf("value %d is below minimum %d", intValue, *sf.Min)
		}

		if sf.Max != nil && intValue > *sf.Max {
			return fmt.Errorf("value %d is above maximum %d", intValue, *sf.Max)
		}

	case "boolean":
		_, err := strconv.ParseBool(strValue)
		if err != nil {
			return fmt.Errorf("invalid boolean value: %s", strValue)
		}

	case "string":
		// Enum validation
		if sf.Enum != "" {
			enumValues := strings.Split(sf.Enum, ",")
			valid := false
			for _, enumValue := range enumValues {
				if strings.TrimSpace(enumValue) == strValue {
					valid = true
					break
				}
			}
			if !valid {
				return fmt.Errorf("value %s is not in allowed values: %s", strValue, sf.Enum)
			}
		}

		// Pattern validation would go here if needed
		// if sf.Pattern != "" { ... }
	}

	return nil
}

// GetSchemaFields returns all schema fields using reflection
// This allows for programmatic access to all configuration fields
func (schema OperatorConfigSchema) GetSchemaFields() map[string]SchemaField {
	fields := make(map[string]SchemaField)

	schemaValue := reflect.ValueOf(schema)
	schemaType := reflect.TypeOf(schema)

	for i := 0; i < schemaValue.NumField(); i++ {
		fieldName := schemaType.Field(i).Name
		fieldValue := schemaValue.Field(i).Interface().(SchemaField)
		fields[fieldName] = fieldValue
	}

	return fields
}

// GetEnvVarMapping returns a mapping of environment variable names to field names
func (schema OperatorConfigSchema) GetEnvVarMapping() map[string]string {
	mapping := make(map[string]string)
	fields := schema.GetSchemaFields()

	for fieldName, schemaField := range fields {
		if schemaField.EnvVar != "" {
			mapping[schemaField.EnvVar] = fieldName
		}
	}

	return mapping
}

// LoadFromEnvironment loads configuration values from environment variables
// using the schema for validation and defaults
func (schema OperatorConfigSchema) LoadFromEnvironment() (map[string]interface{}, []error) {
	config := make(map[string]interface{})
	var errors []error

	fields := schema.GetSchemaFields()

	for fieldName, schemaField := range fields {
		// Get value from environment or use default
		value := os.Getenv(schemaField.EnvVar)
		if value == "" {
			value = schemaField.Default
		}

		// Convert and validate based on type
		var convertedValue interface{}
		var err error

		switch schemaField.Type {
		case "integer":
			if value != "" {
				convertedValue, err = strconv.Atoi(value)
			}
		case "boolean":
			if value != "" {
				convertedValue, err = strconv.ParseBool(value)
			}
		case "string":
			convertedValue = value
		default:
			err = fmt.Errorf("unknown type %s for field %s", schemaField.Type, fieldName)
		}

		if err != nil {
			errors = append(errors, fmt.Errorf("field %s: %w", fieldName, err))
			continue
		}

		// Validate the converted value
		if err := schemaField.ValidateField(convertedValue); err != nil {
			errors = append(errors, fmt.Errorf("field %s: %w", fieldName, err))
			continue
		}

		config[fieldName] = convertedValue
	}

	return config, errors
}

// int64Ptr returns a pointer to an int64 value
// Helper function for setting Min/Max constraints
func int64Ptr(value int64) *int64 {
	return &value
}
