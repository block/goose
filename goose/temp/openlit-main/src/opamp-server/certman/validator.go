package certman

import (
	"crypto/x509"
	"encoding/pem"
	"fmt"
	"log"
	"os"
	"time"
)

// CertificateInfo holds information about a certificate
type CertificateInfo struct {
	Subject      string
	Issuer       string
	SerialNumber string
	NotBefore    time.Time
	NotAfter     time.Time
	DNSNames     []string
	IPAddresses  []string
	IsCA         bool
	KeyUsage     x509.KeyUsage
	ExtKeyUsage  []x509.ExtKeyUsage
}

// CertificateValidator provides certificate validation functionality
type CertificateValidator struct {
	logger *log.Logger
}

// NewCertificateValidator creates a new certificate validator
func NewCertificateValidator() *CertificateValidator {
	return &CertificateValidator{
		logger: logger,
	}
}

// ValidateCertificateFile validates a certificate file
func (cv *CertificateValidator) ValidateCertificateFile(certPath string) (*CertificateInfo, error) {
	// Read certificate file
	certBytes, err := os.ReadFile(certPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read certificate file %s: %v", certPath, err)
	}

	return cv.ValidateCertificateBytes(certBytes)
}

// ValidateCertificateBytes validates certificate bytes
func (cv *CertificateValidator) ValidateCertificateBytes(certBytes []byte) (*CertificateInfo, error) {
	// Decode PEM block
	block, _ := pem.Decode(certBytes)
	if block == nil {
		return nil, fmt.Errorf("failed to decode PEM block")
	}

	// Parse certificate
	cert, err := x509.ParseCertificate(block.Bytes)
	if err != nil {
		return nil, fmt.Errorf("failed to parse certificate: %v", err)
	}

	// Extract certificate information
	info := &CertificateInfo{
		Subject:      cert.Subject.String(),
		Issuer:       cert.Issuer.String(),
		SerialNumber: cert.SerialNumber.String(),
		NotBefore:    cert.NotBefore,
		NotAfter:     cert.NotAfter,
		DNSNames:     cert.DNSNames,
		IsCA:         cert.IsCA,
		KeyUsage:     cert.KeyUsage,
		ExtKeyUsage:  cert.ExtKeyUsage,
	}

	// Convert IP addresses to strings
	for _, ip := range cert.IPAddresses {
		info.IPAddresses = append(info.IPAddresses, ip.String())
	}

	return info, nil
}

// CheckCertificateExpiry checks if a certificate is expired or expiring soon
func (cv *CertificateValidator) CheckCertificateExpiry(certPath string, warningDays int) error {
	info, err := cv.ValidateCertificateFile(certPath)
	if err != nil {
		return err
	}

	now := time.Now()

	// Check if certificate is expired
	if now.After(info.NotAfter) {
		return fmt.Errorf("certificate %s is expired (expired on %s)", certPath, info.NotAfter.Format(time.RFC3339))
	}

	// Check if certificate is expiring soon
	warningTime := now.Add(time.Duration(warningDays) * 24 * time.Hour)
	if warningTime.After(info.NotAfter) {
		cv.logger.Printf("WARNING: Certificate %s will expire in %d days (expires on %s)",
			certPath, int(info.NotAfter.Sub(now).Hours()/24), info.NotAfter.Format(time.RFC3339))
	}

	return nil
}

// VerifyCertificateChain verifies that a certificate is signed by the CA
func (cv *CertificateValidator) VerifyCertificateChain(certPath, caCertPath string) error {
	// Read CA certificate
	caCertBytes, err := os.ReadFile(caCertPath)
	if err != nil {
		return fmt.Errorf("failed to read CA certificate: %v", err)
	}

	caBlock, _ := pem.Decode(caCertBytes)
	if caBlock == nil {
		return fmt.Errorf("failed to decode CA certificate PEM")
	}

	caCert, err := x509.ParseCertificate(caBlock.Bytes)
	if err != nil {
		return fmt.Errorf("failed to parse CA certificate: %v", err)
	}

	// Read certificate to verify
	certBytes, err := os.ReadFile(certPath)
	if err != nil {
		return fmt.Errorf("failed to read certificate: %v", err)
	}

	certBlock, _ := pem.Decode(certBytes)
	if certBlock == nil {
		return fmt.Errorf("failed to decode certificate PEM")
	}

	cert, err := x509.ParseCertificate(certBlock.Bytes)
	if err != nil {
		return fmt.Errorf("failed to parse certificate: %v", err)
	}

	// Create certificate pool with CA
	caPool := x509.NewCertPool()
	caPool.AddCert(caCert)

	// Verify certificate
	opts := x509.VerifyOptions{
		Roots: caPool,
	}

	_, err = cert.Verify(opts)
	if err != nil {
		return fmt.Errorf("certificate verification failed: %v", err)
	}

	cv.logger.Printf("Certificate %s successfully verified against CA %s", certPath, caCertPath)
	return nil
}

// ValidateKeyPair validates that a certificate and private key match
func (cv *CertificateValidator) ValidateKeyPair(certPath, keyPath string) error {
	// This is a basic validation - in production you might want more thorough checks
	certInfo, err := os.Stat(certPath)
	if err != nil {
		return fmt.Errorf("certificate file not found: %v", err)
	}

	keyInfo, err := os.Stat(keyPath)
	if err != nil {
		return fmt.Errorf("private key file not found: %v", err)
	}

	// Check file permissions
	if certInfo.Mode().Perm() != 0644 {
		cv.logger.Printf("WARNING: Certificate file %s has permissions %o, should be 644", certPath, certInfo.Mode().Perm())
	}

	if keyInfo.Mode().Perm() != 0600 {
		cv.logger.Printf("WARNING: Private key file %s has permissions %o, should be 600", keyPath, keyInfo.Mode().Perm())
	}

	return nil
}

// GetCertificateInfo returns detailed information about a certificate
func (cv *CertificateValidator) GetCertificateInfo(certPath string) (*CertificateInfo, error) {
	return cv.ValidateCertificateFile(certPath)
}

// CheckAllCertificates performs comprehensive validation of all certificates
func (cv *CertificateValidator) CheckAllCertificates(caCertPath, serverCertPath, serverKeyPath, clientCertPath, clientKeyPath string) error {
	cv.logger.Printf("Starting comprehensive certificate validation...")

	// Check CA certificate
	cv.logger.Printf("Validating CA certificate...")
	caInfo, err := cv.ValidateCertificateFile(caCertPath)
	if err != nil {
		return fmt.Errorf("CA certificate validation failed: %v", err)
	}

	if !caInfo.IsCA {
		return fmt.Errorf("CA certificate is not marked as CA")
	}

	// Check CA certificate expiry
	if err := cv.CheckCertificateExpiry(caCertPath, 30); err != nil {
		return fmt.Errorf("CA certificate expiry check failed: %v", err)
	}

	// Check server certificate
	cv.logger.Printf("Validating server certificate...")
	if err := cv.ValidateKeyPair(serverCertPath, serverKeyPath); err != nil {
		return fmt.Errorf("server certificate/key validation failed: %v", err)
	}

	if err := cv.VerifyCertificateChain(serverCertPath, caCertPath); err != nil {
		return fmt.Errorf("server certificate chain validation failed: %v", err)
	}

	if err := cv.CheckCertificateExpiry(serverCertPath, 30); err != nil {
		return fmt.Errorf("server certificate expiry check failed: %v", err)
	}

	// Check client certificate
	cv.logger.Printf("Validating client certificate...")
	if err := cv.ValidateKeyPair(clientCertPath, clientKeyPath); err != nil {
		return fmt.Errorf("client certificate/key validation failed: %v", err)
	}

	if err := cv.VerifyCertificateChain(clientCertPath, caCertPath); err != nil {
		return fmt.Errorf("client certificate chain validation failed: %v", err)
	}

	if err := cv.CheckCertificateExpiry(clientCertPath, 30); err != nil {
		return fmt.Errorf("client certificate expiry check failed: %v", err)
	}

	cv.logger.Printf("All certificate validations passed successfully!")
	return nil
}
