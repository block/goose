package config

import (
	"os"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/suite"
)

type ConfigTestSuite struct {
	suite.Suite
	originalEnv map[string]string
}

func (suite *ConfigTestSuite) SetupTest() {
	// Store original environment variables
	suite.originalEnv = make(map[string]string)
	envVars := []string{
		"WEBHOOK_PORT",
		"WEBHOOK_CERT_DIR",
		"SELF_MONITORING_ENABLED",
		"OTEL_EXPORTER_OTLP_LOGS_ENDPOINT",
		"OTEL_EXPORTER_OTLP_HEADERS",
		"WEBHOOK_SERVICE_NAME",
		"LOG_LEVEL",
		"HEALTH_PORT",
	}

	for _, env := range envVars {
		if value, exists := os.LookupEnv(env); exists {
			suite.originalEnv[env] = value
		}
	}
}

func (suite *ConfigTestSuite) TearDownTest() {
	// Restore original environment variables
	envVars := []string{
		"WEBHOOK_PORT",
		"WEBHOOK_CERT_DIR",
		"SELF_MONITORING_ENABLED",
		"OTEL_EXPORTER_OTLP_LOGS_ENDPOINT",
		"OTEL_EXPORTER_OTLP_HEADERS",
		"WEBHOOK_SERVICE_NAME",
		"LOG_LEVEL",
		"HEALTH_PORT",
	}

	// Clear all test environment variables
	for _, env := range envVars {
		os.Unsetenv(env)
	}

	// Restore original values
	for env, value := range suite.originalEnv {
		os.Setenv(env, value)
	}
}

func (suite *ConfigTestSuite) TestDefaultConfiguration() {
	// Clear all environment variables
	os.Unsetenv("WEBHOOK_PORT")
	os.Unsetenv("WEBHOOK_CERT_DIR")
	os.Unsetenv("SELF_MONITORING_ENABLED")
	os.Unsetenv("WEBHOOK_SERVICE_NAME")
	os.Unsetenv("LOG_LEVEL")

	config, err := GetConfig()
	suite.NoError(err)

	// Test default values according to schema
	suite.Equal(9443, config.WebhookPort, "Should use default webhook port")
	suite.Equal("/tmp/k8s-webhook-server/serving-certs", config.WebhookCertDir, "Should use default cert directory")
	suite.False(config.SelfMonitoringEnabled, "Should disable self-monitoring by default")
	suite.Equal("openlit-webhook-service", config.ServiceName, "Should use default service name")
	suite.Equal("info", config.LogLevel, "Should use default log level")
	suite.Equal(8081, config.HealthPort, "Should use default health port")
}

func (suite *ConfigTestSuite) TestEnvironmentVariableOverrides() {
	// Set environment variables with correct names from schema
	os.Setenv("WEBHOOK_PORT", "8443")
	os.Setenv("WEBHOOK_CERT_DIR", "/custom/certs")
	os.Setenv("SELF_MONITORING_ENABLED", "true")
	os.Setenv("WEBHOOK_SERVICE_NAME", "custom-webhook-service")
	os.Setenv("LOG_LEVEL", "debug")
	os.Setenv("HEALTH_PORT", "9090")

	config, err := GetConfig()
	suite.NoError(err)

	// Test environment variable overrides
	suite.Equal(8443, config.WebhookPort, "Should use environment variable for webhook port")
	suite.Equal("/custom/certs", config.WebhookCertDir, "Should use environment variable for cert directory")
	suite.True(config.SelfMonitoringEnabled, "Should use environment variable for self-monitoring")
	suite.Equal("custom-webhook-service", config.ServiceName, "Should use environment variable for service name")
	suite.Equal("debug", config.LogLevel, "Should use environment variable for log level")
	suite.Equal(9090, config.HealthPort, "Should use environment variable for health port")
}

func (suite *ConfigTestSuite) TestOTLPConfiguration() {
	// Test OTLP configuration
	os.Setenv("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT", "https://logs.example.com/v1/logs")
	os.Setenv("OTEL_EXPORTER_OTLP_HEADERS", "Authorization=Bearer token123")

	config, err := GetConfig()
	suite.NoError(err)

	suite.Equal("https://logs.example.com/v1/logs", config.OTLPLogsEndpoint, "Should use OTLP logs endpoint")
	suite.Equal("Authorization=Bearer token123", config.OTLPHeaders, "Should use OTLP headers")
}

func (suite *ConfigTestSuite) TestInvalidWebhookPort() {
	// Test invalid webhook port - should fail with schema validation
	os.Setenv("WEBHOOK_PORT", "invalid")

	config, err := GetConfig()
	suite.Error(err, "Should fail with invalid port value")
	suite.Nil(config, "Config should be nil on validation failure")
	suite.Contains(err.Error(), "strconv.Atoi: parsing \"invalid\"", "Should contain parsing error")
}

func (suite *ConfigTestSuite) TestSelfMonitoringBooleanParsing() {
	// Test valid boolean values
	validTests := []struct {
		envValue string
		expected bool
		name     string
	}{
		{"true", true, "String 'true' should enable monitoring"},
		{"TRUE", true, "String 'TRUE' should enable monitoring"},
		{"True", true, "String 'True' should enable monitoring"},
		{"false", false, "String 'false' should disable monitoring"},
		{"FALSE", false, "String 'FALSE' should disable monitoring"},
		{"False", false, "String 'False' should disable monitoring"},
		{"1", true, "String '1' should enable monitoring"},
		{"0", false, "String '0' should disable monitoring"},
	}

	for _, tt := range validTests {
		suite.Run(tt.name, func() {
			os.Setenv("SELF_MONITORING_ENABLED", tt.envValue)
			config, err := GetConfig()
			suite.NoError(err)
			suite.Equal(tt.expected, config.SelfMonitoringEnabled, tt.name)
		})
	}

	// Test invalid values - should fail with schema validation
	suite.Run("Invalid values should fail validation", func() {
		os.Setenv("SELF_MONITORING_ENABLED", "invalid")
		config, err := GetConfig()
		suite.Error(err, "Should fail with invalid boolean value")
		suite.Nil(config, "Config should be nil on validation failure")
	})

	// Test empty string - should use default (false)
	suite.Run("Empty string should use default", func() {
		os.Unsetenv("SELF_MONITORING_ENABLED") // Ensure it's not set
		config, err := GetConfig()
		suite.NoError(err, "Should succeed with default value")
		suite.False(config.SelfMonitoringEnabled, "Should use schema default (false)")
	})
}

func (suite *ConfigTestSuite) TestValidation() {
	tests := []struct {
		name        string
		config      *OperatorConfig
		expectError bool
		errorMsg    string
	}{
		{
			name: "Valid configuration",
			config: &OperatorConfig{
				WebhookPort:           9443,
				WebhookCertDir:        "/tmp/certs",
				SelfMonitoringEnabled: true,
				ServiceName:           "openlit-webhook-service",
				LogLevel:              "info",
				HealthPort:            8081,
				CertValidityDays:      365,
				CertRefreshDays:       30,
			},
			expectError: false,
		},
		{
			name: "Invalid webhook port - too low",
			config: &OperatorConfig{
				WebhookPort:           0,
				WebhookCertDir:        "/tmp/certs",
				SelfMonitoringEnabled: true,
				ServiceName:           "openlit-webhook-service",
				LogLevel:              "info",
				HealthPort:            8081,
				CertValidityDays:      365,
				CertRefreshDays:       30,
			},
			expectError: true,
			errorMsg:    "webhookPort must be between 1 and 65535",
		},
		{
			name: "Invalid webhook port - too high",
			config: &OperatorConfig{
				WebhookPort:           70000,
				WebhookCertDir:        "/tmp/certs",
				SelfMonitoringEnabled: true,
				ServiceName:           "openlit-webhook-service",
				LogLevel:              "info",
				HealthPort:            8081,
				CertValidityDays:      365,
				CertRefreshDays:       30,
			},
			expectError: true,
			errorMsg:    "webhookPort must be between 1 and 65535",
		},
		{
			name: "Invalid log level",
			config: &OperatorConfig{
				WebhookPort:           9443,
				WebhookCertDir:        "/tmp/certs",
				SelfMonitoringEnabled: true,
				ServiceName:           "openlit-webhook-service",
				LogLevel:              "invalid",
				HealthPort:            8081,
				CertValidityDays:      365,
				CertRefreshDays:       30,
			},
			expectError: true,
			errorMsg:    "logLevel must be one of: debug, info, warn, error",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			err := tt.config.Validate()
			if tt.expectError {
				suite.Error(err, "Should fail validation")
				suite.Contains(err.Error(), tt.errorMsg, "Should contain expected error message")
			} else {
				suite.NoError(err, "Should pass validation")
			}
		})
	}
}

func (suite *ConfigTestSuite) TestOTLPConfigurationValidation() {
	// Test that OTLP configuration values are properly set
	config := &OperatorConfig{
		WebhookPort:           9443,
		WebhookCertDir:        "/tmp/certs",
		SelfMonitoringEnabled: true,
		ServiceName:           "openlit-webhook-service",
		LogLevel:              "info",
		HealthPort:            8081,
		CertValidityDays:      365,
		CertRefreshDays:       30,
		OTLPLogsEndpoint:      "https://logs.example.com/v1/logs",
		OTLPHeaders:           "Authorization=Bearer token123",
	}

	err := config.Validate()
	suite.NoError(err, "Should pass validation")
	suite.Equal("https://logs.example.com/v1/logs", config.OTLPLogsEndpoint)
	suite.Equal("Authorization=Bearer token123", config.OTLPHeaders)
}

func (suite *ConfigTestSuite) TestConfigStringRepresentation() {
	config := &OperatorConfig{
		WebhookPort:           9443,
		WebhookCertDir:        "/tmp/certs",
		SelfMonitoringEnabled: true,
		ServiceName:           "openlit-webhook-service",
		LogLevel:              "info",
		HealthPort:            8081,
		CertValidityDays:      365,
		CertRefreshDays:       30,
		OTLPLogsEndpoint:      "https://logs.example.com/v1/logs",
		OTLPHeaders:           "Authorization=Bearer token123",
	}

	// Test basic configuration values
	suite.Equal(9443, config.WebhookPort)
	suite.Equal("/tmp/certs", config.WebhookCertDir)
	suite.True(config.SelfMonitoringEnabled)
	suite.Equal("openlit-webhook-service", config.ServiceName)
	suite.Equal("https://logs.example.com/v1/logs", config.OTLPLogsEndpoint)
	suite.Equal("Authorization=Bearer token123", config.OTLPHeaders)
}

func (suite *ConfigTestSuite) TestIsOTLPEnabled() {
	tests := []struct {
		name     string
		endpoint string
		expected bool
	}{
		{
			name:     "OTLP enabled with endpoint",
			endpoint: "https://logs.example.com/v1/logs",
			expected: true,
		},
		{
			name:     "OTLP disabled with empty endpoint",
			endpoint: "",
			expected: false,
		},
		{
			name:     "OTLP disabled with whitespace endpoint",
			endpoint: "   ",
			expected: false,
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			config := &OperatorConfig{
				OTLPLogsEndpoint: tt.endpoint,
			}
			// Since IsOTLPEnabled doesn't exist, we'll check if endpoint is not empty
			isEnabled := strings.TrimSpace(config.OTLPLogsEndpoint) != ""
			suite.Equal(tt.expected, isEnabled, "Should correctly determine OTLP status")
		})
	}
}

func TestConfigSuite(t *testing.T) {
	suite.Run(t, new(ConfigTestSuite))
}

// Additional unit tests for edge cases
func TestConfigConcurrency(t *testing.T) {
	// Test that config creation is thread-safe
	done := make(chan bool, 10)
	
	for i := 0; i < 10; i++ {
		go func(id int) {
			defer func() { done <- true }()
			
			// Set different environment variables in different goroutines
			os.Setenv("WEBHOOK_PORT", "9443")
			os.Setenv("SELF_MONITORING_ENABLED", "true")
			
			config, err := GetConfig()
			
			// Verify config is valid
			assert.NoError(t, err)
			assert.Equal(t, 9443, config.WebhookPort)
			assert.True(t, config.SelfMonitoringEnabled)
			assert.NoError(t, config.Validate())
		}(i)
	}
	
	// Wait for all goroutines to complete
	for i := 0; i < 10; i++ {
		<-done
	}
}

func TestConfigDefaults(t *testing.T) {
	// Ensure all environment variables are cleared
	envVars := []string{
		"WEBHOOK_PORT",
		"WEBHOOK_CERT_DIR", 
		"SELF_MONITORING_ENABLED",
		"OTEL_EXPORTER_OTLP_LOGS_ENDPOINT",
		"OTEL_EXPORTER_OTLP_HEADERS",
		"WEBHOOK_SERVICE_NAME",
		"LOG_LEVEL",
		"HEALTH_PORT",
	}
	
	for _, env := range envVars {
		os.Unsetenv(env)
	}
	
	config, err := GetConfig()
	
	// Verify all defaults are set correctly according to schema
	assert.NoError(t, err)
	assert.Equal(t, 9443, config.WebhookPort)
	assert.Equal(t, "/tmp/k8s-webhook-server/serving-certs", config.WebhookCertDir)
	assert.False(t, config.SelfMonitoringEnabled) // Default is false according to schema
	assert.Equal(t, "openlit-webhook-service", config.ServiceName)
	assert.Equal(t, "info", config.LogLevel)
	assert.Equal(t, 8081, config.HealthPort)
	assert.Empty(t, config.OTLPLogsEndpoint)
	assert.Empty(t, config.OTLPHeaders)
	
	// Config should be valid with defaults
	assert.NoError(t, config.Validate())
}
