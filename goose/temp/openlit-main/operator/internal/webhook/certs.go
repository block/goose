package webhook

import (
	"context"
	"crypto/rand"
	"crypto/rsa"
	"crypto/tls"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/pem"
	"fmt"
	"math/big"
	"net"
	"os"
	"strings"
	"time"

	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/kubernetes"

	"github.com/openlit/openlit/operator/internal/observability"
)

// CertificateManager handles TLS certificate generation and management for the webhook
type CertificateManager struct {
	client           kubernetes.Interface
	namespace        string
	serviceName      string
	secretName       string
	validityDays     int
	refreshThreshold int
	logger           *observability.StructuredLogger
	rotationInterval time.Duration
	stopRotation     chan struct{}
}

// NewCertificateManager creates a new certificate manager
func NewCertificateManager(client kubernetes.Interface, namespace, serviceName, secretName string, validityDays, refreshThreshold int) *CertificateManager {
	return &CertificateManager{
		client:           client,
		namespace:        namespace,
		serviceName:      serviceName,
		secretName:       secretName,
		validityDays:     validityDays,
		refreshThreshold: refreshThreshold,
		logger:           observability.NewLogger("cert-manager"),
		rotationInterval: 6 * time.Hour, // Check every 6 hours for rotation
		stopRotation:     make(chan struct{}),
	}
}

// EnsureCertificate ensures that a valid TLS certificate exists for the webhook
// Handles multiple operators gracefully with resource ownership and conflict resolution
func (cm *CertificateManager) EnsureCertificate(ctx context.Context) ([]byte, error) {
	cm.logger.Info("Ensuring TLS certificate for webhook",
		"component", "cert-manager",
		"k8s.secret.name", cm.secretName,
		"k8s.namespace.name", cm.namespace,
		"tls.certificate.validity_days", cm.validityDays,
		"tls.certificate.refresh_threshold_days", cm.refreshThreshold)

	// Check if certificate already exists and is valid
	secret, err := cm.client.CoreV1().Secrets(cm.namespace).Get(ctx, cm.secretName, metav1.GetOptions{})
	if err == nil {
		// Check resource ownership for multiple operator scenarios
		if cm.shouldManageSecret(secret) {
			if cm.isCertificateValid(secret) {
				cm.logger.Info("Valid certificate already exists and owned by this operator",
					"component", "cert-manager",
					"k8s.secret.name", cm.secretName,
					"certificate.managed_by", secret.Labels["managed-by"])
				return secret.Data["ca.crt"], nil
			}
			cm.logger.Info("Existing certificate is invalid, regenerating",
				"component", "cert-manager",
				"k8s.secret.name", cm.secretName,
				"reason", "certificate_expired_or_invalid")
		} else {
			cm.logger.Warn("Certificate secret exists but managed by different operator",
				"component", "cert-manager",
				"k8s.secret.name", cm.secretName,
				"certificate.managed_by", secret.Labels["managed-by"],
				"current.operator", cm.getOperatorIdentity())

			// Use existing certificate from other operator if valid
			if cm.isCertificateValid(secret) {
				cm.logger.Info("Using valid certificate from other operator",
					"component", "cert-manager",
					"k8s.secret.name", cm.secretName)
				return secret.Data["ca.crt"], nil
			}
		}
	}

	// Generate new certificate
	caCert, caKey, err := cm.generateCACertificate()
	if err != nil {
		return nil, fmt.Errorf("failed to generate CA certificate: %w", err)
	}

	serverCert, serverKey, err := cm.generateServerCertificate(caCert, caKey)
	if err != nil {
		return nil, fmt.Errorf("failed to generate server certificate: %w", err)
	}

	// Create or update the secret
	secretData := map[string][]byte{
		"ca.crt":  caCert,
		"tls.crt": serverCert,
		"tls.key": serverKey,
	}

	if err := cm.createOrUpdateSecret(ctx, secretData); err != nil {
		return nil, fmt.Errorf("failed to create/update secret: %w", err)
	}

	cm.logger.Info("✅ TLS certificate generated and stored successfully",
		"component", "cert-manager",
		"k8s.secret.name", cm.secretName,
		"k8s.namespace.name", cm.namespace,
		"tls.certificate.validity_days", cm.validityDays)
	return caCert, nil
}

// generateCACertificate generates a self-signed CA certificate
func (cm *CertificateManager) generateCACertificate() ([]byte, *rsa.PrivateKey, error) {
	// Generate CA private key
	caPrivKey, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		return nil, nil, err
	}

	// Create CA certificate template
	caTemplate := x509.Certificate{
		SerialNumber: big.NewInt(1),
		Subject: pkix.Name{
			CommonName: "OpenLIT Webhook CA",
		},
		NotBefore:             time.Now(),
		NotAfter:              time.Now().Add(time.Duration(cm.validityDays) * 24 * time.Hour),
		KeyUsage:              x509.KeyUsageCertSign | x509.KeyUsageDigitalSignature,
		ExtKeyUsage:           []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth},
		BasicConstraintsValid: true,
		IsCA:                  true,
	}

	// Generate CA certificate
	caCertDER, err := x509.CreateCertificate(rand.Reader, &caTemplate, &caTemplate, &caPrivKey.PublicKey, caPrivKey)
	if err != nil {
		return nil, nil, err
	}

	// Encode CA certificate to PEM
	caCertPEM := pem.EncodeToMemory(&pem.Block{
		Type:  "CERTIFICATE",
		Bytes: caCertDER,
	})

	return caCertPEM, caPrivKey, nil
}

// generateServerCertificate generates a server certificate signed by the CA
func (cm *CertificateManager) generateServerCertificate(caCertPEM []byte, caKey *rsa.PrivateKey) ([]byte, []byte, error) {
	// Parse CA certificate
	caCertBlock, _ := pem.Decode(caCertPEM)
	caCert, err := x509.ParseCertificate(caCertBlock.Bytes)
	if err != nil {
		return nil, nil, err
	}

	// Generate server private key
	serverPrivKey, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		return nil, nil, err
	}

	// Create server certificate template
	serverTemplate := x509.Certificate{
		SerialNumber: big.NewInt(2),
		Subject: pkix.Name{
			CommonName: fmt.Sprintf("%s.%s.svc", cm.serviceName, cm.namespace),
		},
		NotBefore:   time.Now(),
		NotAfter:    time.Now().Add(time.Duration(cm.validityDays) * 24 * time.Hour),
		KeyUsage:    x509.KeyUsageDigitalSignature | x509.KeyUsageKeyEncipherment,
		ExtKeyUsage: []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth},
		DNSNames: []string{
			cm.serviceName,
			fmt.Sprintf("%s.%s", cm.serviceName, cm.namespace),
			fmt.Sprintf("%s.%s.svc", cm.serviceName, cm.namespace),
			fmt.Sprintf("%s.%s.svc.cluster.local", cm.serviceName, cm.namespace),
		},
		IPAddresses: []net.IP{
			net.ParseIP("127.0.0.1"),
		},
	}

	// Generate server certificate
	serverCertDER, err := x509.CreateCertificate(rand.Reader, &serverTemplate, caCert, &serverPrivKey.PublicKey, caKey)
	if err != nil {
		return nil, nil, err
	}

	// Encode server certificate to PEM
	serverCertPEM := pem.EncodeToMemory(&pem.Block{
		Type:  "CERTIFICATE",
		Bytes: serverCertDER,
	})

	// Encode server private key to PEM
	serverKeyPEM := pem.EncodeToMemory(&pem.Block{
		Type:  "RSA PRIVATE KEY",
		Bytes: x509.MarshalPKCS1PrivateKey(serverPrivKey),
	})

	return serverCertPEM, serverKeyPEM, nil
}

// isCertificateValid checks if the existing certificate is still valid
func (cm *CertificateManager) isCertificateValid(secret *corev1.Secret) bool {
	certPEM, exists := secret.Data["tls.crt"]
	if !exists {
		return false
	}

	certBlock, _ := pem.Decode(certPEM)
	if certBlock == nil {
		return false
	}

	cert, err := x509.ParseCertificate(certBlock.Bytes)
	if err != nil {
		return false
	}

	// Check if certificate is still valid (not expired and valid for at least refreshThreshold more days)
	return time.Now().Add(time.Duration(cm.refreshThreshold) * 24 * time.Hour).Before(cert.NotAfter)
}

// createOrUpdateSecret creates or updates the TLS secret with proper ownership labels
func (cm *CertificateManager) createOrUpdateSecret(ctx context.Context, data map[string][]byte) error {
	operatorIdentity := cm.getOperatorIdentity()

	secret := &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{
			Name:      cm.secretName,
			Namespace: cm.namespace,
			Labels: map[string]string{
				"app":        "openlit-operator",
				"component":  "webhook-certs",
				"managed-by": operatorIdentity,
			},
		},
		Type: corev1.SecretTypeTLS,
		Data: data,
	}

	_, err := cm.client.CoreV1().Secrets(cm.namespace).Get(ctx, cm.secretName, metav1.GetOptions{})
	if err != nil {
		// Secret doesn't exist, create it
		_, err = cm.client.CoreV1().Secrets(cm.namespace).Create(ctx, secret, metav1.CreateOptions{})
		return err
	}

	// Secret exists, update it
	_, err = cm.client.CoreV1().Secrets(cm.namespace).Update(ctx, secret, metav1.UpdateOptions{})
	return err
}

// GetTLSConfig returns a TLS config for the webhook server
func (cm *CertificateManager) GetTLSConfig(ctx context.Context) (*tls.Config, error) {
	secret, err := cm.client.CoreV1().Secrets(cm.namespace).Get(ctx, cm.secretName, metav1.GetOptions{})
	if err != nil {
		return nil, fmt.Errorf("failed to get TLS secret: %w", err)
	}

	cert, err := tls.X509KeyPair(secret.Data["tls.crt"], secret.Data["tls.key"])
	if err != nil {
		return nil, fmt.Errorf("failed to load key pair: %w", err)
	}

	return &tls.Config{
		Certificates: []tls.Certificate{cert},
	}, nil
}

// shouldManageSecret determines if this operator should manage the given secret
// Handles multiple operator scenarios:
// 1. Same deployment (multiple replicas) - shared ownership via deployment name
// 2. Multiple operators in same namespace - first-come-first-served
// 3. Multiple operators in different namespaces - namespace isolation
func (cm *CertificateManager) shouldManageSecret(secret *corev1.Secret) bool {
	if secret.Labels == nil {
		// Secret has no ownership labels - we can manage it
		return true
	}

	managedBy, exists := secret.Labels["managed-by"]
	if !exists {
		// Secret has no managed-by label - we can manage it
		return true
	}

	currentOperator := cm.getOperatorIdentity()

	// If we're the same operator identity, we should manage it
	if managedBy == currentOperator {
		return true
	}

	// Check if it's managed by a replica of the same deployment
	if cm.isSameDeployment(managedBy, currentOperator) {
		return true
	}

	// Different operator manages this secret
	return false
}

// getOperatorIdentity returns a unique identifier for this operator instance
// Format: deployment-name.namespace for deployment-based operators
// Format: pod-name.namespace for standalone operators
func (cm *CertificateManager) getOperatorIdentity() string {
	// Try to get deployment name from environment or hostname patterns
	hostname := os.Getenv("HOSTNAME")
	if hostname == "" {
		hostname = "openlit-operator"
	}

	// Extract deployment name from pod name (removes replica hash)
	// Pod names in deployments follow pattern: deployment-name-replicaset-hash-pod-hash
	deploymentName := cm.extractDeploymentName(hostname)

	return fmt.Sprintf("%s.%s", deploymentName, cm.namespace)
}

// extractDeploymentName extracts the deployment name from a pod hostname
// For deployments: "deployment-name-abc123-def456" -> "deployment-name"
// For standalone pods: uses the full hostname
func (cm *CertificateManager) extractDeploymentName(hostname string) string {
	// Split by hyphens and try to find deployment pattern
	parts := strings.Split(hostname, "-")

	// For typical deployment pods, remove the last two parts (replicaset hash + pod hash)
	if len(parts) >= 3 {
		// Check if last two parts look like Kubernetes generated hashes (alphanumeric, 5-10 chars)
		lastPart := parts[len(parts)-1]
		secondLastPart := parts[len(parts)-2]

		if cm.looksLikeHash(lastPart) && cm.looksLikeHash(secondLastPart) {
			// Reconstruct deployment name without the hash parts
			return strings.Join(parts[:len(parts)-2], "-")
		}
	}

	// If pattern doesn't match, use the full hostname
	return hostname
}

// looksLikeHash checks if a string looks like a Kubernetes-generated hash
func (cm *CertificateManager) looksLikeHash(s string) bool {
	if len(s) < 5 || len(s) > 10 {
		return false
	}

	for _, c := range s {
		if !((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9')) {
			return false
		}
	}

	return true
}

// isSameDeployment checks if two operator identities belong to the same deployment
func (cm *CertificateManager) isSameDeployment(identity1, identity2 string) bool {
	// Extract deployment names from identities
	deployment1 := strings.Split(identity1, ".")[0]
	deployment2 := strings.Split(identity2, ".")[0]

	return deployment1 == deployment2
}

// StartCertificateRotation starts the automatic certificate rotation background process
func (cm *CertificateManager) StartCertificateRotation(ctx context.Context) {
	cm.logger.Info("Starting automatic certificate rotation",
		"component", "cert-manager",
		"rotation.interval", cm.rotationInterval.String(),
		"rotation.threshold_days", cm.refreshThreshold)

	go cm.rotationLoop(ctx)
}

// StopCertificateRotation stops the automatic certificate rotation background process
func (cm *CertificateManager) StopCertificateRotation() {
	cm.logger.Info("Stopping automatic certificate rotation",
		"component", "cert-manager")

	close(cm.stopRotation)
}

// rotationLoop runs the certificate rotation check loop
func (cm *CertificateManager) rotationLoop(ctx context.Context) {
	ticker := time.NewTicker(cm.rotationInterval)
	defer ticker.Stop()

	cm.logger.Info("Certificate rotation loop started",
		"component", "cert-manager",
		"check.interval", cm.rotationInterval.String())

	for {
		select {
		case <-ctx.Done():
			cm.logger.Info("Certificate rotation stopped - context cancelled",
				"component", "cert-manager")
			return
		case <-cm.stopRotation:
			cm.logger.Info("Certificate rotation stopped - stop signal received",
				"component", "cert-manager")
			return
		case <-ticker.C:
			cm.checkAndRotateCertificate(ctx)
		}
	}
}

// checkAndRotateCertificate checks if certificate needs rotation and performs it
func (cm *CertificateManager) checkAndRotateCertificate(ctx context.Context) {
	cm.logger.Info("Performing periodic certificate rotation check",
		"component", "cert-manager",
		"k8s.secret.name", cm.secretName)

	// Get current certificate
	secret, err := cm.client.CoreV1().Secrets(cm.namespace).Get(ctx, cm.secretName, metav1.GetOptions{})
	if err != nil {
		cm.logger.Warn("Certificate rotation check failed - secret not found",
			"component", "cert-manager",
			"k8s.secret.name", cm.secretName,
			"error", err.Error())
		return
	}

	// Check if we should manage this secret
	if !cm.shouldManageSecret(secret) {
		cm.logger.Info("Skipping certificate rotation - secret managed by different operator",
			"component", "cert-manager",
			"k8s.secret.name", cm.secretName,
			"certificate.managed_by", secret.Labels["managed-by"])
		return
	}

	// Check if certificate needs rotation
	needsRotation, reason := cm.needsCertificateRotation(secret)
	if !needsRotation {
		cm.logger.Info("Certificate rotation check passed - no rotation needed",
			"component", "cert-manager",
			"k8s.secret.name", cm.secretName,
			"certificate.status", "valid")
		return
	}

	cm.logger.Warn("Certificate needs rotation",
		"component", "cert-manager",
		"k8s.secret.name", cm.secretName,
		"rotation.reason", reason)

	// Perform certificate rotation
	if err := cm.rotateCertificate(ctx); err != nil {
		cm.logger.Error("Certificate rotation failed", err,
			"component", "cert-manager",
			"k8s.secret.name", cm.secretName,
			"rotation.reason", reason)
	} else {
		cm.logger.Info("Certificate rotation completed successfully",
			"component", "cert-manager",
			"k8s.secret.name", cm.secretName,
			"rotation.reason", reason)
	}
}

// needsCertificateRotation checks if the certificate needs rotation and returns the reason
func (cm *CertificateManager) needsCertificateRotation(secret *corev1.Secret) (bool, string) {
	certPEM, exists := secret.Data["tls.crt"]
	if !exists {
		return true, "certificate_missing"
	}

	certBlock, _ := pem.Decode(certPEM)
	if certBlock == nil {
		return true, "certificate_invalid_format"
	}

	cert, err := x509.ParseCertificate(certBlock.Bytes)
	if err != nil {
		return true, "certificate_parse_error"
	}

	now := time.Now()

	// Check if certificate is already expired
	if now.After(cert.NotAfter) {
		return true, "certificate_expired"
	}

	// Check if certificate expires within the refresh threshold
	rotationTime := now.Add(time.Duration(cm.refreshThreshold) * 24 * time.Hour)
	if rotationTime.After(cert.NotAfter) {
		daysUntilExpiry := int(cert.NotAfter.Sub(now).Hours() / 24)
		return true, fmt.Sprintf("certificate_expiring_in_%d_days", daysUntilExpiry)
	}

	// Check if certificate has valid DNS names for our service
	expectedDNS := fmt.Sprintf("%s.%s.svc", cm.serviceName, cm.namespace)
	hasValidDNS := false
	for _, dnsName := range cert.DNSNames {
		if dnsName == expectedDNS || dnsName == fmt.Sprintf("%s.%s.svc.cluster.local", cm.serviceName, cm.namespace) {
			hasValidDNS = true
			break
		}
	}

	if !hasValidDNS {
		return true, "certificate_invalid_dns_names"
	}

	return false, "certificate_valid"
}

// rotateCertificate performs the actual certificate rotation
func (cm *CertificateManager) rotateCertificate(ctx context.Context) error {
	cm.logger.Info("Starting certificate rotation",
		"component", "cert-manager",
		"k8s.secret.name", cm.secretName)

	// Generate new certificate
	caCert, caKey, err := cm.generateCACertificate()
	if err != nil {
		return fmt.Errorf("failed to generate new CA certificate: %w", err)
	}

	serverCert, serverKey, err := cm.generateServerCertificate(caCert, caKey)
	if err != nil {
		return fmt.Errorf("failed to generate new server certificate: %w", err)
	}

	// Update the secret with new certificate
	secretData := map[string][]byte{
		"ca.crt":  caCert,
		"tls.crt": serverCert,
		"tls.key": serverKey,
	}

	if err := cm.createOrUpdateSecret(ctx, secretData); err != nil {
		return fmt.Errorf("failed to update secret with new certificate: %w", err)
	}

	cm.logger.Info("Certificate rotation completed",
		"component", "cert-manager",
		"k8s.secret.name", cm.secretName,
		"certificate.validity_days", cm.validityDays)

	return nil
}
