package observability

import (
	"context"
	"errors"
	"os"
	"testing"
	"time"

	"github.com/go-logr/logr"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/suite"
	"go.opentelemetry.io/otel/log"
	"go.opentelemetry.io/otel/sdk/resource"
	semconv "go.opentelemetry.io/otel/semconv/v1.24.0"
)

type ObservabilityTestSuite struct {
	suite.Suite
	ctx context.Context
}

func (suite *ObservabilityTestSuite) SetupTest() {
	suite.ctx = context.Background()
}

func (suite *ObservabilityTestSuite) TestNewLoggerProvider() {
	tests := []struct {
		name                    string
		selfMonitoringEnabled   bool
		otlpEndpoint           string
		otlpLogsEndpoint       string
		serviceName            string
		serviceVersion         string
		namespace              string
		expectOTLPEnabled      bool
		expectError            bool
		description            string
	}{
		{
			name:                  "Self-monitoring disabled",
			selfMonitoringEnabled: false,
			otlpEndpoint:          "",
			otlpLogsEndpoint:      "",
			serviceName:           "test-operator",
			serviceVersion:        "1.0.0",
			namespace:             "test-ns",
			expectOTLPEnabled:     false,
			expectError:           false,
			description:           "Should create provider with stdout when monitoring disabled",
		},
		{
			name:                  "OTLP logs endpoint provided",
			selfMonitoringEnabled: true,
			otlpEndpoint:          "",
			otlpLogsEndpoint:      "http://localhost:4318/v1/logs",
			serviceName:           "test-operator",
			serviceVersion:        "1.0.0",
			namespace:             "test-ns",
			expectOTLPEnabled:     true, // Client creation succeeds (no connection test)
			expectError:           false,
			description:           "Should create OTLP client successfully",
		},
		{
			name:                  "General OTLP endpoint provided",
			selfMonitoringEnabled: true,
			otlpEndpoint:          "http://localhost:4318",
			otlpLogsEndpoint:      "",
			serviceName:           "test-operator",
			serviceVersion:        "1.0.0",
			namespace:             "test-ns",
			expectOTLPEnabled:     true, // Client creation succeeds (no connection test)
			expectError:           false,
			description:           "Should create OTLP client successfully",
		},
		{
			name:                  "Both endpoints provided - logs takes priority",
			selfMonitoringEnabled: true,
			otlpEndpoint:          "http://localhost:4318",
			otlpLogsEndpoint:      "http://localhost:4319/v1/logs",
			serviceName:           "test-operator",
			serviceVersion:        "1.0.0",
			namespace:             "test-ns",
			expectOTLPEnabled:     true, // Client creation succeeds (no connection test)
			expectError:           false,
			description:           "Should prioritize logs endpoint and create OTLP client",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			provider, err := NewLoggerProvider(
				suite.ctx,
				tt.selfMonitoringEnabled,
				tt.otlpEndpoint,
				tt.otlpLogsEndpoint,
				tt.serviceName,
				tt.serviceVersion,
				tt.namespace,
			)

			if tt.expectError {
				suite.Error(err, tt.description)
				suite.Nil(provider, tt.description)
			} else {
				suite.NoError(err, tt.description)
				suite.NotNil(provider, tt.description)
				suite.Equal(tt.expectOTLPEnabled, provider.OTLPEnabled, tt.description)

				// Verify provider can be shut down
				err = provider.Shutdown(suite.ctx)
				suite.NoError(err, "Should shutdown gracefully")
			}
		})
	}
}

func (suite *ObservabilityTestSuite) TestLoggerProviderResourceAttributes() {
	provider, err := NewLoggerProvider(
		suite.ctx,
		false, // Self-monitoring disabled for simple test
		"",
		"",
		"test-service",
		"1.2.3",
		"test-namespace",
	)

	suite.NoError(err)
	suite.NotNil(provider)

	// Provider should be created successfully with proper resource attributes
	// We can't easily inspect internal resource attributes, but we can verify creation
	suite.NotNil(provider.provider)

	// Cleanup
	err = provider.Shutdown(suite.ctx)
	suite.NoError(err)
}

func (suite *ObservabilityTestSuite) TestStructuredLogger() {
	// Create a simple provider for testing
	provider, err := NewLoggerProvider(
		suite.ctx,
		false,
		"",
		"",
		"test-service",
		"1.0.0",
		"test-ns",
	)
	suite.NoError(err)
	defer provider.Shutdown(suite.ctx)

	// Create structured logger
	otelLogger := provider.provider.Logger("test-logger")
	logger := NewStructuredLogger(otelLogger, suite.ctx)

	suite.NotNil(logger)
	suite.NotNil(logger.logger)
	suite.NotNil(logger.ctx)

	// Test all log levels (these should not error)
	logger.Debug("Debug message", "key1", "value1", "key2", 42)
	logger.Info("Info message", "component", "test", "success", true)
	logger.Warn("Warning message", "warning_code", 1001)
	logger.Error("Error message", errors.New("test error"), "error_context", "test")

	// Test context methods
	newLogger := logger.WithComponent("new-component")
	suite.NotNil(newLogger)
	
	ctxLogger := logger.WithContext(context.WithValue(suite.ctx, "test", "value"))
	suite.NotNil(ctxLogger)
}

func (suite *ObservabilityTestSuite) TestNewLogger() {
	// This uses the global logger provider
	logger := NewLogger("test-component")
	
	suite.NotNil(logger)
	suite.NotNil(logger.logger)
	suite.NotNil(logger.ctx)

	// Should be able to log without errors
	logger.Info("Test log message", "test", "value")
}

func (suite *ObservabilityTestSuite) TestLoggerAttributeTypes() {
	provider, err := NewLoggerProvider(
		suite.ctx,
		false,
		"",
		"",
		"test-service",
		"1.0.0",
		"test-ns",
	)
	suite.NoError(err)
	defer provider.Shutdown(suite.ctx)

	otelLogger := provider.provider.Logger("test-logger")
	logger := NewStructuredLogger(otelLogger, suite.ctx)

	// Test different attribute types
	testError := errors.New("test error")
	
	logger.Info("Testing attribute types",
		"string_attr", "string_value",
		"int_attr", 42,
		"int64_attr", int64(1234567890),
		"int32_attr", int32(999),
		"float64_attr", 3.14159,
		"bool_attr", true,
		"error_attr", testError,
		"interface_attr", struct{ Name string }{Name: "test"},
	)

	// Should complete without errors
}

func (suite *ObservabilityTestSuite) TestOpenTelemetryLogr() {
	// Test logr.Logger implementation
	logger := NewLogr("test-component")
	
	suite.NotNil(logger)

	// Test Info logging
	logger.Info("Test info message", "key1", "value1", "key2", 42)
	
	// Test Error logging
	testErr := errors.New("test error")
	logger.Error(testErr, "Test error message", "context", "test")

	// Test WithValues
	loggerWithValues := logger.WithValues("persistent_key", "persistent_value")
	suite.NotNil(loggerWithValues)
	loggerWithValues.Info("Message with persistent values", "temp_key", "temp_value")

	// Test WithName
	namedLogger := logger.WithName("sub-component")
	suite.NotNil(namedLogger)
	namedLogger.Info("Message from named logger")

	// Test nested names
	deeplyNamed := namedLogger.WithName("deep-component")
	suite.NotNil(deeplyNamed)
	deeplyNamed.Info("Message from deeply named logger")
}

func (suite *ObservabilityTestSuite) TestLogLevels() {
	// Test the LogLevel constants
	suite.Equal(LogLevel("debug"), LogLevelDebug)
	suite.Equal(LogLevel("info"), LogLevelInfo)
	suite.Equal(LogLevel("warn"), LogLevelWarn)
	suite.Equal(LogLevel("error"), LogLevelError)
}

func (suite *ObservabilityTestSuite) TestHelperFunctions() {
	// Test getServiceInstanceID
	originalHostname := os.Getenv("HOSTNAME")
	defer os.Setenv("HOSTNAME", originalHostname)

	os.Setenv("HOSTNAME", "test-pod-123")
	instanceID := getServiceInstanceID()
	suite.Equal("test-pod-123", instanceID)

	os.Unsetenv("HOSTNAME")
	instanceID = getServiceInstanceID()
	suite.Equal("unknown", instanceID)
}

func (suite *ObservabilityTestSuite) TestGetDeploymentEnvironment() {
	// Store original values
	originalDeployEnv := os.Getenv("DEPLOYMENT_ENVIRONMENT")
	originalEnv := os.Getenv("ENVIRONMENT")
	defer func() {
		if originalDeployEnv != "" {
			os.Setenv("DEPLOYMENT_ENVIRONMENT", originalDeployEnv)
		} else {
			os.Unsetenv("DEPLOYMENT_ENVIRONMENT")
		}
		if originalEnv != "" {
			os.Setenv("ENVIRONMENT", originalEnv)
		} else {
			os.Unsetenv("ENVIRONMENT")
		}
	}()

	// Test DEPLOYMENT_ENVIRONMENT takes priority
	os.Setenv("DEPLOYMENT_ENVIRONMENT", "staging")
	os.Setenv("ENVIRONMENT", "production")
	env := getDeploymentEnvironment()
	suite.Equal("staging", env)

	// Test ENVIRONMENT fallback
	os.Unsetenv("DEPLOYMENT_ENVIRONMENT")
	os.Setenv("ENVIRONMENT", "production")
	env = getDeploymentEnvironment()
	suite.Equal("production", env)

	// Test default fallback
	os.Unsetenv("DEPLOYMENT_ENVIRONMENT")
	os.Unsetenv("ENVIRONMENT")
	env = getDeploymentEnvironment()
	suite.Equal("kubernetes", env)
}

func (suite *ObservabilityTestSuite) TestGetPodName() {
	// getPodName uses os.Hostname() which we can't easily mock
	// but we can test that it returns a non-empty string
	podName := getPodName()
	suite.NotEmpty(podName)
}

func (suite *ObservabilityTestSuite) TestGetNamespaceFromServiceAccount() {
	// Test the function (will likely use fallback in test environment)
	namespace := getNamespaceFromServiceAccount()
	suite.NotEmpty(namespace)
	// In test environment, will likely be "openlit" (fallback)
	suite.Equal("openlit", namespace)
}

func (suite *ObservabilityTestSuite) TestOTLPConnectionHandling() {
	// Test OTLP client creation with invalid endpoint (no connection testing done)
	provider, err := NewLoggerProvider(
		suite.ctx,
		true,
		"http://invalid-endpoint:9999",
		"",
		"test-service",
		"1.0.0",
		"test-ns",
	)

	suite.NoError(err, "Should not error during client creation")
	suite.NotNil(provider)
	suite.True(provider.OTLPEnabled, "OTLP client creation succeeds (no connection test performed)")
	suite.Empty(provider.ErrorMessage, "No error message during client creation")

	// Cleanup
	err = provider.Shutdown(suite.ctx)
	suite.NoError(err)
}

func (suite *ObservabilityTestSuite) TestLoggerProviderShutdown() {
	provider, err := NewLoggerProvider(
		suite.ctx,
		false,
		"",
		"",
		"test-service",
		"1.0.0",
		"test-ns",
	)
	suite.NoError(err)
	suite.NotNil(provider)

	// Test shutdown
	err = provider.Shutdown(suite.ctx)
	suite.NoError(err)

	// Test shutdown with timeout context
	ctx, cancel := context.WithTimeout(suite.ctx, 5*time.Second)
	defer cancel()

	provider2, err := NewLoggerProvider(
		suite.ctx,
		false,
		"",
		"",
		"test-service",
		"1.0.0",
		"test-ns",
	)
	suite.NoError(err)

	err = provider2.Shutdown(ctx)
	suite.NoError(err)
}

func (suite *ObservabilityTestSuite) TestOpenTelemetryLogrEnabled() {
	logr := &OpenTelemetryLogr{
		logger: NewLogger("test"),
		name:   "test",
		level:  0,
	}

	// Test enabled at different levels
	suite.True(logr.Enabled(0))
	suite.True(logr.Enabled(1))
	suite.True(logr.Enabled(10))

	// Test with higher base level
	logr.level = 5
	suite.False(logr.Enabled(0))
	suite.False(logr.Enabled(4))
	suite.True(logr.Enabled(5))
	suite.True(logr.Enabled(10))
}

func (suite *ObservabilityTestSuite) TestOpenTelemetryLogrInit() {
	logr := &OpenTelemetryLogr{
		logger: NewLogger("test"),
		name:   "test",
		level:  0,
	}

	// Test passes if no panic occurs during setup
	suite.NotNil(logr, "Logger should be created successfully")
}

func (suite *ObservabilityTestSuite) TestConcurrentLogging() {
	provider, err := NewLoggerProvider(
		suite.ctx,
		false,
		"",
		"",
		"test-service",
		"1.0.0",
		"test-ns",
	)
	suite.NoError(err)
	defer provider.Shutdown(suite.ctx)

	otelLogger := provider.provider.Logger("concurrent-test")
	logger := NewStructuredLogger(otelLogger, suite.ctx)

	// Test concurrent logging
	done := make(chan bool, 10)
	
	for i := 0; i < 10; i++ {
		go func(id int) {
			defer func() { done <- true }()
			
			for j := 0; j < 10; j++ {
				logger.Info("Concurrent log message",
					"goroutine_id", id,
					"message_id", j,
					"test_data", "concurrent_test",
				)
			}
		}(i)
	}

	// Wait for all goroutines to complete
	for i := 0; i < 10; i++ {
		<-done
	}

	// Should complete without deadlocks or panics
}

func (suite *ObservabilityTestSuite) TestLoggerWithNilValues() {
	provider, err := NewLoggerProvider(
		suite.ctx,
		false,
		"",
		"",
		"test-service",
		"1.0.0",
		"test-ns",
	)
	suite.NoError(err)
	defer provider.Shutdown(suite.ctx)

	otelLogger := provider.provider.Logger("nil-test")
	logger := NewStructuredLogger(otelLogger, suite.ctx)

	// Test logging with odd number of attributes (should handle gracefully)
	logger.Info("Test with odd attributes", "key1", "value1", "key2")
	
	// Test logging with nil values
	logger.Info("Test with nil", "nil_key", nil)

	// Should complete without errors
}

func TestObservabilityTestSuite(t *testing.T) {
	suite.Run(t, new(ObservabilityTestSuite))
}

// Additional unit tests for specific functionality
func TestLogLevelConstants(t *testing.T) {
	assert.Equal(t, "debug", string(LogLevelDebug))
	assert.Equal(t, "info", string(LogLevelInfo))
	assert.Equal(t, "warn", string(LogLevelWarn))
	assert.Equal(t, "error", string(LogLevelError))
}

func TestResourceCreation(t *testing.T) {
	ctx := context.Background()
	
	// Test resource creation with semantic conventions
	res, err := resource.New(ctx,
		resource.WithAttributes(
			semconv.ServiceNameKey.String("test-service"),
			semconv.ServiceVersionKey.String("1.0.0"),
		),
	)
	
	assert.NoError(t, err)
	assert.NotNil(t, res)
	
	// Verify resource attributes
	attrs := res.Attributes()
	
	foundServiceName := false
	foundServiceVersion := false
	
	for _, kv := range attrs {
		switch kv.Key {
		case "service.name":
			assert.Equal(t, "test-service", kv.Value.AsString())
			foundServiceName = true
		case "service.version":
			assert.Equal(t, "1.0.0", kv.Value.AsString())
			foundServiceVersion = true
		}
	}
	
	assert.True(t, foundServiceName)
	assert.True(t, foundServiceVersion)
}

func TestLogSeverityMapping(t *testing.T) {
	// Test that log severities work correctly
	provider, err := NewLoggerProvider(
		context.Background(),
		false,
		"",
		"",
		"test-service",
		"1.0.0",
		"test-ns",
	)
	assert.NoError(t, err)
	defer provider.Shutdown(context.Background())

	otelLogger := provider.provider.Logger("severity-test")
	logger := NewStructuredLogger(otelLogger, context.Background())

	// Test severity constants exist and can be used
	logger.emit(log.SeverityDebug, "Debug test")
	logger.emit(log.SeverityInfo, "Info test")
	logger.emit(log.SeverityWarn, "Warn test")
	logger.emit(log.SeverityError, "Error test")
}

func TestLoggerProviderErrorScenarios(t *testing.T) {
	ctx := context.Background()
	
	// Test with empty service name (should still work)
	provider, err := NewLoggerProvider(
		ctx,
		false,
		"",
		"",
		"", // Empty service name
		"1.0.0",
		"test-ns",
	)
	assert.NoError(t, err)
	assert.NotNil(t, provider)
	provider.Shutdown(ctx)
	
	// Test with empty version (should still work)
	provider2, err := NewLoggerProvider(
		ctx,
		false,
		"",
		"",
		"test-service",
		"", // Empty version
		"test-ns",
	)
	assert.NoError(t, err)
	assert.NotNil(t, provider2)
	provider2.Shutdown(ctx)
}

func TestLogrIntegrationCompliance(t *testing.T) {
	// Test that our logr implementation satisfies the interface
	var logger logr.Logger = NewLogr("test")
	
	// These should all work without compilation errors
	logger.Info("test message")
	logger.Error(errors.New("test error"), "error occurred")
	
	withValues := logger.WithValues("key", "value")
	withValues.Info("message with values")
	
	withName := logger.WithName("sub-component")
	withName.Info("message from sub-component")
	
	// Test chaining
	chained := logger.WithName("component").WithValues("id", 123)
	chained.Info("chained logger message")
}
