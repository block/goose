# OpenLIT OpAMP Server - TLS Certificate Management

This document explains how to set up and manage TLS certificates for secure OpAMP communication between the OpenLIT server and OpAMP supervisors.

## Overview

The OpenLIT OpAMP server uses TLS certificates to secure communication with OpAMP supervisors. This implementation provides:

- **Self-signed CA** for development and testing
- **Production-ready certificate management** with proper validation
- **Environment-aware configuration** (development vs production)
- **Mutual TLS support** for enhanced security
- **Certificate validation and monitoring** utilities

## Quick Start

### 1. Generate Certificates

```bash
cd src/opamp-server/certs
./generate.sh
```

### 2. Configure Environment

**For Development:**
```bash
export OPAMP_ENVIRONMENT=development
export OPAMP_CERTS_DIR=/path/to/certs
```

**For Production:**
```bash
export OPAMP_ENVIRONMENT=production
export OPAMP_CERTS_DIR=/path/to/certs
export OPAMP_TLS_INSECURE_SKIP_VERIFY=false
```

### 3. Start the Server

```bash
cd src/opamp-server
./opamp-server
```

### 4. Configure Supervisors

**For Development:**
```bash
./setup-supervisor.sh --development
```

**For Production:**
```bash
./setup-supervisor.sh --production --server-url wss://your-server:4320/v1/opamp
```

## Certificate Structure

```
certs/
├── cert/
│   └── ca.cert.pem          # CA certificate (public)
├── private/
│   └── ca.key.pem           # CA private key (keep secure!)
├── server/
│   ├── server.cert.pem      # Server certificate
│   ├── server.key.pem       # Server private key
│   └── server.csr           # Server certificate signing request
├── client/
│   ├── client.cert.pem      # Client certificate (optional)
│   ├── client.key.pem       # Client private key (optional)
│   └── client.csr           # Client certificate signing request
└── generate.sh              # Certificate generation script
```

## Environment Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OPAMP_ENVIRONMENT` | `production` | Environment mode (`development`, `production`, `testing`) |
| `OPAMP_CERTS_DIR` | `/app/opamp/certs` | Certificate directory path |
| `OPAMP_TLS_INSECURE_SKIP_VERIFY` | `false` | Skip certificate verification (dev only) |
| `OPAMP_TLS_REQUIRE_CLIENT_CERT` | `true` | Require client certificates for mutual TLS |
| `OPAMP_TLS_MIN_VERSION` | `1.2` | Minimum TLS version |
| `OPAMP_TLS_MAX_VERSION` | `1.3` | Maximum TLS version |

### Environment Modes

#### Development Mode
- `insecure_skip_verify: true` by default
- Client certificates not required
- Relaxed certificate validation
- Suitable for local development and testing

#### Production Mode (Default)
- `insecure_skip_verify: false` (strict verification)
- Client certificates required for mutual TLS authentication
- Full certificate chain validation
- Certificate expiry monitoring
- Secure cipher suites only

#### Testing Mode
- Similar to development mode
- Optimized for automated testing
- Flexible certificate validation

## Certificate Generation

### Basic Generation

```bash
cd certs/
./generate.sh
```

### Advanced Options

```bash
# Clean existing certificates and regenerate
./generate.sh --clean

# Set custom certificate validity periods
CERT_VALIDITY_DAYS=7300 SERVER_CERT_DAYS=730 ./generate.sh

# Generate with custom configuration
CERT_VALIDITY_DAYS=3650 ./generate.sh
```

### Certificate Properties

- **Key Size**: 4096-bit RSA keys for enhanced security
- **Hash Algorithm**: SHA-256
- **CA Validity**: 10 years (configurable)
- **Server/Client Validity**: 1 year (configurable)
- **Extensions**: Proper key usage and extended key usage
- **Subject Alternative Names**: Comprehensive SAN list for flexibility

### Subject Alternative Names (SANs)

The server certificate includes SANs for various deployment scenarios:

- `localhost` - Local development
- `127.0.0.1`, `0.0.0.0` - Local IP addresses
- `172.17.0.1` - Docker default bridge
- `host.docker.internal` - Docker host access

For production, customize the SANs in `server_ext.conf`:

```conf
[alt_names]
DNS.1 = your-domain.com
DNS.2 = opamp.your-domain.com
IP.1 = 192.168.1.100
```

## Supervisor Configuration

### Development Setup

```yaml
server:
  endpoint: wss://localhost:4320/v1/opamp
  tls:
    insecure_skip_verify: true
```

### Production Setup

```yaml
server:
  endpoint: wss://your-server:4320/v1/opamp
  tls:
    insecure_skip_verify: false
    ca_file: /app/opamp/certs/cert/ca.cert.pem
    # Required for mutual TLS (default in production):
    cert_file: /app/opamp/certs/client/client.cert.pem
    key_file: /app/opamp/certs/client/client.key.pem
```

### Automated Setup

Use the provided setup script:

```bash
# Development configuration
./setup-supervisor.sh --development

# Production configuration
./setup-supervisor.sh --production \
  --server-url wss://my-server:4320/v1/opamp \
  --output-dir /etc/opamp
```

## Security Best Practices

### Certificate Management

1. **Secure Storage**: Store private keys with restricted permissions (600)
2. **Regular Rotation**: Rotate certificates before expiration
3. **CA Protection**: Keep CA private key highly secure
4. **Distribution**: Securely distribute CA certificates to clients

### Production Deployment

1. **Use Real Domains**: Configure proper DNS names in certificates
2. **Enable Mutual TLS**: Use client certificates for authentication
3. **Monitor Expiry**: Set up alerts for certificate expiration
4. **Secure Transport**: Always use TLS in production

### File Permissions

```bash
# Set proper permissions after certificate generation
chmod 600 private/*.pem client/*.key.pem server/*.key.pem
chmod 644 cert/*.pem client/*.cert.pem server/*.cert.pem
chmod 700 private/
```

## Troubleshooting

### Common Issues

#### "certificate signed by unknown authority"

**Cause**: Client doesn't trust the self-signed CA certificate.

**Solutions**:
1. **Development**: Set `insecure_skip_verify: true` in supervisor config
2. **Production**: Provide CA certificate path in supervisor config
3. **Verify**: Ensure CA certificate is accessible and readable

#### "tls: certificate required" / "client didn't provide a certificate"

**Cause**: Server requires client certificates for mutual TLS, but client configuration is missing certificate files.

**Solutions**:
1. **Check client certificate files exist**:
   ```bash
   ls -la /app/opamp/certs/client/client.cert.pem
   ls -la /app/opamp/certs/client/client.key.pem
   ```
2. **Regenerate certificates if missing**:
   ```bash
   cd /app/opamp/certs && ./generate.sh --clean
   ```
3. **Update supervisor configuration**:
   ```yaml
   server:
     tls:
       cert_file: /app/opamp/certs/client/client.cert.pem
       key_file: /app/opamp/certs/client/client.key.pem
   ```
4. **Disable mutual TLS for development**:
   ```bash
   export OPAMP_TLS_REQUIRE_CLIENT_CERT=false
   ```

#### "certificate has expired"

**Cause**: Certificate validity period has passed.

**Solutions**:
1. Regenerate certificates: `cd certs && ./generate.sh --clean`
2. Check expiry: `openssl x509 -in cert/ca.cert.pem -noout -dates`
3. Set longer validity: `CERT_VALIDITY_DAYS=7300 ./generate.sh`

#### "certificate is valid for localhost, not your-server"

**Cause**: Certificate doesn't include the server's hostname/IP in SANs.

**Solutions**:
1. Update `server_ext.conf` with correct DNS names/IPs
2. Regenerate server certificate
3. Use IP address or localhost for development

#### Connection refused or timeout

**Cause**: Network connectivity or server configuration issues.

**Solutions**:
1. Verify server is running: `netstat -tlnp | grep 4320`
2. Check firewall rules for port 4320
3. Verify WebSocket support in network infrastructure

#### "Can't load /root/.rnd into RNG"

**Cause**: OpenSSL trying to access a random number generator file that doesn't exist.

**Solutions**:
1. **Fixed automatically**: This issue has been resolved in the current version
2. **Manual fix**: Remove or comment out `RANDFILE` in `openssl.conf`
3. **Verify fix**: Run `./generate.sh` and check for the error

### Validation Tools

#### Certificate Information

```bash
# View certificate details
openssl x509 -in cert/ca.cert.pem -noout -text

# Check certificate expiry
openssl x509 -in server/server.cert.pem -noout -dates

# Verify certificate chain
openssl verify -CAfile cert/ca.cert.pem server/server.cert.pem
```

#### Test TLS Connection

```bash
# Test TLS handshake
openssl s_client -connect localhost:4320 -CAfile cert/ca.cert.pem

# Test with client certificate
openssl s_client -connect localhost:4320 \
  -CAfile cert/ca.cert.pem \
  -cert client/client.cert.pem \
  -key client/client.key.pem
```

#### Built-in Validation

```bash
# Run certificate validation (requires Go build)
cd src/opamp-server
OPAMP_CERTS_DIR=/path/to/certs go run -tags validation ./cmd/validate-certs
```

## Advanced Configuration

### Custom Certificate Authority

For production environments, you may want to use certificates from a trusted CA:

1. **Obtain certificates** from your CA (Let's Encrypt, internal CA, etc.)
2. **Update paths** in configuration to point to your certificates
3. **Ensure compatibility** with OpAMP server certificate requirements

### Mutual TLS (mTLS)

Enable mutual TLS for enhanced security:

1. **Server configuration**:
   ```bash
   export OPAMP_TLS_REQUIRE_CLIENT_CERT=true
   ```

2. **Supervisor configuration**:
   ```yaml
   server:
     tls:
       cert_file: /path/to/client.cert.pem
       key_file: /path/to/client.key.pem
   ```

### Certificate Rotation

Implement automated certificate rotation:

1. **Monitor expiry** using the built-in validation tools
2. **Generate new certificates** before expiration
3. **Update configurations** and restart services
4. **Validate connectivity** after rotation

## Support

For issues related to certificate management:

1. Check the troubleshooting section above
2. Review server and supervisor logs
3. Validate certificate properties and expiry
4. Test network connectivity and TLS handshake

## Files Reference

- `generate.sh` - Certificate generation script
- `setup-supervisor.sh` - Supervisor configuration helper
- `openssl.conf` - OpenSSL CA configuration
- `server.conf` - Server certificate configuration
- `server_ext.conf` - Server certificate extensions
- `client.conf` - Client certificate configuration
- `supervisor.yaml` - Development supervisor configuration
- `supervisor-production.yaml` - Production supervisor template
