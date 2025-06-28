---
title: Using Mutual TLS with OpenAI Provider
description: Learn how to configure mutual TLS authentication for secure enterprise connections with the OpenAI provider
---

# Using Mutual TLS with OpenAI Provider

This guide explains how to configure mutual TLS (mTLS) authentication when using Goose with the OpenAI provider. Mutual TLS provides enhanced security by requiring both the client and server to authenticate each other using X.509 certificates, making it ideal for enterprise environments.

## Overview

Mutual TLS authentication ensures:
- **Client Authentication**: Your Goose instance authenticates to the OpenAI API using a client certificate
- **Server Verification**: The OpenAI server's identity is verified using a custom Certificate Authority (CA)
- **Enhanced Security**: All communications are encrypted and both parties are authenticated

## Prerequisites

Before configuring mTLS, you'll need:
- Client certificate and private key (in PEM format)
- CA certificate for server verification (optional, if using custom CA)
- Proper file permissions on certificate files

## Configuration

Configure mTLS for the OpenAI provider using environment variables:

### Basic Configuration

```bash
# Required OpenAI configuration
export OPENAI_API_KEY="your-api-key"

# Optional: Custom OpenAI endpoint (defaults to https://api.openai.com)
export OPENAI_HOST="https://your-enterprise-openai.com"

# mTLS Certificate Configuration
export OPENAI_CLIENT_CERT_PATH="/path/to/client.crt"
export OPENAI_CLIENT_KEY_PATH="/path/to/client.key"
export OPENAI_CA_CERT_PATH="/path/to/ca.crt"
```

### Configuration Options

| Environment Variable | Required | Description |
|---------------------|----------|-------------|
| `OPENAI_CLIENT_CERT_PATH` | No | Path to client certificate file (PEM format) |
| `OPENAI_CLIENT_KEY_PATH` | No | Path to client private key file (PEM format) |
| `OPENAI_CA_CERT_PATH` | No | Path to CA certificate file (PEM format) |

:::note
All mTLS configuration is optional. If not provided, standard TLS will be used.
:::

## Use Cases

### Enterprise OpenAI Deployment

For organizations with enterprise OpenAI deployments requiring client certificates:

```bash
export OPENAI_API_KEY="enterprise-api-key"
export OPENAI_HOST="https://openai.yourcompany.com"
export OPENAI_CLIENT_CERT_PATH="/etc/ssl/certs/goose-client.crt"
export OPENAI_CLIENT_KEY_PATH="/etc/ssl/private/goose-client.key"
export OPENAI_CA_CERT_PATH="/etc/ssl/certs/company-ca.crt"
```

### OpenAI-Compatible API with Custom CA

For OpenAI-compatible APIs (like Azure OpenAI) with custom certificate authorities:

```bash
export OPENAI_API_KEY="your-api-key"
export OPENAI_HOST="https://your-azure-openai.openai.azure.com"
export OPENAI_CA_CERT_PATH="/path/to/azure-ca.crt"
```

### Development Environment

For development with self-signed certificates:

```bash
export OPENAI_API_KEY="dev-api-key"
export OPENAI_HOST="https://dev-openai.internal"
export OPENAI_CLIENT_CERT_PATH="./certs/dev-client.crt"
export OPENAI_CLIENT_KEY_PATH="./certs/dev-client.key"
export OPENAI_CA_CERT_PATH="./certs/dev-ca.crt"
```

## Certificate Management

### Certificate Requirements

- **Format**: All certificates must be in PEM format
- **Client Certificate**: Must include the full certificate chain if intermediate CAs are used
- **Private Key**: Must correspond to the client certificate
- **CA Certificate**: Must be the root or intermediate CA that signed the server's certificate

### File Permissions

Ensure proper security by setting appropriate file permissions:

```bash
# Set restrictive permissions on private key
chmod 600 /path/to/client.key

# Set read permissions for certificates
chmod 644 /path/to/client.crt
chmod 644 /path/to/ca.crt
```

### Verifying Certificates

Before using certificates with Goose, verify they are properly formatted:

```bash
# Verify certificate format
openssl x509 -in /path/to/client.crt -text -noout

# Verify private key format
openssl rsa -in /path/to/client.key -check

# Verify certificate and key match
cert_modulus=$(openssl x509 -noout -modulus -in /path/to/client.crt | openssl md5)
key_modulus=$(openssl rsa -noout -modulus -in /path/to/client.key | openssl md5)
echo "Certificate: $cert_modulus"
echo "Key: $key_modulus"
# These should match
```

## Testing Configuration

### Basic Connection Test

Test your mTLS configuration by running a simple Goose command:

```bash
goose session -n test-mtls
```

### Debug Mode

Enable debug logging to see certificate loading and TLS handshake details:

```bash
export RUST_LOG=debug
goose session -n test-mtls
```

Look for log messages indicating successful certificate loading and TLS handshake completion.

## Troubleshooting

### Common Issues

**Permission Denied**
```
Error: Failed to read client certificate: Permission denied
```
- Solution: Check file permissions and ensure the process can read the certificate files

**Invalid Certificate Format**
```
Error: Failed to parse client certificate: Invalid PEM format
```
- Solution: Verify certificates are in PEM format, not DER or other formats

**Certificate/Key Mismatch**
```
Error: Failed to create client identity: Certificate and key do not match
```
- Solution: Verify the certificate and private key are a matching pair

**CA Trust Issues**
```
Error: TLS handshake failed: Certificate verification failed
```
- Solution: Verify the CA certificate is correct and the server's certificate is signed by this CA

### Debug Information

For detailed troubleshooting, enable debug logging:

```bash
export RUST_LOG=reqwest=debug,rustls=debug
goose session -n test-mtls
```

This provides detailed TLS handshake information and certificate validation details.

## Security Best Practices

### Certificate Storage
- Store certificates in secure locations with appropriate permissions
- Use dedicated certificate directories (e.g., `/etc/ssl/certs/`, `/etc/ssl/private/`)
- Never commit certificates to version control

### Key Management
- Use strong private keys (RSA 2048+ or ECC P-256+)
- Regularly rotate certificates before expiration
- Consider using hardware security modules (HSMs) for key storage

### Environment Variables
- Use secure methods to set environment variables
- Consider using tools like `direnv` for project-specific configuration
- In production, use secret management systems instead of plain environment variables

### Certificate Lifecycle
- Monitor certificate expiration dates
- Implement automated certificate renewal where possible
- Test certificate updates in staging environments first

## Alternative Configuration Methods

### Using Goose Configuration Files

Instead of environment variables, you can use Goose's configuration system:

```toml
# ~/.config/goose/config.toml
[providers.openai]
api_key = "your-api-key"
host = "https://your-enterprise-openai.com"
client_cert_path = "/path/to/client.crt"
client_key_path = "/path/to/client.key"
ca_cert_path = "/path/to/ca.crt"
```

### Docker Environment

When running Goose in Docker, mount certificate volumes:

```bash
docker run -v /path/to/certs:/etc/ssl/certs:ro \
  -e OPENAI_API_KEY=your-key \
  -e OPENAI_CLIENT_CERT_PATH=/etc/ssl/certs/client.crt \
  -e OPENAI_CLIENT_KEY_PATH=/etc/ssl/certs/client.key \
  -e OPENAI_CA_CERT_PATH=/etc/ssl/certs/ca.crt \
  goose:latest
```

## Support

If you encounter issues with mTLS configuration:

1. Verify your certificates are properly formatted and accessible
2. Check the debug logs for detailed error information
3. Ensure your OpenAI endpoint supports and requires client certificates
4. Consult your organization's security team for certificate-related questions

For additional help, refer to the [troubleshooting guide](../troubleshooting.md) or open an issue on the Goose GitHub repository.
