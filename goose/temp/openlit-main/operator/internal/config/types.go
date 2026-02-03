package config

import (
	"fmt"
)

// OperatorConfig holds all configuration for the OpenLIT operator
// This struct contains ONLY operator infrastructure fields
// All user-facing instrumentation config is defined in AutoInstrumentation CR
type OperatorConfig struct {
	// Webhook Configuration
	WebhookPort    int    `json:"webhookPort"`
	WebhookPath    string `json:"webhookPath"`
	WebhookCertDir string `json:"webhookCertDir"`

	// TLS Certificate Configuration
	CertValidityDays int `json:"certValidityDays"`
	CertRefreshDays  int `json:"certRefreshDays"`

	// Kubernetes Configuration
	Namespace   string `json:"namespace"`
	ServiceName string `json:"serviceName"`
	SecretName  string `json:"secretName"`
	ConfigName  string `json:"configName"`

	// Observability Configuration
	// MetricsPort removed - no metrics server implemented
	HealthPort            int    `json:"healthPort"`
	SelfMonitoringEnabled bool   `json:"selfMonitoringEnabled"`
	LogLevel              string `json:"logLevel"`

	// OpenTelemetry Configuration (for operator self-monitoring)
	OTLPEndpoint        string `json:"otlpEndpoint"`
	OTLPHeaders         string `json:"otlpHeaders"`
	OTLPLogsEndpoint    string `json:"otlpLogsEndpoint"`
	OTLPMetricsEndpoint string `json:"otlpMetricsEndpoint"`

	// Multi-Operator Support
	WatchNamespace string `json:"watchNamespace"`

	// Webhook Behavior Configuration
	FailurePolicy      string `json:"failurePolicy"`
	ReinvocationPolicy string `json:"reinvocationPolicy"`
}

// Validate validates the operator configuration
func (c *OperatorConfig) Validate() error {
	// Port validation
	if c.WebhookPort <= 0 || c.WebhookPort > 65535 {
		return fmt.Errorf("webhookPort must be between 1 and 65535, got %d", c.WebhookPort)
	}

	// MetricsPort validation removed - no metrics server implemented

	if c.HealthPort <= 0 || c.HealthPort > 65535 {
		return fmt.Errorf("healthPort must be between 1 and 65535, got %d", c.HealthPort)
	}

	// Certificate validation
	if c.CertValidityDays <= 0 || c.CertValidityDays > 3650 {
		return fmt.Errorf("certValidityDays must be between 1 and 3650, got %d", c.CertValidityDays)
	}

	if c.CertRefreshDays <= 0 || c.CertRefreshDays > 365 {
		return fmt.Errorf("certRefreshDays must be between 1 and 365, got %d", c.CertRefreshDays)
	}

	// Log level validation
	validLogLevels := map[string]bool{
		"debug": true,
		"info":  true,
		"warn":  true,
		"error": true,
	}

	if !validLogLevels[c.LogLevel] {
		return fmt.Errorf("logLevel must be one of: debug, info, warn, error, got %s", c.LogLevel)
	}

	return nil
}

// GetWebhookServerAddress returns the webhook server address
func (c *OperatorConfig) GetWebhookServerAddress() string {
	return fmt.Sprintf(":%d", c.WebhookPort)
}

// GetMetricsServerAddress removed - no metrics server implemented

// GetHealthServerAddress returns the health server address
func (c *OperatorConfig) GetHealthServerAddress() string {
	return fmt.Sprintf(":%d", c.HealthPort)
}

// IsNamespaceScoped returns true if the operator should watch a specific namespace
func (c *OperatorConfig) IsNamespaceScoped() bool {
	return c.WatchNamespace != ""
}

// GetWatchedNamespaces returns the list of namespaces to watch
func (c *OperatorConfig) GetWatchedNamespaces() []string {
	if c.WatchNamespace == "" {
		return nil // Watch all namespaces
	}
	return []string{c.WatchNamespace}
}
