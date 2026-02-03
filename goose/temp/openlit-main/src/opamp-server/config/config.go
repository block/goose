package config

import (
	"os"
	"strconv"
	"strings"
)

// TLSConfig holds TLS-related configuration
type TLSConfig struct {
	// Certificate paths
	CertificatesDirectory string
	CaCertPath            string
	ServerCertPath        string
	ServerCertKeyPath     string
	ClientCertPath        string
	ClientKeyPath         string
	PrivateKeyPath        string

	// TLS behavior
	InsecureSkipVerify bool
	RequireClientCert  bool
	MinTLSVersion      string
	MaxTLSVersion      string

	// Environment
	Environment string
}

// ServerConfig holds server configuration
type ServerConfig struct {
	// Server settings
	ListenAddress string
	ListenPort    string
	APIPort       string

	// TLS configuration
	TLS TLSConfig

	// Logging
	LogLevel string
}

// NewConfig creates a new configuration with environment-aware defaults
func NewConfig() *ServerConfig {
	config := &ServerConfig{
		ListenAddress: "0.0.0.0",
		ListenPort:    "4320",
		APIPort:       "8080",
		LogLevel:      getEnvOrDefault("OPAMP_LOG_LEVEL", "info"),
	}

	// Initialize TLS configuration
	config.TLS = TLSConfig{
		Environment:           getEnvOrDefault("OPAMP_ENVIRONMENT", "development"),
		CertificatesDirectory: getEnvOrDefault("OPAMP_CERTS_DIR", "/app/opamp/certs"),
		MinTLSVersion:         getEnvOrDefault("OPAMP_TLS_MIN_VERSION", "1.2"),
		MaxTLSVersion:         getEnvOrDefault("OPAMP_TLS_MAX_VERSION", "1.3"),
		InsecureSkipVerify:    getEnvBool("OPAMP_TLS_INSECURE_SKIP_VERIFY", false),
		RequireClientCert:     getEnvBool("OPAMP_TLS_REQUIRE_CLIENT_CERT", true),
	}

	// Set certificate paths based on certificates directory
	config.TLS.CaCertPath = config.TLS.CertificatesDirectory + "/cert/ca.cert.pem"
	config.TLS.ServerCertPath = config.TLS.CertificatesDirectory + "/server/server.cert.pem"
	config.TLS.ServerCertKeyPath = config.TLS.CertificatesDirectory + "/server/server.key.pem"
	config.TLS.ClientCertPath = config.TLS.CertificatesDirectory + "/client/client.cert.pem"
	config.TLS.ClientKeyPath = config.TLS.CertificatesDirectory + "/client/client.key.pem"
	config.TLS.PrivateKeyPath = config.TLS.CertificatesDirectory + "/private/ca.key.pem"

	// Environment-specific overrides
	switch strings.ToLower(config.TLS.Environment) {
	case "development", "dev":
		config.TLS.InsecureSkipVerify = getEnvBool("OPAMP_TLS_INSECURE_SKIP_VERIFY", true)
		config.TLS.RequireClientCert = getEnvBool("OPAMP_TLS_REQUIRE_CLIENT_CERT", false)
	case "production", "prod":
		config.TLS.InsecureSkipVerify = getEnvBool("OPAMP_TLS_INSECURE_SKIP_VERIFY", false)
		config.TLS.RequireClientCert = getEnvBool("OPAMP_TLS_REQUIRE_CLIENT_CERT", true)
	case "testing", "test":
		config.TLS.InsecureSkipVerify = getEnvBool("OPAMP_TLS_INSECURE_SKIP_VERIFY", true)
		config.TLS.RequireClientCert = getEnvBool("OPAMP_TLS_REQUIRE_CLIENT_CERT", false)
	}

	return config
}

// IsProduction returns true if running in production environment
func (c *ServerConfig) IsProduction() bool {
	env := strings.ToLower(c.TLS.Environment)
	return env == "production" || env == "prod"
}

// IsDevelopment returns true if running in development environment
func (c *ServerConfig) IsDevelopment() bool {
	env := strings.ToLower(c.TLS.Environment)
	return env == "development" || env == "dev"
}

// GetListenEndpoint returns the formatted listen endpoint
func (c *ServerConfig) GetListenEndpoint() string {
	return c.ListenAddress + ":" + c.ListenPort
}

// GetAPIEndpoint returns the formatted API endpoint
func (c *ServerConfig) GetAPIEndpoint() string {
	return c.ListenAddress + ":" + c.APIPort
}

// Validate checks if the configuration is valid
func (c *ServerConfig) Validate() error {
	// Check if certificate files exist in production
	if c.IsProduction() && !c.TLS.InsecureSkipVerify {
		if err := c.validateCertificateFiles(); err != nil {
			return err
		}
	}
	return nil
}

// validateCertificateFiles checks if required certificate files exist
func (c *ServerConfig) validateCertificateFiles() error {
	requiredFiles := []string{
		c.TLS.CaCertPath,
		c.TLS.ServerCertPath,
		c.TLS.ServerCertKeyPath,
	}

	for _, file := range requiredFiles {
		if _, err := os.Stat(file); os.IsNotExist(err) {
			return &CertificateError{
				Message: "Required certificate file not found: " + file,
				Path:    file,
			}
		}
	}
	return nil
}

// Helper functions
func getEnvOrDefault(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

func getEnvBool(key string, defaultValue bool) bool {
	if value := os.Getenv(key); value != "" {
		if parsed, err := strconv.ParseBool(value); err == nil {
			return parsed
		}
	}
	return defaultValue
}

// CertificateError represents a certificate-related error
type CertificateError struct {
	Message string
	Path    string
}

func (e *CertificateError) Error() string {
	return e.Message
}
