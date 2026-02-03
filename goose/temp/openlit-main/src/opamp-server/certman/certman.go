package certman

import (
	"bytes"
	"crypto/rand"
	"crypto/rsa"
	"crypto/tls"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/pem"
	"errors"
	"fmt"
	"io/ioutil"
	"log"
	"math/big"
	"net"
	"os"
	"sync"
	"time"

	"opamp-server/constants"

	"github.com/open-telemetry/opamp-go/protobufs"
)

var logger = log.New(log.Default().Writer(), "[CertMan] ", log.Default().Flags()|log.Lmsgprefix|log.Lmicroseconds)

var (
	caCert      *x509.Certificate
	caPrivKey   *rsa.PrivateKey
	caCertBytes []byte
)

var loadCACertOnce sync.Once

func loadCACert() {

	// Load CA certificate.
	var err error
	caCertBytes, err = ioutil.ReadFile(constants.CaCertPath)
	if err != nil {
		logger.Fatalf("Cannot read CA cert: %v", err)
	}

	caKeyBytes, err := ioutil.ReadFile(constants.PrivateKeyPath)
	if err != nil {
		logger.Fatalf("Cannot read CA key: %v", err)
	}

	// Convert from DER to PEM format.
	caCertPB, _ := pem.Decode(caCertBytes)
	caKeyPB, _ := pem.Decode(caKeyBytes)

	caCert, err = x509.ParseCertificate(caCertPB.Bytes)
	if err != nil {
		logger.Fatalf("Cannot parse CA certificate: %v", err)
	}

	caPrivKey, err = x509.ParsePKCS1PrivateKey(caKeyPB.Bytes)
	if err != nil {
		logger.Fatalf("Cannot parse CA key: %v", err)
	}
}

func createClientTLSCertTemplate() *x509.Certificate {
	return &x509.Certificate{
		SerialNumber: big.NewInt(1),
		IPAddresses:  []net.IP{net.IPv4(127, 0, 0, 1)},
		NotBefore:    time.Now(),
		NotAfter:     time.Now().Add(time.Hour * 1000),
		ExtKeyUsage:  []x509.ExtKeyUsage{x509.ExtKeyUsageClientAuth, x509.ExtKeyUsageServerAuth},
		KeyUsage:     x509.KeyUsageDigitalSignature,
	}
}

func CreateClientTLSCertFromCSR(csr *x509.CertificateRequest) (*protobufs.TLSCertificate, error) {
	loadCACertOnce.Do(loadCACert)

	template := createClientTLSCertTemplate()

	// Use the Subject from CSR.
	template.Subject = csr.Subject

	// Create the client cert and sign it using CA cert.
	certBytes, err := x509.CreateCertificate(rand.Reader, template, caCert, csr.PublicKey, caPrivKey)
	if err != nil {
		err := fmt.Errorf("cannot create certificate: %v", err)
		return nil, err
	}

	// Convert from DER to PEM format.
	certPEM := new(bytes.Buffer)
	pem.Encode(
		certPEM, &pem.Block{
			Type:  "CERTIFICATE",
			Bytes: certBytes,
		},
	)

	// We have a client certificate with a public and private key.
	certificate := &protobufs.TLSCertificate{
		Cert:   certPEM.Bytes(),
		CaCert: caCertBytes,
	}

	return certificate, nil
}

func CreateClientTLSCert() (*protobufs.TLSCertificate, error) {
	loadCACertOnce.Do(loadCACert)

	// Generate a keypair for new client cert.
	clientCertKeyPair, err := rsa.GenerateKey(rand.Reader, 4096)
	if err != nil {
		err := fmt.Errorf("cannot generate keypair: %v", err)
		return nil, err
	}

	// Prepare certificate template.
	template := createClientTLSCertTemplate()
	template.Subject = pkix.Name{
		CommonName:   "OpAMP Example Client",
		Organization: []string{"OpenTelemetry OpAMP Workgroup"},
		Locality:     []string{"Server-initiated"},
	}

	// Create the client cert. Sign it using CA cert.
	certDER, err := x509.CreateCertificate(rand.Reader, template, caCert, &clientCertKeyPair.PublicKey, caPrivKey)
	if err != nil {
		err := fmt.Errorf("cannot create certificate: %v", err)
		return nil, err
	}

	certPEM := new(bytes.Buffer)
	pem.Encode(
		certPEM, &pem.Block{
			Type:  "CERTIFICATE",
			Bytes: certDER,
		},
	)

	privateKeyPEM := new(bytes.Buffer)
	pem.Encode(
		privateKeyPEM, &pem.Block{
			Type:  "RSA PRIVATE KEY",
			Bytes: x509.MarshalPKCS1PrivateKey(clientCertKeyPair),
		},
	)

	// We have a client certificate with a public and private key.
	certificate := &protobufs.TLSCertificate{
		Cert:       certPEM.Bytes(),
		PrivateKey: privateKeyPEM.Bytes(),
		CaCert:     caCertBytes,
	}

	return certificate, nil
}

// TLSConfigOptions holds options for TLS configuration
type TLSConfigOptions struct {
	InsecureSkipVerify bool
	RequireClientCert  bool
	MinTLSVersion      uint16
	MaxTLSVersion      uint16
}

// CreateServerTLSConfig creates a TLS configuration for the server with enhanced security options
func CreateServerTLSConfig(caCertPath, serverCertPath, serverKeyPath string) (*tls.Config, error) {
	return CreateServerTLSConfigWithOptions(caCertPath, serverCertPath, serverKeyPath, TLSConfigOptions{
		InsecureSkipVerify: false,
		RequireClientCert:  false,
		MinTLSVersion:      tls.VersionTLS12,
		MaxTLSVersion:      tls.VersionTLS13,
	})
}

// CreateServerTLSConfigWithOptions creates a TLS configuration with custom options
func CreateServerTLSConfigWithOptions(caCertPath, serverCertPath, serverKeyPath string, options TLSConfigOptions) (*tls.Config, error) {
	// Validate certificate files first
	validator := NewCertificateValidator()

	// Check if certificate files exist
	if _, err := os.Stat(caCertPath); os.IsNotExist(err) {
		return nil, fmt.Errorf("CA certificate file not found: %s", caCertPath)
	}
	if _, err := os.Stat(serverCertPath); os.IsNotExist(err) {
		return nil, fmt.Errorf("server certificate file not found: %s", serverCertPath)
	}
	if _, err := os.Stat(serverKeyPath); os.IsNotExist(err) {
		return nil, fmt.Errorf("server private key file not found: %s", serverKeyPath)
	}

	// Validate certificate chain
	if err := validator.VerifyCertificateChain(serverCertPath, caCertPath); err != nil {
		logger.Printf("WARNING: Server certificate chain validation failed: %v", err)
	}

	// Check certificate expiry
	if err := validator.CheckCertificateExpiry(serverCertPath, 30); err != nil {
		logger.Printf("WARNING: %v", err)
	}

	// Read the CA's public key
	caCertBytes, err := os.ReadFile(caCertPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read CA certificate: %v", err)
	}

	// Create a certificate pool and make our CA trusted
	caCertPool := x509.NewCertPool()
	if ok := caCertPool.AppendCertsFromPEM(caCertBytes); !ok {
		return nil, errors.New("failed to append CA certificate to pool")
	}

	// Load server's certificate
	cert, err := tls.LoadX509KeyPair(serverCertPath, serverKeyPath)
	if err != nil {
		return nil, fmt.Errorf("failed to load server certificate/key pair: %v", err)
	}

	// Determine client authentication mode
	clientAuth := tls.NoClientCert
	if options.RequireClientCert {
		clientAuth = tls.RequireAndVerifyClientCert
	} else {
		clientAuth = tls.VerifyClientCertIfGiven
	}

	tlsConfig := &tls.Config{
		Certificates: []tls.Certificate{cert},
		ClientAuth:   clientAuth,
		ClientCAs:    caCertPool,
		MinVersion:   options.MinTLSVersion,
		MaxVersion:   options.MaxTLSVersion,

		// Security settings
		InsecureSkipVerify: options.InsecureSkipVerify,

		// Prefer server cipher suites for better security
		PreferServerCipherSuites: true,

		// Use only secure cipher suites
		CipherSuites: []uint16{
			tls.TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
			tls.TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305,
			tls.TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
			tls.TLS_RSA_WITH_AES_256_GCM_SHA384,
			tls.TLS_RSA_WITH_AES_128_GCM_SHA256,
		},

		// Curve preferences
		CurvePreferences: []tls.CurveID{
			tls.X25519,
			tls.CurveP256,
			tls.CurveP384,
		},
	}

	logger.Printf("TLS configuration created successfully")
	logger.Printf("  Min TLS Version: %s", getTLSVersionString(options.MinTLSVersion))
	logger.Printf("  Max TLS Version: %s", getTLSVersionString(options.MaxTLSVersion))
	logger.Printf("  Client Auth: %s", getClientAuthString(clientAuth))
	logger.Printf("  Insecure Skip Verify: %t", options.InsecureSkipVerify)

	return tlsConfig, nil
}

// Helper function to get TLS version string
func getTLSVersionString(version uint16) string {
	switch version {
	case tls.VersionTLS10:
		return "TLS 1.0"
	case tls.VersionTLS11:
		return "TLS 1.1"
	case tls.VersionTLS12:
		return "TLS 1.2"
	case tls.VersionTLS13:
		return "TLS 1.3"
	default:
		return fmt.Sprintf("Unknown (%d)", version)
	}
}

// Helper function to get client auth string
func getClientAuthString(auth tls.ClientAuthType) string {
	switch auth {
	case tls.NoClientCert:
		return "No Client Certificate"
	case tls.RequestClientCert:
		return "Request Client Certificate"
	case tls.RequireAnyClientCert:
		return "Require Any Client Certificate"
	case tls.VerifyClientCertIfGiven:
		return "Verify Client Certificate If Given"
	case tls.RequireAndVerifyClientCert:
		return "Require and Verify Client Certificate"
	default:
		return fmt.Sprintf("Unknown (%d)", auth)
	}
}

func CreateTLSCert(caCertPath, caKeyPath string) (*protobufs.TLSCertificate, error) {
	// Load CA Cert.
	caCertBytes, err := os.ReadFile(caCertPath)
	if err != nil {
		return nil, fmt.Errorf("cannot read CA cert: %v", err)
	}

	caKeyBytes, err := os.ReadFile(caKeyPath)
	if err != nil {
		return nil, fmt.Errorf("cannot read CA key: %v", err)
	}

	caCertPB, _ := pem.Decode(caCertBytes)
	caKeyPB, _ := pem.Decode(caKeyBytes)
	caCert, err := x509.ParseCertificate(caCertPB.Bytes)
	if err != nil {
		return nil, fmt.Errorf("cannot parse CA cert: %v", err)
	}

	caPrivKey, err := x509.ParsePKCS1PrivateKey(caKeyPB.Bytes)
	if err != nil {
		return nil, fmt.Errorf("cannot parse CA key: %v", err)
	}

	// Generate a private key for new client cert.
	certPrivKey, err := rsa.GenerateKey(rand.Reader, 4096)
	if err != nil {
		err := fmt.Errorf("cannot generate private key: %v", err)
		return nil, err
	}

	// Prepare certificate template.
	template := &x509.Certificate{
		SerialNumber: big.NewInt(1),
		Subject: pkix.Name{
			CommonName:    "OpAMP Example Client",
			Organization:  []string{"OpAMP Example"},
			Country:       []string{"CA"},
			Province:      []string{"ON"},
			Locality:      []string{"City"},
			StreetAddress: []string{""},
			PostalCode:    []string{""},
		},
		IPAddresses: []net.IP{net.IPv4(127, 0, 0, 1)},
		NotBefore:   time.Now(),
		NotAfter:    time.Now().Add(time.Hour * 1000),
		ExtKeyUsage: []x509.ExtKeyUsage{x509.ExtKeyUsageClientAuth, x509.ExtKeyUsageServerAuth},
		KeyUsage:    x509.KeyUsageDigitalSignature,
	}

	// Create the client cert. Sign it using CA cert.
	certBytes, err := x509.CreateCertificate(rand.Reader, template, caCert, &certPrivKey.PublicKey, caPrivKey)
	if err != nil {
		err := fmt.Errorf("cannot create certificate: %v", err)
		return nil, err
	}

	publicKeyPEM := new(bytes.Buffer)
	pem.Encode(publicKeyPEM, &pem.Block{
		Type:  "CERTIFICATE",
		Bytes: certBytes,
	})

	privateKeyPEM := new(bytes.Buffer)
	pem.Encode(privateKeyPEM, &pem.Block{
		Type:  "RSA PRIVATE KEY",
		Bytes: x509.MarshalPKCS1PrivateKey(certPrivKey),
	})

	// We have a client certificate with a public and private key.
	certificate := &protobufs.TLSCertificate{
		Cert:       publicKeyPEM.Bytes(),
		PrivateKey: privateKeyPEM.Bytes(),
		CaCert:     caCertBytes,
	}

	return certificate, nil
}
