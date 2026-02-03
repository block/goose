package constants

import (
	"os"
	"path/filepath"
)

// Default certificate paths - can be overridden by environment variables
var (
	// Base directory for certificates
	CertificatesDirectory = getEnvOrDefault("OPAMP_CERTS_DIR", "/app/opamp/certs")

	// Certificate Authority files
	CaCertPath     = filepath.Join(CertificatesDirectory, "cert", "ca.cert.pem")
	PrivateKeyPath = filepath.Join(CertificatesDirectory, "private", "ca.key.pem")

	// Server certificate files
	ServerCertPath    = filepath.Join(CertificatesDirectory, "server", "server.cert.pem")
	ServerCertKeyPath = filepath.Join(CertificatesDirectory, "server", "server.key.pem")
	ServerCSRPath     = filepath.Join(CertificatesDirectory, "server", "server.csr")

	// Client certificate files
	ClientCertPath = filepath.Join(CertificatesDirectory, "client", "client.cert.pem")
	ClientKeyPath  = filepath.Join(CertificatesDirectory, "client", "client.key.pem")
	ClientCSRPath  = filepath.Join(CertificatesDirectory, "client", "client.csr")
)

// Agent configuration storage directory
const ConfigDirectory = "/app/client/data/configs"

// getEnvOrDefault returns environment variable value or default if not set
func getEnvOrDefault(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

// UpdateCertificatePaths updates all certificate paths based on a new base directory
func UpdateCertificatePaths(baseDir string) {
	CertificatesDirectory = baseDir
	CaCertPath = filepath.Join(baseDir, "cert", "ca.cert.pem")
	PrivateKeyPath = filepath.Join(baseDir, "private", "ca.key.pem")
	ServerCertPath = filepath.Join(baseDir, "server", "server.cert.pem")
	ServerCertKeyPath = filepath.Join(baseDir, "server", "server.key.pem")
	ServerCSRPath = filepath.Join(baseDir, "server", "server.csr")
	ClientCertPath = filepath.Join(baseDir, "client", "client.cert.pem")
	ClientKeyPath = filepath.Join(baseDir, "client", "client.key.pem")
	ClientCSRPath = filepath.Join(baseDir, "client", "client.csr")
}
