/*
Copyright 2024.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

package validation

import (
	"fmt"
	"net/url"
	"strings"

	"github.com/openlit/openlit/operator/api/v1alpha1"
	"github.com/openlit/openlit/operator/internal/observability"
	"go.opentelemetry.io/otel/sdk/resource"
	semconv "go.opentelemetry.io/otel/semconv/v1.24.0"
)

// ValidationResult represents the result of validation
type ValidationResult struct {
	Valid    bool
	Errors   []string
	Warnings []string
}

// AutoInstrumentationValidator validates AutoInstrumentation resources
type AutoInstrumentationValidator struct {
	logger *observability.StructuredLogger
}

// NewAutoInstrumentationValidator creates a new validator
func NewAutoInstrumentationValidator() *AutoInstrumentationValidator {
	logger := observability.NewLogger("autoinstrumentation-validator")
	return &AutoInstrumentationValidator{
		logger: logger,
	}
}

// Validate validates the entire AutoInstrumentation
func (v *AutoInstrumentationValidator) Validate(config *v1alpha1.AutoInstrumentation) *ValidationResult {
	result := &ValidationResult{
		Valid:    true,
		Errors:   []string{},
		Warnings: []string{},
	}

	v.logger.Info("Starting AutoInstrumentation validation",
		"component", "validation",
		"config.name", config.Name,
		"config.namespace", config.Namespace,
		"k8s.namespace.name", config.Namespace,
		"k8s.object.name", config.Name)

	// Validate required fields
	v.validateRequiredFields(config, result)

	// Validate selector
	v.validateSelector(config.Spec.Selector, result)

	// Validate ignore selector if provided
	if config.Spec.Ignore != nil {
		v.validateIgnoreSelector(*config.Spec.Ignore, result)
	}

	// Validate Python instrumentation
	if config.Spec.Python != nil {
		v.validatePythonInstrumentation(config.Spec.Python, result)
	}

	// Validate OTLP configuration
	v.validateOTLPConfig(config.Spec.OTLP, result)

	// Validate resource configuration
	if config.Spec.Resource != nil {
		v.validateResourceConfig(config.Spec.Resource, result)
	}

	// Log validation summary
	if len(result.Errors) > 0 {
		result.Valid = false
		v.logger.Error("AutoInstrumentation validation failed", fmt.Errorf("validation failed with %d errors", len(result.Errors)),
			"component", "validation",
			"config.name", config.Name,
			"config.namespace", config.Namespace,
			"validation.errors.count", len(result.Errors),
			"validation.warnings.count", len(result.Warnings))
	} else {
		v.logger.Info("AutoInstrumentation validation passed",
			"component", "validation",
			"config.name", config.Name,
			"config.namespace", config.Namespace,
			"validation.warnings.count", len(result.Warnings))
	}

	return result
}

// validateRequiredFields validates that all required fields are present
func (v *AutoInstrumentationValidator) validateRequiredFields(config *v1alpha1.AutoInstrumentation, result *ValidationResult) {
	// Check if selector is provided
	if len(config.Spec.Selector.MatchLabels) == 0 && len(config.Spec.Selector.MatchExpressions) == 0 {
		error := "spec.selector must have either matchLabels or matchExpressions"
		result.Errors = append(result.Errors, error)
		v.logger.Error("🚫 Missing required selector", fmt.Errorf("%s", error),
			"component", "validation",
			"validation.field", "spec.selector",
			"config.name", config.Name)
	}

	// Check if OTLP endpoint is provided
	if config.Spec.OTLP.Endpoint == "" {
		error := "spec.otlp.endpoint is required"
		result.Errors = append(result.Errors, error)
		v.logger.Error("🚫 Missing required OTLP endpoint", fmt.Errorf("%s", error),
			"component", "validation",
			"validation.field", "spec.otlp.endpoint",
			"config.name", config.Name)
	}
}

// validateSelector validates pod selector configuration
func (v *AutoInstrumentationValidator) validateSelector(selector v1alpha1.PodSelector, result *ValidationResult) {
	// Validate matchLabels
	for key, value := range selector.MatchLabels {
		if key == "" {
			error := "matchLabels keys cannot be empty"
			result.Errors = append(result.Errors, error)
			v.logger.Error("🚫 Invalid matchLabels key", fmt.Errorf("%s", error),
				"component", "validation",
				"validation.field", "spec.selector.matchLabels",
				"label.key", key,
				"label.value", value)
		}
	}

	// Validate matchExpressions
	for i, expr := range selector.MatchExpressions {
		if expr.Key == "" {
			error := fmt.Sprintf("matchExpressions[%d].key cannot be empty", i)
			result.Errors = append(result.Errors, error)
			v.logger.Error("🚫 Invalid matchExpressions key", fmt.Errorf("%s", error),
				"component", "validation",
				"validation.field", "spec.selector.matchExpressions",
				"expression.index", i)
		}
	}
}

// validateIgnoreSelector validates ignore selector configuration
func (v *AutoInstrumentationValidator) validateIgnoreSelector(ignore v1alpha1.PodSelector, result *ValidationResult) {
	// Use same validation logic as regular selector
	v.validateSelector(ignore, result)

	// Additional warning if ignore selector is too broad
	if len(ignore.MatchLabels) == 0 && len(ignore.MatchExpressions) == 0 {
		warning := "ignore selector with no criteria will ignore all pods"
		result.Warnings = append(result.Warnings, warning)
		v.logger.Warn("Broad ignore selector",
			"component", "validation",
			"validation.field", "spec.ignore",
			"validation.warning", warning)
	}
}

// validatePythonInstrumentation validates Python instrumentation configuration
func (v *AutoInstrumentationValidator) validatePythonInstrumentation(python *v1alpha1.PythonInstrumentation, result *ValidationResult) {
	if python.Instrumentation == nil {
		return
	}

	instrumentation := python.Instrumentation

	// Validate version format
	if instrumentation.Version != "" && instrumentation.Version != "latest" {
		// Simple semantic version validation
		if !strings.Contains(instrumentation.Version, ".") {
			warning := "version should follow semantic versioning (e.g., 1.0.0)"
			result.Warnings = append(result.Warnings, warning)
			v.logger.Warn("Non-semantic version format",
				"component", "validation",
				"validation.field", "spec.python.instrumentation.version",
				"validation.warning", warning,
				"version", instrumentation.Version)
		}
	}

	// Validate provider-specific settings
	if instrumentation.Provider == "custom" && instrumentation.CustomInitImage == "" {
		error := "customInitImage is required when provider is 'custom'"
		result.Errors = append(result.Errors, error)
		v.logger.Error("🚫 Missing custom init image", fmt.Errorf("%s", error),
			"component", "validation",
			"validation.field", "spec.python.instrumentation",
			"provider", instrumentation.Provider)
	}

	// Validate environment variables
	v.validateEnvironmentVariables(instrumentation.Env, result)
}

// validateEnvironmentVariables validates environment variable configuration
func (v *AutoInstrumentationValidator) validateEnvironmentVariables(envVars []v1alpha1.EnvVar, result *ValidationResult) {
	envNames := make(map[string]bool)

	for i, envVar := range envVars {
		// Check for duplicate names
		if envNames[envVar.Name] {
			error := fmt.Sprintf("duplicate environment variable name '%s' at position %d", envVar.Name, i)
			result.Errors = append(result.Errors, error)
			v.logger.Error("🚫 Duplicate environment variable", fmt.Errorf("%s", error),
				"component", "validation",
				"validation.field", "spec.python.instrumentation.env",
				"env.name", envVar.Name,
				"env.index", i)
		}
		envNames[envVar.Name] = true

		// Validate that either value or valueFrom is set, but not both
		hasValue := envVar.Value != ""
		hasValueFrom := envVar.ValueFrom != nil

		if !hasValue && !hasValueFrom {
			error := fmt.Sprintf("environment variable '%s' must have either 'value' or 'valueFrom' set", envVar.Name)
			result.Errors = append(result.Errors, error)
			v.logger.Error("🚫 Missing environment variable value", fmt.Errorf("%s", error),
				"component", "validation",
				"validation.field", "spec.python.instrumentation.env",
				"env.name", envVar.Name,
				"env.index", i)
		}

		if hasValue && hasValueFrom {
			error := fmt.Sprintf("environment variable '%s' cannot have both 'value' and 'valueFrom' set", envVar.Name)
			result.Errors = append(result.Errors, error)
			v.logger.Error("🚫 Conflicting environment variable sources", fmt.Errorf("%s", error),
				"component", "validation",
				"validation.field", "spec.python.instrumentation.env",
				"env.name", envVar.Name,
				"env.index", i)
		}

		// Check for reserved environment variables
		reservedVars := []string{
			"OTEL_EXPORTER_OTLP_ENDPOINT",
			"OTEL_EXPORTER_OTLP_HEADERS",
			"OTEL_EXPORTER_OTLP_TIMEOUT",
			"OTEL_RESOURCE_ATTRIBUTES",
			"OTEL_SERVICE_NAME",
			"OTEL_SERVICE_VERSION",
			"PYTHONPATH",
		}

		for _, reserved := range reservedVars {
			if envVar.Name == reserved {
				warning := fmt.Sprintf("environment variable '%s' is reserved and may be overridden by the operator", envVar.Name)
				result.Warnings = append(result.Warnings, warning)
				v.logger.Warn("Reserved environment variable",
					"component", "validation",
					"validation.field", "spec.python.instrumentation.env",
					"validation.warning", warning,
					"env.name", envVar.Name,
					"env.index", i)
				break
			}
		}
	}
}

// validateOTLPConfig validates OTLP configuration
func (v *AutoInstrumentationValidator) validateOTLPConfig(otlp v1alpha1.OTLPConfig, result *ValidationResult) {
	// Validate endpoint URL
	if otlp.Endpoint != "" {
		if _, err := url.Parse(otlp.Endpoint); err != nil {
			error := fmt.Sprintf("invalid OTLP endpoint URL: %v", err)
			result.Errors = append(result.Errors, error)
			v.logger.Error("🚫 Invalid OTLP endpoint", fmt.Errorf("%s", error),
				"component", "validation",
				"validation.field", "spec.otlp.endpoint",
				"endpoint", otlp.Endpoint)
		}

		// Check for common mistakes
		if strings.HasSuffix(otlp.Endpoint, "/v1/traces") {
			warning := "OTLP endpoint should not include '/v1/traces' suffix for OpenLIT"
			result.Warnings = append(result.Warnings, warning)
			v.logger.Warn("OTLP endpoint includes traces suffix",
				"component", "validation",
				"validation.field", "spec.otlp.endpoint",
				"validation.warning", warning,
				"endpoint", otlp.Endpoint)
		}
	}

	// Validate timeout range
	if otlp.Timeout != nil {
		if *otlp.Timeout < 1 || *otlp.Timeout > 300 {
			error := "timeout must be between 1 and 300 seconds"
			result.Errors = append(result.Errors, error)
			v.logger.Error("🚫 Invalid timeout value", fmt.Errorf("%s", error),
				"component", "validation",
				"validation.field", "spec.otlp.timeout",
				"timeout", *otlp.Timeout)
		}
	}
}

// validateResourceConfig validates resource configuration
func (v *AutoInstrumentationValidator) validateResourceConfig(res *v1alpha1.ResourceConfig, result *ValidationResult) {
	// Validate environment name format
	if res.Environment != "" {
		// Environment should be DNS-compatible
		if strings.Contains(res.Environment, " ") || strings.Contains(res.Environment, ".") {
			warning := "environment should be DNS-compatible (no spaces or dots)"
			result.Warnings = append(result.Warnings, warning)
			v.logger.Warn("Non-DNS-compatible environment",
				"component", "validation",
				"validation.field", "spec.resource.environment",
				"validation.warning", warning,
				"environment", res.Environment)
		}
	}
}

// GetValidationAttributes returns OpenTelemetry resource attributes for validation telemetry
func GetValidationAttributes() *resource.Resource {
	return resource.NewWithAttributes(
		semconv.SchemaURL,
		semconv.ServiceName("openlit-operator-validation"),
		semconv.ServiceVersion("1.0.0"),
		semconv.DeploymentEnvironment("kubernetes"),
	)
}
