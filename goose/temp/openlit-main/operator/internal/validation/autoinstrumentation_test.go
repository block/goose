package validation

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/suite"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"

	autoinstrumentationv1alpha1 "github.com/openlit/openlit/operator/api/v1alpha1"
)

type AutoInstrumentationValidatorTestSuite struct {
	suite.Suite
	validator *AutoInstrumentationValidator
}

func (suite *AutoInstrumentationValidatorTestSuite) SetupTest() {
	// Create validator using the proper constructor
	suite.validator = NewAutoInstrumentationValidator()
}

func (suite *AutoInstrumentationValidatorTestSuite) TestValidateValidConfiguration() {
	autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "valid-instrumentation",
			Namespace: "default",
		},
		Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
			Selector: autoinstrumentationv1alpha1.PodSelector{
				MatchLabels: map[string]string{
					"app": "python-app",
				},
			},
			Python: &autoinstrumentationv1alpha1.PythonInstrumentation{
				Instrumentation: &autoinstrumentationv1alpha1.InstrumentationSettings{
					Provider:        "openlit",
					CustomInitImage: "openlit-instrumentation:latest",
				},
			},
			OTLP: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "http://openlit.default.svc.cluster.local:4318",
			},
			Resource: &autoinstrumentationv1alpha1.ResourceConfig{
				Environment: "test",
			},
		},
	}

	result := suite.validator.Validate(autoInstr)

	suite.True(result.Valid, "Valid configuration should pass validation")
	suite.Empty(result.Errors, "Valid configuration should have no errors")
}

func (suite *AutoInstrumentationValidatorTestSuite) TestValidateRequiredFields() {
	tests := []struct {
		name             string
		autoInstr        *autoinstrumentationv1alpha1.AutoInstrumentation
		expectedErrors   []string
		expectedWarnings []string
		description      string
	}{
		{
			name: "Missing selector",
			autoInstr: &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "missing-selector",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						// Empty selector
					},
					Python: &autoinstrumentationv1alpha1.PythonInstrumentation{
						Instrumentation: &autoinstrumentationv1alpha1.InstrumentationSettings{
							Provider:        "openlit",
							CustomInitImage: "openlit-instrumentation:latest",
						},
					},
					OTLP: autoinstrumentationv1alpha1.OTLPConfig{
						Endpoint: "http://test:4318",
					},
				},
			},
			expectedErrors: []string{
				"spec.selector must have either matchLabels or matchExpressions",
			},
			description: "Should fail when selector is completely empty",
		},
		{
			name: "Valid selector with matchLabels",
			autoInstr: &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "valid-matchlabels",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"app": "python-app",
						},
					},
					Python: &autoinstrumentationv1alpha1.PythonInstrumentation{
						Instrumentation: &autoinstrumentationv1alpha1.InstrumentationSettings{
							Provider:        "openlit",
							CustomInitImage: "openlit-instrumentation:latest",
						},
					},
					OTLP: autoinstrumentationv1alpha1.OTLPConfig{
						Endpoint: "http://test:4318",
					},
				},
			},
			expectedErrors: []string{},
			description:    "Should pass with valid matchLabels",
		},
		{
			name: "Valid selector with matchExpressions",
			autoInstr: &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "valid-expressions",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchExpressions: []metav1.LabelSelectorRequirement{
							{
								Key:      "app",
								Operator: metav1.LabelSelectorOpIn,
								Values:   []string{"python-app", "web-app"},
							},
						},
					},
					Python: &autoinstrumentationv1alpha1.PythonInstrumentation{
						Instrumentation: &autoinstrumentationv1alpha1.InstrumentationSettings{
							Provider:        "openlit",
							CustomInitImage: "openlit-instrumentation:latest",
						},
					},
					OTLP: autoinstrumentationv1alpha1.OTLPConfig{
						Endpoint: "http://test:4318",
					},
				},
			},
			expectedErrors: []string{},
			description:    "Should pass with valid matchExpressions",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			result := suite.validator.Validate(tt.autoInstr)

			for _, expectedError := range tt.expectedErrors {
				found := false
				for _, actualError := range result.Errors {
					if actualError == expectedError {
						found = true
						break
					}
				}
				suite.True(found, "Expected error not found: %s", expectedError)
			}

			if len(tt.expectedErrors) > 0 {
				suite.False(result.Valid, tt.description)
			} else {
				suite.True(result.Valid, tt.description)
			}
		})
	}
}

func (suite *AutoInstrumentationValidatorTestSuite) TestValidateSelector() {
	tests := []struct {
		name           string
		selector       autoinstrumentationv1alpha1.PodSelector
		expectedErrors []string
		description    string
	}{
		{
			name: "Empty key in matchLabels",
			selector: autoinstrumentationv1alpha1.PodSelector{
				MatchLabels: map[string]string{
					"":    "value",
					"app": "python-app",
				},
			},
			expectedErrors: []string{
				"matchLabels keys cannot be empty",
			},
			description: "Should fail with empty key in matchLabels",
		},
		{
			name: "Empty key in matchExpressions",
			selector: autoinstrumentationv1alpha1.PodSelector{
				MatchExpressions: []metav1.LabelSelectorRequirement{
					{
						Key:      "",
						Operator: metav1.LabelSelectorOpIn,
						Values:   []string{"value"},
					},
				},
			},
			expectedErrors: []string{
				"matchExpressions[0].key cannot be empty",
			},
			description: "Should fail with empty key in matchExpressions",
		},
		{
			name: "Valid selector",
			selector: autoinstrumentationv1alpha1.PodSelector{
				MatchLabels: map[string]string{
					"app":     "python-app",
					"version": "v1.0.0",
				},
				MatchExpressions: []metav1.LabelSelectorRequirement{
					{
						Key:      "environment",
						Operator: metav1.LabelSelectorOpNotIn,
						Values:   []string{"staging"},
					},
				},
			},
			expectedErrors: []string{},
			description:    "Should pass with valid selector",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			result := &ValidationResult{
				Valid:    true,
				Errors:   []string{},
				Warnings: []string{},
			}

			suite.validator.validateSelector(tt.selector, result)

			for _, expectedError := range tt.expectedErrors {
				found := false
				for _, actualError := range result.Errors {
					if actualError == expectedError {
						found = true
						break
					}
				}
				suite.True(found, "Expected error not found: %s", expectedError)
			}

			if len(tt.expectedErrors) > 0 {
				suite.NotEmpty(result.Errors, tt.description)
			} else {
				suite.Empty(result.Errors, tt.description)
			}
		})
	}
}

func (suite *AutoInstrumentationValidatorTestSuite) TestValidateIgnoreSelector() {
	tests := []struct {
		name             string
		ignore           autoinstrumentationv1alpha1.PodSelector
		expectedWarnings []string
		description      string
	}{
		{
			name: "Empty ignore selector",
			ignore: autoinstrumentationv1alpha1.PodSelector{
				// Completely empty
			},
			expectedWarnings: []string{
				"ignore selector with no criteria will ignore all pods",
			},
			description: "Should warn about overly broad ignore selector",
		},
		{
			name: "Specific ignore selector",
			ignore: autoinstrumentationv1alpha1.PodSelector{
				MatchLabels: map[string]string{
					"skip": "true",
				},
			},
			expectedWarnings: []string{},
			description:      "Should not warn about specific ignore selector",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			result := &ValidationResult{
				Valid:    true,
				Errors:   []string{},
				Warnings: []string{},
			}

			suite.validator.validateIgnoreSelector(tt.ignore, result)

			for _, expectedWarning := range tt.expectedWarnings {
				found := false
				for _, actualWarning := range result.Warnings {
					if actualWarning == expectedWarning {
						found = true
						break
					}
				}
				suite.True(found, "Expected warning not found: %s", expectedWarning)
			}

			suite.Equal(len(tt.expectedWarnings), len(result.Warnings), tt.description)
		})
	}
}

func (suite *AutoInstrumentationValidatorTestSuite) TestValidateEnvironmentVariables() {
	tests := []struct {
		name             string
		envVars          []autoinstrumentationv1alpha1.EnvVar
		expectedErrors   []string
		expectedWarnings []string
		description      string
	}{
		{
			name: "Duplicate environment variables",
			envVars: []autoinstrumentationv1alpha1.EnvVar{
				{Name: "APP_ENV", Value: "production"},
				{Name: "APP_ENV", Value: "staging"},
			},
			expectedErrors: []string{
				"duplicate environment variable name 'APP_ENV' at position 1",
			},
			description: "Should fail with duplicate environment variable names",
		},
		{
			name: "Missing value and valueFrom",
			envVars: []autoinstrumentationv1alpha1.EnvVar{
				{Name: "EMPTY_VAR"},
			},
			expectedErrors: []string{
				"environment variable 'EMPTY_VAR' must have either 'value' or 'valueFrom' set",
			},
			description: "Should fail when neither value nor valueFrom is set",
		},
		{
			name: "Both value and valueFrom set",
			envVars: []autoinstrumentationv1alpha1.EnvVar{
				{
					Name:  "CONFLICT_VAR",
					Value: "direct-value",
					ValueFrom: &autoinstrumentationv1alpha1.EnvVarSource{
						SecretKeyRef: &autoinstrumentationv1alpha1.SecretKeySelector{
							Name: "secret",
							Key:  "key",
						},
					},
				},
			},
			expectedErrors: []string{
				"environment variable 'CONFLICT_VAR' cannot have both 'value' and 'valueFrom' set",
			},
			description: "Should fail when both value and valueFrom are set",
		},
		{
			name: "Reserved environment variable",
			envVars: []autoinstrumentationv1alpha1.EnvVar{
				{Name: "OTEL_EXPORTER_OTLP_ENDPOINT", Value: "http://custom-endpoint"},
				{Name: "PYTHONPATH", Value: "/custom/path"},
			},
			expectedWarnings: []string{
				"environment variable 'OTEL_EXPORTER_OTLP_ENDPOINT' is reserved and may be overridden by the operator",
				"environment variable 'PYTHONPATH' is reserved and may be overridden by the operator",
			},
			description: "Should warn about reserved environment variables",
		},
		{
			name: "Valid environment variables",
			envVars: []autoinstrumentationv1alpha1.EnvVar{
				{Name: "APP_NAME", Value: "my-app"},
				{Name: "LOG_LEVEL", Value: "debug"},
				{
					Name: "DB_PASSWORD",
					ValueFrom: &autoinstrumentationv1alpha1.EnvVarSource{
						SecretKeyRef: &autoinstrumentationv1alpha1.SecretKeySelector{
							Name: "db-secret",
							Key:  "password",
						},
					},
				},
			},
			expectedErrors:   []string{},
			expectedWarnings: []string{},
			description:      "Should pass with valid environment variables",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			result := &ValidationResult{
				Valid:    true,
				Errors:   []string{},
				Warnings: []string{},
			}

			suite.validator.validateEnvironmentVariables(tt.envVars, result)

			// Check errors
			for _, expectedError := range tt.expectedErrors {
				found := false
				for _, actualError := range result.Errors {
					if actualError == expectedError {
						found = true
						break
					}
				}
				suite.True(found, "Expected error not found: %s", expectedError)
			}

			// Check warnings
			for _, expectedWarning := range tt.expectedWarnings {
				found := false
				for _, actualWarning := range result.Warnings {
					if actualWarning == expectedWarning {
						found = true
						break
					}
				}
				suite.True(found, "Expected warning not found: %s", expectedWarning)
			}

			suite.Equal(len(tt.expectedErrors), len(result.Errors), tt.description)
			suite.Equal(len(tt.expectedWarnings), len(result.Warnings), tt.description)
		})
	}
}

func (suite *AutoInstrumentationValidatorTestSuite) TestValidateOTLPConfig() {
	tests := []struct {
		name             string
		otlp             autoinstrumentationv1alpha1.OTLPConfig
		expectedErrors   []string
		expectedWarnings []string
		description      string
	}{
		{
			name: "Invalid endpoint URL",
			otlp: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "http://invalid url with spaces",
			},
			expectedErrors: []string{
				"invalid OTLP endpoint URL:",
			},
			description: "Should fail with invalid URL",
		},
		{
			name: "Endpoint with traces suffix",
			otlp: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "http://jaeger:14268/v1/traces",
			},
			expectedWarnings: []string{
				"OTLP endpoint should not include '/v1/traces' suffix for OpenLIT",
			},
			description: "Should warn about traces suffix",
		},
		{
			name: "Invalid timeout - too low",
			otlp: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "http://openlit:4318",
				Timeout:  &[]int32{0}[0],
			},
			expectedErrors: []string{
				"timeout must be between 1 and 300 seconds",
			},
			description: "Should fail with timeout too low",
		},
		{
			name: "Invalid timeout - too high",
			otlp: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "http://openlit:4318",
				Timeout:  &[]int32{500}[0],
			},
			expectedErrors: []string{
				"timeout must be between 1 and 300 seconds",
			},
			description: "Should fail with timeout too high",
		},
		{
			name: "Valid OTLP configuration",
			otlp: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "http://openlit.default.svc.cluster.local:4318",
				Headers:  "Authorization=Bearer token123",
				Timeout:  &[]int32{30}[0],
			},
			expectedErrors:   []string{},
			expectedWarnings: []string{},
			description:      "Should pass with valid OTLP configuration",
		},
		{
			name: "HTTPS endpoint",
			otlp: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "https://otlp.example.com/v1/traces",
			},
			expectedWarnings: []string{
				"OTLP endpoint should not include '/v1/traces' suffix for OpenLIT",
			},
			description: "Should warn about HTTPS endpoint with traces suffix",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			result := &ValidationResult{
				Valid:    true,
				Errors:   []string{},
				Warnings: []string{},
			}

			suite.validator.validateOTLPConfig(tt.otlp, result)

			// Check errors (allowing for slight differences in error messages)
			suite.Equal(len(tt.expectedErrors), len(result.Errors), "Error count mismatch for %s", tt.description)
			if len(tt.expectedErrors) > 0 {
				suite.NotEmpty(result.Errors, tt.description)
				// Check if the actual error contains the expected substring
				for i, expectedError := range tt.expectedErrors {
					if i < len(result.Errors) {
						suite.Contains(result.Errors[i], expectedError, "Error message should contain expected substring")
					}
				}
			}

			// Check warnings
			for _, expectedWarning := range tt.expectedWarnings {
				found := false
				for _, actualWarning := range result.Warnings {
					if actualWarning == expectedWarning {
						found = true
						break
					}
				}
				suite.True(found, "Expected warning not found: %s", expectedWarning)
			}

			suite.Equal(len(tt.expectedWarnings), len(result.Warnings), tt.description)
		})
	}
}

func (suite *AutoInstrumentationValidatorTestSuite) TestValidateResourceConfig() {
	tests := []struct {
		name             string
		resource         *autoinstrumentationv1alpha1.ResourceConfig
		expectedWarnings []string
		description      string
	}{
		{
			name: "Environment with spaces",
			resource: &autoinstrumentationv1alpha1.ResourceConfig{
				Environment: "my environment",
			},
			expectedWarnings: []string{
				"environment should be DNS-compatible (no spaces or dots)",
			},
			description: "Should warn about spaces in environment",
		},
		{
			name: "Environment with dots",
			resource: &autoinstrumentationv1alpha1.ResourceConfig{
				Environment: "my.environment",
			},
			expectedWarnings: []string{
				"environment should be DNS-compatible (no spaces or dots)",
			},
			description: "Should warn about dots in environment",
		},
		{
			name: "Valid DNS-compatible environment",
			resource: &autoinstrumentationv1alpha1.ResourceConfig{
				Environment: "production",
			},
			expectedWarnings: []string{},
			description:      "Should pass with valid environment",
		},
		{
			name: "Hyphenated environment",
			resource: &autoinstrumentationv1alpha1.ResourceConfig{
				Environment: "prod-us-west",
			},
			expectedWarnings: []string{},
			description:      "Should pass with hyphenated environment",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			result := &ValidationResult{
				Valid:    true,
				Errors:   []string{},
				Warnings: []string{},
			}

			suite.validator.validateResourceConfig(tt.resource, result)

			for _, expectedWarning := range tt.expectedWarnings {
				found := false
				for _, actualWarning := range result.Warnings {
					if actualWarning == expectedWarning {
						found = true
						break
					}
				}
				suite.True(found, "Expected warning not found: %s", expectedWarning)
			}

			suite.Equal(len(tt.expectedWarnings), len(result.Warnings), tt.description)
		})
	}
}

func (suite *AutoInstrumentationValidatorTestSuite) TestComplexValidationScenarios() {
	tests := []struct {
		name           string
		autoInstr      *autoinstrumentationv1alpha1.AutoInstrumentation
		expectValid    bool
		expectErrors   int
		expectWarnings int
		description    string
	}{
		{
			name: "Multiple validation issues",
			autoInstr: &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "problematic-config",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						// Empty selector - error
					},
					Python: &autoinstrumentationv1alpha1.PythonInstrumentation{
						Instrumentation: &autoinstrumentationv1alpha1.InstrumentationSettings{
							Provider:        "openlit",
							CustomInitImage: "openlit-instrumentation:latest",
						},
					},
					OTLP: autoinstrumentationv1alpha1.OTLPConfig{
						Endpoint: "http://test:4318",
					},
					Ignore: &autoinstrumentationv1alpha1.PodSelector{
						// Empty ignore - warning
					},
				},
			},
			expectValid:    false,
			expectErrors:   1, // Missing selector
			expectWarnings: 1, // Broad ignore selector
			description:    "Should handle multiple validation issues",
		},
		{
			name: "Complex valid configuration",
			autoInstr: &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "complex-valid",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"app": "python-app",
						},
						MatchExpressions: []metav1.LabelSelectorRequirement{
							{
								Key:      "environment",
								Operator: metav1.LabelSelectorOpIn,
								Values:   []string{"production", "staging"},
							},
						},
					},
					Python: &autoinstrumentationv1alpha1.PythonInstrumentation{
						Instrumentation: &autoinstrumentationv1alpha1.InstrumentationSettings{
							Provider:        "openlit",
							CustomInitImage: "openlit-instrumentation:latest",
						},
					},
					OTLP: autoinstrumentationv1alpha1.OTLPConfig{
						Endpoint: "https://otlp.example.com",
					},
					Resource: &autoinstrumentationv1alpha1.ResourceConfig{
						Environment: "production",
					},
					Ignore: &autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"skip-instrumentation": "true",
						},
					},
				},
			},
			expectValid:    true,
			expectErrors:   0,
			expectWarnings: 0,
			description:    "Should pass complex valid configuration",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			result := suite.validator.Validate(tt.autoInstr)

			suite.Equal(tt.expectValid, result.Valid, tt.description)
			suite.Equal(tt.expectErrors, len(result.Errors), "Error count mismatch for %s", tt.description)
			suite.Equal(tt.expectWarnings, len(result.Warnings), "Warning count mismatch for %s", tt.description)
		})
	}
}

func TestAutoInstrumentationValidatorSuite(t *testing.T) {
	suite.Run(t, new(AutoInstrumentationValidatorTestSuite))
}

// Additional unit tests for specific functions
func TestNewAutoInstrumentationValidator(t *testing.T) {
	validator := NewAutoInstrumentationValidator()
	assert.NotNil(t, validator)
	assert.NotNil(t, validator.logger)
}

func TestGetValidationAttributes(t *testing.T) {
	resource := GetValidationAttributes()
	assert.NotNil(t, resource)
	
	attrs := resource.Attributes()
	
	foundServiceName := false
	foundServiceVersion := false
	
	for _, kv := range attrs {
		switch kv.Key {
		case "service.name":
			assert.Equal(t, "openlit-operator-validation", kv.Value.AsString())
			foundServiceName = true
		case "service.version":
			assert.Equal(t, "1.0.0", kv.Value.AsString())
			foundServiceVersion = true
		}
	}
	
	assert.True(t, foundServiceName, "Should contain service.name attribute")
	assert.True(t, foundServiceVersion, "Should contain service.version attribute")
}

func TestValidationResultInitialization(t *testing.T) {
	result := &ValidationResult{
		Valid:    true,
		Errors:   []string{},
		Warnings: []string{},
	}
	
	assert.True(t, result.Valid)
	assert.Empty(t, result.Errors)
	assert.Empty(t, result.Warnings)
}

func TestValidationErrorHandling(t *testing.T) {
	// Test that validator handles nil configurations gracefully
	// Create validator using the proper constructor
	validator := NewAutoInstrumentationValidator()

	// This should not panic even with a minimal configuration
	autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "minimal",
			Namespace: "default",
		},
		Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{},
	}

	result := validator.Validate(autoInstr)
	assert.NotNil(t, result)
	assert.False(t, result.Valid) // Should be invalid due to missing required fields
	assert.NotEmpty(t, result.Errors) // Should have validation errors
}
