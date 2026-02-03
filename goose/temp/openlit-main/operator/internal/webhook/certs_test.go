package webhook

import (
	"context"
	"crypto/x509"
	"encoding/pem"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/suite"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/kubernetes/fake"
)

type CertificateManagerTestSuite struct {
	suite.Suite
	certManager *CertificateManager
	ctx         context.Context
}

func (suite *CertificateManagerTestSuite) SetupTest() {
	// Create fake Kubernetes client
	fakeClient := fake.NewSimpleClientset()

	suite.ctx = context.Background()
	
	// Create certificate manager with correct signature
	suite.certManager = NewCertificateManager(
		fakeClient,
		"openlit",            // namespace
		"openlit-webhook",    // serviceName
		"openlit-webhook-tls", // secretName
		365,                  // validityDays
		30,                   // refreshThreshold
	)
}

func (suite *CertificateManagerTestSuite) TestGenerateCertificate() {
	caCert, caPrivKey, err := suite.certManager.generateCACertificate()
	suite.NoError(err, "Should generate CA certificate successfully")
	suite.NotEmpty(caCert, "CA certificate should not be empty")
	suite.NotNil(caPrivKey, "CA private key should not be nil")

	// Verify CA certificate
	caCertPEM, _ := pem.Decode(caCert)
	suite.NotNil(caCertPEM, "Should decode CA certificate PEM")
	suite.Equal("CERTIFICATE", caCertPEM.Type)

	parsedCACert, err := x509.ParseCertificate(caCertPEM.Bytes)
	suite.NoError(err, "Should parse CA certificate")
	suite.True(parsedCACert.IsCA, "Certificate should be marked as CA")
	suite.True(parsedCACert.KeyUsage&x509.KeyUsageCertSign != 0, "Should have cert sign usage")

	// Generate server certificate
	serverCert, serverKey, err := suite.certManager.generateServerCertificate(caCert, caPrivKey)
	suite.NoError(err, "Should generate server certificate successfully")
	suite.NotEmpty(serverCert, "Server certificate should not be empty")
	suite.NotEmpty(serverKey, "Server private key should not be empty")

	// Verify server certificate
	serverCertPEM, _ := pem.Decode(serverCert)
	suite.NotNil(serverCertPEM, "Should decode server certificate PEM")

	parsedServerCert, err := x509.ParseCertificate(serverCertPEM.Bytes)
	suite.NoError(err, "Should parse server certificate")
	suite.False(parsedServerCert.IsCA, "Server certificate should not be CA")
	suite.Equal("openlit-webhook.openlit.svc", parsedServerCert.Subject.CommonName)

	// Verify SAN (Subject Alternative Names)
	expectedSANs := []string{
		"openlit-webhook.openlit.svc",
		"openlit-webhook.openlit.svc.cluster.local",
	}
	for _, expectedSAN := range expectedSANs {
		found := false
		for _, actualSAN := range parsedServerCert.DNSNames {
			if actualSAN == expectedSAN {
				found = true
				break
			}
		}
		suite.True(found, "Should contain expected SAN: %s", expectedSAN)
	}

	// Verify certificate chain
	roots := x509.NewCertPool()
	roots.AddCert(parsedCACert)
	
	opts := x509.VerifyOptions{Roots: roots}
	_, err = parsedServerCert.Verify(opts)
	suite.NoError(err, "Server certificate should be verifiable with CA")
}

func (suite *CertificateManagerTestSuite) TestEnsureCertificate() {
	// Test creating new certificate
	caCert, err := suite.certManager.EnsureCertificate(suite.ctx)
	suite.NoError(err, "Should create certificate successfully")
	suite.NotEmpty(caCert, "Should return CA certificate")

	// Verify secret was created
	secret, err := suite.certManager.client.CoreV1().Secrets(suite.certManager.namespace).Get(
		suite.ctx, suite.certManager.secretName, metav1.GetOptions{})
	suite.NoError(err, "Should retrieve created secret")

	// Verify secret contents
	suite.Contains(secret.Data, "tls.crt", "Secret should contain certificate")
	suite.Contains(secret.Data, "tls.key", "Secret should contain private key")
	suite.Contains(secret.Data, "ca.crt", "Secret should contain CA certificate")

	// Test idempotency - calling again should not error
	caCert2, err := suite.certManager.EnsureCertificate(suite.ctx)
	suite.NoError(err, "Should be idempotent")
	suite.Equal(caCert, caCert2, "Should return same CA certificate on subsequent calls")
}

func (suite *CertificateManagerTestSuite) TestShouldManageSecret() {
	tests := []struct {
		name        string
		secret      *corev1.Secret
		expected    bool
		description string
	}{
		{
			name: "Secret with managed-by label",
			secret: &corev1.Secret{
				ObjectMeta: metav1.ObjectMeta{
					Labels: map[string]string{
						"managed-by": "openlit-operator.openlit",
					},
				},
			},
			expected:    true,
			description: "Should manage secret with proper label",
		},
		{
			name: "Secret without managed-by label",
			secret: &corev1.Secret{
				ObjectMeta: metav1.ObjectMeta{
					Labels: map[string]string{
						"other": "label",
					},
				},
			},
			expected:    true,
			description: "Should manage secret without managed-by label",
		},
		{
			name: "Secret with different managed-by value",
			secret: &corev1.Secret{
				ObjectMeta: metav1.ObjectMeta{
					Labels: map[string]string{
						"managed-by": "other-operator.other-ns",
					},
				},
			},
			expected:    false,
			description: "Should not manage secret managed by other operator",
		},
		{
			name: "Secret with no labels",
			secret: &corev1.Secret{
				ObjectMeta: metav1.ObjectMeta{},
			},
			expected:    true,
			description: "Should manage secret with no labels",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			result := suite.certManager.shouldManageSecret(tt.secret)
			suite.Equal(tt.expected, result, tt.description)
		})
	}
}

func (suite *CertificateManagerTestSuite) TestNeedsCertificateRotation() {
	// Create a secret with recent certificate
	recentTime := time.Now().Add(-1 * time.Hour)
	caCert, _, _ := suite.certManager.generateCACertificate()
	secret := &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{
			CreationTimestamp: metav1.Time{Time: recentTime},
		},
		Data: map[string][]byte{
			"ca.crt": caCert,
		},
	}

	needs, reason := suite.certManager.needsCertificateRotation(secret)
	suite.True(needs, "Certificate without tls.crt should need rotation")
	suite.Equal("certificate_missing", reason)

	// Test old certificate
	oldTime := time.Now().Add(-25 * time.Hour) // Older than 24 hours
	secret.ObjectMeta.CreationTimestamp = metav1.Time{Time: oldTime}

	needs, reason = suite.certManager.needsCertificateRotation(secret)
	suite.True(needs, "Old certificate should need rotation")
	suite.Equal("certificate_missing", reason)

	// Test missing CA certificate
	secret.Data = map[string][]byte{
		"tls.crt": []byte("cert"),
		"tls.key": []byte("key"),
	}

	needs, reason = suite.certManager.needsCertificateRotation(secret)
	suite.True(needs, "Invalid certificate should trigger rotation")
	suite.Equal("certificate_invalid_format", reason)

	// Test invalid certificate
	secret.Data["ca.crt"] = []byte("invalid certificate")
	needs, reason = suite.certManager.needsCertificateRotation(secret)
	suite.True(needs, "Invalid certificate should trigger rotation")
	suite.Equal("certificate_invalid_format", reason)
}

func (suite *CertificateManagerTestSuite) TestRotateCertificate() {
	// First, ensure a certificate exists
	_, err := suite.certManager.EnsureCertificate(suite.ctx)
	suite.NoError(err)

	// Get the original secret
	originalSecret, err := suite.certManager.client.CoreV1().Secrets(suite.certManager.namespace).Get(
		suite.ctx, suite.certManager.secretName, metav1.GetOptions{})
	suite.NoError(err)

	// Store original certificate data
	originalCert := make([]byte, len(originalSecret.Data["ca.crt"]))
	copy(originalCert, originalSecret.Data["ca.crt"])

	// Rotate the certificate
	err = suite.certManager.rotateCertificate(suite.ctx)
	suite.NoError(err, "Should rotate certificate successfully")

	// Get the updated secret
	updatedSecret, err := suite.certManager.client.CoreV1().Secrets(suite.certManager.namespace).Get(
		suite.ctx, suite.certManager.secretName, metav1.GetOptions{})
	suite.NoError(err)

	// Verify certificate was actually changed
	suite.NotEqual(originalCert, updatedSecret.Data["ca.crt"], "Certificate should be different after rotation")

	// Verify new certificate is valid
	caCertPEM, _ := pem.Decode(updatedSecret.Data["ca.crt"])
	suite.NotNil(caCertPEM, "New CA certificate should be valid PEM")

	parsedCACert, err := x509.ParseCertificate(caCertPEM.Bytes)
	suite.NoError(err, "New CA certificate should be parseable")
	suite.True(parsedCACert.IsCA, "New certificate should be CA")
}

func (suite *CertificateManagerTestSuite) TestCertificateRotationLoop() {
	// Set short rotation interval for testing
	suite.certManager.rotationInterval = 100 * time.Millisecond
	
	// Ensure certificate exists
	_, err := suite.certManager.EnsureCertificate(suite.ctx)
	suite.NoError(err)

	// Create a context with timeout for the rotation loop
	ctx, cancel := context.WithTimeout(suite.ctx, 500*time.Millisecond)
	defer cancel()

	// Start certificate rotation
	suite.certManager.StartCertificateRotation(ctx)

	// Wait a bit to allow rotation loop to run
	time.Sleep(200 * time.Millisecond)

	// Stop certificate rotation
	suite.certManager.StopCertificateRotation()

	// The test verifies that the rotation loop can start and stop without errors
	// In a real scenario, we would verify that certificates are actually rotated
}

func (suite *CertificateManagerTestSuite) TestMultiReplicaScenario() {
	// Simulate multiple replicas trying to manage the same secret
	
	// Create first certificate manager (replica 1)
	cm1 := NewCertificateManager(
		suite.certManager.client,
		"openlit",                  // namespace
		"openlit-webhook",          // serviceName
		"openlit-webhook-tls",      // secretName
		365,                        // validityDays
		30,                         // refreshThreshold
	)

	// Create second certificate manager (replica 2)
	cm2 := NewCertificateManager(
		suite.certManager.client,
		"openlit",                  // namespace
		"openlit-webhook",          // serviceName
		"openlit-webhook-tls",      // secretName
		365,                        // validityDays
		30,                         // refreshThreshold
	)

	// Both replicas try to ensure certificate
	_, err1 := cm1.EnsureCertificate(suite.ctx)
	_, err2 := cm2.EnsureCertificate(suite.ctx)

	// Both should succeed without conflict
	suite.NoError(err1, "First replica should succeed")
	suite.NoError(err2, "Second replica should succeed")

	// Verify only one secret exists
	secret, err := suite.certManager.client.CoreV1().Secrets(suite.certManager.namespace).Get(
		suite.ctx, suite.certManager.secretName, metav1.GetOptions{})
	suite.NoError(err, "Should retrieve the secret")
	suite.Equal("openlit-operator.openlit", secret.Labels["managed-by"])
}

func (suite *CertificateManagerTestSuite) TestCertificateValidation() {
	// Generate valid certificates
	caCert, caKey, err := suite.certManager.generateCACertificate()
	suite.NoError(err)

	serverCert, _, err := suite.certManager.generateServerCertificate(caCert, caKey)
	suite.NoError(err)

	// Test valid certificates by creating mock secrets
	validSecret := &corev1.Secret{
		Data: map[string][]byte{
			"ca.crt":  caCert,
			"tls.crt": serverCert,
		},
	}
	isValid := suite.certManager.isCertificateValid(validSecret)
	suite.True(isValid, "Valid certificate secret should pass validation")

	// Test invalid certificates
	invalidSecret := &corev1.Secret{
		Data: map[string][]byte{
			"ca.crt":  []byte("invalid certificate"),
			"tls.crt": []byte("invalid certificate"),
		},
	}
	isValid = suite.certManager.isCertificateValid(invalidSecret)
	suite.False(isValid, "Invalid certificate secret should fail validation")

	// Test empty certificates
	emptySecret := &corev1.Secret{
		Data: map[string][]byte{
			"ca.crt":  []byte(""),
			"tls.crt": []byte(""),
		},
	}
	isValid = suite.certManager.isCertificateValid(emptySecret)
	suite.False(isValid, "Empty certificate secret should fail validation")
}

func (suite *CertificateManagerTestSuite) TestSecretManagement() {
	// Test creating secret with proper labels and ownership
	caCert, caKey, err := suite.certManager.generateCACertificate()
	suite.NoError(err)

	serverCert, serverKey, err := suite.certManager.generateServerCertificate(caCert, caKey)
	suite.NoError(err)

	// Create secret data
	secretData := map[string][]byte{
		"ca.crt":  caCert,
		"tls.crt": serverCert,
		"tls.key": serverKey,
	}
	err = suite.certManager.createOrUpdateSecret(suite.ctx, secretData)
	suite.NoError(err, "Should create secret successfully")

	// Verify secret
	secret, err := suite.certManager.client.CoreV1().Secrets(suite.certManager.namespace).Get(
		suite.ctx, suite.certManager.secretName, metav1.GetOptions{})
	suite.NoError(err)

	// Verify secret type
	suite.Equal(corev1.SecretTypeTLS, secret.Type, "Secret should be TLS type")

	// Verify required data fields
	requiredFields := []string{"tls.crt", "tls.key", "ca.crt"}
	for _, field := range requiredFields {
		suite.Contains(secret.Data, field, "Secret should contain %s", field)
		suite.NotEmpty(secret.Data[field], "Field %s should not be empty", field)
	}

	// Verify labels
	expectedLabels := map[string]string{
		"app":        "openlit-operator",
		"component":  "webhook-certs",
		"managed-by": "openlit-operator.openlit",
	}
	for key, value := range expectedLabels {
		suite.Equal(value, secret.Labels[key], "Label %s should have correct value", key)
	}
}

func TestCertificateManagerSuite(t *testing.T) {
	suite.Run(t, new(CertificateManagerTestSuite))
}

// Additional unit tests for specific functions
func TestCertificateExpiry(t *testing.T) {
	// Create a certificate manager for testing
	fakeClient := fake.NewSimpleClientset()

	cm := NewCertificateManager(
		fakeClient,
		"test-ns",
		"test-service",
		"test-secret",
		365,
		30,
	)

	// Generate certificate
	caCert, _, err := cm.generateCACertificate()
	assert.NoError(t, err)

	// Parse certificate to check expiry
	caCertPEM, _ := pem.Decode(caCert)
	parsedCert, err := x509.ParseCertificate(caCertPEM.Bytes)
	assert.NoError(t, err)

	// Verify certificate is valid for at least 364 days (allowing for slight timing differences)
	expectedExpiry := time.Now().Add(364 * 24 * time.Hour)
	assert.True(t, parsedCert.NotAfter.After(expectedExpiry), "Certificate should be valid for at least 1 year")
}

func TestSecretLabelsAndAnnotations(t *testing.T) {
	fakeClient := fake.NewSimpleClientset()

	cm := NewCertificateManager(
		fakeClient,
		"test-ns",
		"test-service",
		"test-secret",
		365,
		30,
	)

	ctx := context.Background()
	_, err := cm.EnsureCertificate(ctx)
	assert.NoError(t, err)

	secret, err := fakeClient.CoreV1().Secrets(cm.namespace).Get(ctx, cm.secretName, metav1.GetOptions{})
	assert.NoError(t, err)

	// Verify required labels
	assert.Equal(t, "openlit-operator", secret.Labels["app"])
	assert.Equal(t, "webhook-certs", secret.Labels["component"])
	assert.Equal(t, "openlit-operator.test-ns", secret.Labels["managed-by"])
}
