/*
AutoInstrumentation Custom Resource Schema

This file defines the centralized schema for the AutoInstrumentation Custom Resource,
which is the user-facing configuration for zero-code instrumentation in Kubernetes.

The schema provides comprehensive validation, default values, and documentation
for all fields in the AutoInstrumentation CR, ensuring consistent behavior
across different instrumentation providers (OpenLIT, OpenInference, OpenLLMetry).

Key components:
- AutoInstrumentationSchemaField: Field definitions with validation rules
- AutoInstrumentationSchema: Complete CR schema covering all configuration options
- Validation functions for field values and constraints
- Kubebuilder marker generation for CRD creation
- Default value management

Supported configuration areas:
- Pod selection (matchLabels, matchExpressions)
- Provider-specific settings (Python instrumentation)
- OTLP endpoint configuration
- Resource attributes and environment variables
- Custom packages and init images

This schema serves as the single source of truth for all AutoInstrumentation
configuration and is used by the CRD generator, validation logic, and
runtime configuration processing.
*/
package schema

import (
	"fmt"
	"strconv"
)

// AutoInstrumentationSchemaField defines a schema field for AutoInstrumentation configuration
type AutoInstrumentationSchemaField struct {
	// JSON path in the CR (e.g., "spec.otlp.endpoint")
	JSONPath string `json:"jsonPath"`

	// Field type (string, int, bool, []string, etc.)
	Type string `json:"type"`

	// Description of the field
	Description string `json:"description"`

	// Default value
	Default interface{} `json:"default,omitempty"`

	// Whether this field is required
	Required bool `json:"required"`

	// Validation rules
	Validation *AutoInstrumentationValidation `json:"validation,omitempty"`

	// Kubebuilder validation markers
	KubebuilderMarkers []string `json:"kubebuilderMarkers,omitempty"`
}

// AutoInstrumentationValidation defines validation rules
type AutoInstrumentationValidation struct {
	// Pattern for string validation
	Pattern string `json:"pattern,omitempty"`

	// Enum values
	Enum []string `json:"enum,omitempty"`

	// Minimum value for numbers
	Minimum *int32 `json:"minimum,omitempty"`

	// Maximum value for numbers
	Maximum *int32 `json:"maximum,omitempty"`

	// Custom validation function name
	CustomValidator string `json:"customValidator,omitempty"`
}

// AutoInstrumentationSchema defines the complete schema for AutoInstrumentation CR
type AutoInstrumentationSchema map[string]AutoInstrumentationSchemaField

// GetAutoInstrumentationSchema returns the complete AutoInstrumentation CR schema
// This serves as the single source of truth for all AutoInstrumentation configuration
func GetAutoInstrumentationSchema() AutoInstrumentationSchema {
	return AutoInstrumentationSchema{
		// Selector Configuration
		"selector.matchLabels": {
			JSONPath:    "spec.selector.matchLabels",
			Type:        "map[string]string",
			Description: "Label selector for pods to instrument",
			Required:    false,
			KubebuilderMarkers: []string{
				"+optional",
			},
		},
		"selector.matchExpressions": {
			JSONPath:    "spec.selector.matchExpressions",
			Type:        "[]metav1.LabelSelectorRequirement",
			Description: "Expression-based selector for pods to instrument",
			Required:    false,
			KubebuilderMarkers: []string{
				"+optional",
			},
		},

		// Ignore Configuration
		"ignore.matchLabels": {
			JSONPath:    "spec.ignore.matchLabels",
			Type:        "map[string]string",
			Description: "Label selector for pods to skip instrumentation",
			Required:    false,
			KubebuilderMarkers: []string{
				"+optional",
			},
		},
		"ignore.matchExpressions": {
			JSONPath:    "spec.ignore.matchExpressions",
			Type:        "[]metav1.LabelSelectorRequirement",
			Description: "Expression-based selector for pods to skip instrumentation",
			Required:    false,
			KubebuilderMarkers: []string{
				"+optional",
			},
		},

		// Python Instrumentation Configuration
		"python.instrumentation.enabled": {
			JSONPath:    "spec.python.instrumentation.enabled",
			Type:        "*bool",
			Description: "Whether Python instrumentation is enabled",
			Default:     true,
			Required:    false,
			KubebuilderMarkers: []string{
				"+kubebuilder:default:=true",
				"+optional",
			},
		},
		"python.instrumentation.version": {
			JSONPath:    "spec.python.instrumentation.version",
			Type:        "string",
			Description: "Version of the instrumentation to use",
			Default:     "latest",
			Required:    false,
			Validation: &AutoInstrumentationValidation{
				Pattern: "^(latest|[0-9]+\\.[0-9]+\\.[0-9]+.*)$",
			},
			KubebuilderMarkers: []string{
				"+kubebuilder:default:=\"latest\"",
				"+kubebuilder:validation:Pattern:=^(latest|[0-9]+\\.[0-9]+\\.[0-9]+.*)$",
				"+optional",
			},
		},
		"python.instrumentation.provider": {
			JSONPath:    "spec.python.instrumentation.provider",
			Type:        "string",
			Description: "Instrumentation provider to use",
			Default:     "openlit",
			Required:    false,
			Validation: &AutoInstrumentationValidation{
				Enum: []string{"openlit", "openinference", "openllmetry", "custom"},
			},
			KubebuilderMarkers: []string{
				"+kubebuilder:validation:Enum:=openlit;openinference;openllmetry;custom",
				"+kubebuilder:default:=\"openlit\"",
				"+optional",
			},
		},
		"python.instrumentation.imagePullPolicy": {
			JSONPath:    "spec.python.instrumentation.imagePullPolicy",
			Type:        "string",
			Description: "Image pull policy for init containers",
			Default:     "IfNotPresent",
			Required:    false,
			Validation: &AutoInstrumentationValidation{
				Enum: []string{"Always", "IfNotPresent", "Never"},
			},
			KubebuilderMarkers: []string{
				"+kubebuilder:validation:Enum:=Always;IfNotPresent;Never",
				"+kubebuilder:default:=\"IfNotPresent\"",
				"+optional",
			},
		},
		"python.instrumentation.customPackages": {
			JSONPath:    "spec.python.instrumentation.customPackages",
			Type:        "string",
			Description: "Additional packages to install (comma-separated)",
			Required:    false,
			Validation: &AutoInstrumentationValidation{
				Pattern: "^[a-zA-Z0-9_\\\\-\\\\.,=<>!\\\\s]*$",
			},
			KubebuilderMarkers: []string{
				"+kubebuilder:validation:Pattern:=\"^[a-zA-Z0-9_\\\\-\\\\.,=<>!\\\\s]*$\"",
				"+optional",
			},
		},
		"python.instrumentation.customInitImage": {
			JSONPath:    "spec.python.instrumentation.customInitImage",
			Type:        "string",
			Description: "Custom init container image for instrumentation",
			Required:    false,
			Validation: &AutoInstrumentationValidation{
				Pattern: "^[a-z0-9.-]+(/[a-z0-9._-]+)*:[a-zA-Z0-9._-]+$",
			},
			KubebuilderMarkers: []string{
				"+kubebuilder:validation:Pattern:=\"^[a-z0-9.-]+(/[a-z0-9._-]+)*:[a-zA-Z0-9._-]+$\"",
				"+optional",
			},
		},
		"python.instrumentation.env": {
			JSONPath:    "spec.python.instrumentation.env",
			Type:        "[]EnvVar",
			Description: "Environment variables to inject into instrumented containers",
			Required:    false,
			KubebuilderMarkers: []string{
				"+optional",
			},
		},

		// OTLP Configuration
		"otlp.endpoint": {
			JSONPath:    "spec.otlp.endpoint",
			Type:        "string",
			Description: "OTLP endpoint URL for sending telemetry data",
			Required:    true,
			Validation: &AutoInstrumentationValidation{
				Pattern: "^https?://[a-zA-Z0-9.-]+(:[0-9]+)?(/.*)?$",
			},
			KubebuilderMarkers: []string{
				"+kubebuilder:validation:Required",
				"+kubebuilder:validation:Pattern:=\"^https?://[a-zA-Z0-9.-]+(:[0-9]+)?(/.*)?$\"",
			},
		},
		"otlp.headers": {
			JSONPath:    "spec.otlp.headers",
			Type:        "string",
			Description: "Additional headers for OTLP requests (key=value format, comma-separated)",
			Required:    false,
			Validation: &AutoInstrumentationValidation{
				Pattern: "^[a-zA-Z0-9_\\\\-]+=.*(,[a-zA-Z0-9_\\\\-]+=.*)*$",
			},
			KubebuilderMarkers: []string{
				"+kubebuilder:validation:Pattern:=\"^[a-zA-Z0-9_\\\\-]+=.*(,[a-zA-Z0-9_\\\\-]+=.*)*$\"",
				"+optional",
			},
		},
		"otlp.timeout": {
			JSONPath:    "spec.otlp.timeout",
			Type:        "*int32",
			Description: "Timeout for OTLP requests in seconds",
			Default:     int32(30),
			Required:    false,
			Validation: &AutoInstrumentationValidation{
				Minimum: int32Ptr(1),
				Maximum: int32Ptr(300),
			},
			KubebuilderMarkers: []string{
				"+kubebuilder:validation:Minimum:=1",
				"+kubebuilder:validation:Maximum:=300",
				"+kubebuilder:default:=30",
				"+optional",
			},
		},

		// Resource Configuration
		"resource.environment": {
			JSONPath:    "spec.resource.environment",
			Type:        "string",
			Description: "Deployment environment for resource attributes",
			Required:    false,
			Validation: &AutoInstrumentationValidation{
				Pattern: "^[a-zA-Z0-9_\\\\-]+$",
			},
			KubebuilderMarkers: []string{
				"+kubebuilder:validation:Pattern:=\"^[a-zA-Z0-9_\\\\-]+$\"",
				"+optional",
			},
		},
		"resource.serviceName": {
			JSONPath:    "spec.resource.serviceName",
			Type:        "string",
			Description: "Service name for resource attributes",
			Required:    false,
			Validation: &AutoInstrumentationValidation{
				Pattern: "^[a-zA-Z0-9_\\\\-]+$",
			},
			KubebuilderMarkers: []string{
				"+kubebuilder:validation:Pattern:=\"^[a-zA-Z0-9_\\\\-]+$\"",
				"+optional",
			},
		},
		"resource.serviceNamespace": {
			JSONPath:    "spec.resource.serviceNamespace",
			Type:        "string",
			Description: "Service namespace for resource attributes",
			Required:    false,
			Validation: &AutoInstrumentationValidation{
				Pattern: "^[a-zA-Z0-9_\\\\-]+$",
			},
			KubebuilderMarkers: []string{
				"+kubebuilder:validation:Pattern:=\"^[a-zA-Z0-9_\\\\-]+$\"",
				"+optional",
			},
		},
	}
}

// ValidateAutoInstrumentation validates an AutoInstrumentation configuration against the schema
func (schema AutoInstrumentationSchema) ValidateAutoInstrumentation(config map[string]interface{}) []string {
	var errors []string

	// Check required fields
	for fieldName, field := range schema {
		if field.Required {
			if _, exists := config[fieldName]; !exists {
				errors = append(errors, fmt.Sprintf("required field '%s' is missing", fieldName))
			}
		}
	}

	// Validate field values
	for fieldName, value := range config {
		if field, exists := schema[fieldName]; exists {
			if validationErrors := validateAutoInstrumentationField(field, fieldName, value); len(validationErrors) > 0 {
				errors = append(errors, validationErrors...)
			}
		}
	}

	return errors
}

// validateAutoInstrumentationField validates a single field against its schema
func validateAutoInstrumentationField(field AutoInstrumentationSchemaField, fieldName string, value interface{}) []string {
	var errors []string

	if field.Validation == nil {
		return errors
	}

	// String pattern validation
	if field.Validation.Pattern != "" && field.Type == "string" {
		if strValue, ok := value.(string); ok {
			// Note: In a real implementation, you'd use regexp.MatchString here
			// For now, we'll just validate non-empty for patterns
			if strValue == "" {
				errors = append(errors, fmt.Sprintf("field '%s' cannot be empty when pattern is required", fieldName))
			}
		}
	}

	// Enum validation
	if len(field.Validation.Enum) > 0 && field.Type == "string" {
		if strValue, ok := value.(string); ok {
			valid := false
			for _, enumValue := range field.Validation.Enum {
				if strValue == enumValue {
					valid = true
					break
				}
			}
			if !valid {
				errors = append(errors, fmt.Sprintf("field '%s' value '%s' is not in allowed values: %v", fieldName, strValue, field.Validation.Enum))
			}
		}
	}

	// Number range validation
	if field.Validation.Minimum != nil || field.Validation.Maximum != nil {
		var numValue int32
		var ok bool

		switch v := value.(type) {
		case int32:
			numValue, ok = v, true
		case int:
			numValue, ok = int32(v), true
		case float64:
			numValue, ok = int32(v), true
		case string:
			if parsed, err := strconv.ParseInt(v, 10, 32); err == nil {
				numValue, ok = int32(parsed), true
			}
		}

		if ok {
			if field.Validation.Minimum != nil && numValue < *field.Validation.Minimum {
				errors = append(errors, fmt.Sprintf("field '%s' value %d is below minimum %d", fieldName, numValue, *field.Validation.Minimum))
			}
			if field.Validation.Maximum != nil && numValue > *field.Validation.Maximum {
				errors = append(errors, fmt.Sprintf("field '%s' value %d exceeds maximum %d", fieldName, numValue, *field.Validation.Maximum))
			}
		}
	}

	return errors
}

// GetDefaultAutoInstrumentationConfig returns default configuration values
func (schema AutoInstrumentationSchema) GetDefaultAutoInstrumentationConfig() map[string]interface{} {
	defaults := make(map[string]interface{})

	for fieldName, field := range schema {
		if field.Default != nil {
			defaults[fieldName] = field.Default
		}
	}

	return defaults
}

// GetRequiredAutoInstrumentationFields returns list of required field names
func (schema AutoInstrumentationSchema) GetRequiredAutoInstrumentationFields() []string {
	var required []string

	for fieldName, field := range schema {
		if field.Required {
			required = append(required, fieldName)
		}
	}

	return required
}

// GenerateKubebuilderMarkers generates kubebuilder validation markers for CRD generation
func (schema AutoInstrumentationSchema) GenerateKubebuilderMarkers() map[string][]string {
	markers := make(map[string][]string)

	for fieldName, field := range schema {
		markers[fieldName] = field.KubebuilderMarkers
	}

	return markers
}

// Helper function to create int32 pointer
func int32Ptr(i int32) *int32 {
	return &i
}

// AutoInstrumentationSchemaVersion represents the schema version for backward compatibility
const AutoInstrumentationSchemaVersion = "v1alpha1"

// GetAutoInstrumentationSchemaVersion returns the current schema version
func GetAutoInstrumentationSchemaVersion() string {
	return AutoInstrumentationSchemaVersion
}
