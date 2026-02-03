package observability

import (
	"context"
	"fmt"
	"io/ioutil"
	"os"
	"strings"

	"github.com/go-logr/logr"
	"go.opentelemetry.io/otel/exporters/otlp/otlplog/otlploghttp"
	"go.opentelemetry.io/otel/exporters/stdout/stdoutlog"
	"go.opentelemetry.io/otel/log"
	"go.opentelemetry.io/otel/log/global"
	sdklog "go.opentelemetry.io/otel/sdk/log"
	"go.opentelemetry.io/otel/sdk/resource"
	semconv "go.opentelemetry.io/otel/semconv/v1.24.0"
)

// LogLevel represents the available log levels
type LogLevel string

const (
	LogLevelDebug LogLevel = "debug"
	LogLevelInfo  LogLevel = "info"
	LogLevelWarn  LogLevel = "warn"
	LogLevelError LogLevel = "error"
)

// StructuredLogger provides OpenTelemetry-based structured logging
type StructuredLogger struct {
	logger log.Logger
	ctx    context.Context
}

// LoggerProvider manages the OpenTelemetry logging setup
type LoggerProvider struct {
	provider     *sdklog.LoggerProvider
	OTLPEnabled  bool
	OTLPEndpoint string
	ErrorMessage string
}

// NewLoggerProvider creates a new OpenTelemetry logger provider
func NewLoggerProvider(ctx context.Context, selfMonitoringEnabled bool, otlpEndpoint string, otlpLogsEndpoint string, serviceName string, serviceVersion string, namespace string) (*LoggerProvider, error) {
	// Create resource with Kubernetes semantic conventions
	res, err := resource.New(ctx,
		resource.WithAttributes(
			semconv.ServiceNameKey.String(serviceName),
			semconv.ServiceVersionKey.String(serviceVersion),
			semconv.ServiceNamespaceKey.String(namespace),
			semconv.ServiceInstanceIDKey.String(getServiceInstanceID()),
			semconv.DeploymentEnvironmentKey.String(getDeploymentEnvironment()),
			semconv.K8SNamespaceNameKey.String(namespace),
			semconv.K8SPodNameKey.String(getPodName()),
			semconv.K8SContainerNameKey.String("openlit-operator"),
			semconv.TelemetrySDKNameKey.String("openlit-operator"),
			semconv.TelemetrySDKLanguageKey.String("go"),
			semconv.TelemetrySDKVersionKey.String("1.0.0"),
		),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to create resource: %w", err)
	}

	var processor sdklog.Processor
	var otlpAvailable bool

	if selfMonitoringEnabled && (otlpEndpoint != "" || otlpLogsEndpoint != "") {
		// Determine which endpoint to use (specific logs endpoint takes priority)
		logsEndpoint := otlpLogsEndpoint
		if logsEndpoint == "" {
			logsEndpoint = otlpEndpoint
		}

		// Try to create OTLP exporter for self-monitoring
		// Note: Headers are automatically handled by OTEL_EXPORTER_OTLP_HEADERS env var
		exporter, err := otlploghttp.New(ctx,
			otlploghttp.WithEndpointURL(logsEndpoint),
		)
		if err != nil {
			// OTLP backend unavailable - will use stdout fallback
			otlpAvailable = false
		} else {
			processor = sdklog.NewBatchProcessor(exporter)
			otlpAvailable = true
		}
	}

	// Use stdout exporter as fallback when OTLP is unavailable or disabled
	if !otlpAvailable {
		exporter, err := stdoutlog.New()
		if err != nil {
			return nil, fmt.Errorf("failed to create stdout log exporter: %w", err)
		}
		processor = sdklog.NewBatchProcessor(exporter)
	}

	provider := sdklog.NewLoggerProvider(
		sdklog.WithResource(res),
		sdklog.WithProcessor(processor),
	)

	// Set global logger provider
	global.SetLoggerProvider(provider)

	// Prepare connection status information
	var finalEndpoint string
	var errorMsg string

	if selfMonitoringEnabled && (otlpEndpoint != "" || otlpLogsEndpoint != "") {
		finalEndpoint = otlpLogsEndpoint
		if finalEndpoint == "" {
			finalEndpoint = otlpEndpoint
		}
		if !otlpAvailable {
			errorMsg = "OTLP backend connection failed"
		}
	}

	return &LoggerProvider{
		provider:     provider,
		OTLPEnabled:  otlpAvailable,
		OTLPEndpoint: finalEndpoint,
		ErrorMessage: errorMsg,
	}, nil
}

// Shutdown gracefully shuts down the logger provider
func (lp *LoggerProvider) Shutdown(ctx context.Context) error {
	return lp.provider.Shutdown(ctx)
}

// NewStructuredLogger creates a new structured logger with OpenTelemetry semantic conventions
func NewStructuredLogger(otelLogger log.Logger, ctx context.Context) *StructuredLogger {
	return &StructuredLogger{
		logger: otelLogger,
		ctx:    ctx,
	}
}

// NewLogger creates a new logger for a specific component
func NewLogger(component string) *StructuredLogger {
	logger := global.GetLoggerProvider().Logger(
		"openlit-operator",
		log.WithInstrumentationVersion("1.0.0"),
	)

	ctx := context.WithValue(context.Background(), "component", component)

	return &StructuredLogger{
		logger: logger,
		ctx:    ctx,
	}
}

// Debug logs a debug message with structured attributes
func (sl *StructuredLogger) Debug(message string, attrs ...interface{}) {
	sl.emit(log.SeverityDebug, message, attrs...)
}

// Info logs an info message with structured attributes
func (sl *StructuredLogger) Info(message string, attrs ...interface{}) {
	sl.emit(log.SeverityInfo, message, attrs...)
}

// Warn logs a warning message with structured attributes
func (sl *StructuredLogger) Warn(message string, attrs ...interface{}) {
	sl.emit(log.SeverityWarn, message, attrs...)
}

// Error logs an error message with structured attributes
func (sl *StructuredLogger) Error(message string, err error, attrs ...interface{}) {
	allAttrs := append(attrs, "error", err.Error())
	sl.emit(log.SeverityError, message, allAttrs...)
}

// WithContext returns a new logger with the given context
func (sl *StructuredLogger) WithContext(ctx context.Context) *StructuredLogger {
	return &StructuredLogger{
		logger: sl.logger,
		ctx:    ctx,
	}
}

// WithComponent returns a new logger with the component attribute set
func (sl *StructuredLogger) WithComponent(component string) *StructuredLogger {
	ctx := context.WithValue(sl.ctx, "component", component)
	return &StructuredLogger{
		logger: sl.logger,
		ctx:    ctx,
	}
}

// emit logs a message with the given severity and attributes
func (sl *StructuredLogger) emit(severity log.Severity, message string, attrs ...interface{}) {
	// Convert key-value pairs to log attributes
	var logAttrs []log.KeyValue

	// Add component from context if available
	if component := sl.ctx.Value("component"); component != nil {
		logAttrs = append(logAttrs, log.String("component", component.(string)))
	}

	// Process variadic attributes
	for i := 0; i < len(attrs); i += 2 {
		if i+1 < len(attrs) {
			key := fmt.Sprintf("%v", attrs[i])
			value := attrs[i+1]

			switch v := value.(type) {
			case string:
				logAttrs = append(logAttrs, log.String(key, v))
			case int:
				logAttrs = append(logAttrs, log.Int64(key, int64(v)))
			case int64:
				logAttrs = append(logAttrs, log.Int64(key, v))
			case int32:
				logAttrs = append(logAttrs, log.Int64(key, int64(v)))
			case float64:
				logAttrs = append(logAttrs, log.Float64(key, v))
			case bool:
				logAttrs = append(logAttrs, log.Bool(key, v))
			case error:
				logAttrs = append(logAttrs, log.String(key, v.Error()))
			default:
				logAttrs = append(logAttrs, log.String(key, fmt.Sprintf("%v", v)))
			}
		}
	}

	// Create log record
	record := log.Record{}
	record.SetTimestamp(record.Timestamp())
	record.SetSeverity(severity)
	record.SetBody(log.StringValue(message))
	record.AddAttributes(logAttrs...)

	// Emit the log record
	sl.logger.Emit(sl.ctx, record)
}

// Helper functions to get Kubernetes resource attributes
func getServiceInstanceID() string {
	hostname := os.Getenv("HOSTNAME")
	if hostname == "" {
		hostname = "unknown"
	}
	return hostname
}

func getDeploymentEnvironment() string {
	if env := os.Getenv("DEPLOYMENT_ENVIRONMENT"); env != "" {
		return env
	}
	if env := os.Getenv("ENVIRONMENT"); env != "" {
		return env
	}
	return "kubernetes"
}

func getPodName() string {
	// Try to get actual pod name from hostname (Kubernetes sets this to pod name)
	if hostname, err := os.Hostname(); err == nil && hostname != "" {
		return hostname
	}
	// Fallback to operator name
	return "openlit-operator"
}

// getNamespaceFromServiceAccount gets the current namespace from the service account token
func getNamespaceFromServiceAccount() string {
	// Read namespace from service account token (standard Kubernetes mount)
	if data, err := ioutil.ReadFile("/var/run/secrets/kubernetes.io/serviceaccount/namespace"); err == nil {
		namespace := strings.TrimSpace(string(data))
		if namespace != "" {
			return namespace
		}
	}
	// Fallback to default
	return "openlit"
}

// OpenTelemetryLogr implements logr.LogSink to bridge controller-runtime logging with OpenTelemetry
type OpenTelemetryLogr struct {
	logger *StructuredLogger
	name   string
	level  int
}

// NewLogr creates a new logr.Logger that uses OpenTelemetry structured logging
// This ensures controller-runtime logs are captured in our telemetry system
func NewLogr(name string) logr.Logger {
	return logr.New(&OpenTelemetryLogr{
		logger: NewLogger(name),
		name:   name,
		level:  0,
	})
}

// Init implements logr.LogSink
func (l *OpenTelemetryLogr) Init(info logr.RuntimeInfo) {
	// No initialization needed
}

// Enabled implements logr.LogSink
func (l *OpenTelemetryLogr) Enabled(level int) bool {
	// Enable all log levels
	return level >= l.level
}

// Info implements logr.LogSink
func (l *OpenTelemetryLogr) Info(level int, msg string, keysAndValues ...interface{}) {
	if level >= l.level {
		l.logger.Info(msg, keysAndValues...)
	}
}

// Error implements logr.LogSink
func (l *OpenTelemetryLogr) Error(err error, msg string, keysAndValues ...interface{}) {
	l.logger.Error(msg, err, keysAndValues...)
}

// WithValues implements logr.LogSink
func (l *OpenTelemetryLogr) WithValues(keysAndValues ...interface{}) logr.LogSink {
	return &OpenTelemetryLogr{
		logger: l.logger,
		name:   l.name,
		level:  l.level,
	}
}

// WithName implements logr.LogSink
func (l *OpenTelemetryLogr) WithName(name string) logr.LogSink {
	newName := l.name
	if name != "" {
		if l.name != "" {
			newName = l.name + "." + name
		} else {
			newName = name
		}
	}
	return &OpenTelemetryLogr{
		logger: NewLogger(newName),
		name:   newName,
		level:  l.level,
	}
}
