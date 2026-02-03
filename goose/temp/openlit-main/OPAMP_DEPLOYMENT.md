# OpenLIT OpAMP Server Deployment Guide

This guide explains how to deploy OpenLIT with the integrated OpAMP server for Fleet Hub management using Docker Compose.

## Overview

The OpenLIT OpAMP server provides secure communication between the OpenLIT platform and OpAMP supervisors managing OpenTelemetry Collectors. This deployment includes:

- **Automatic certificate generation** for secure TLS communication
- **Environment-aware configuration** (development vs production)
- **Dynamic supervisor configuration** based on environment settings
- **Controlled network configuration** with user-configurable security settings

## Quick Start

### 1. Clone and Setup

```bash
git clone https://github.com/openlit/openlit.git
cd openlit
```

### 2. Configure Environment (Optional)

Copy the example environment file and customize:

```bash
cp env.example .env
# Edit .env with your preferred settings
```

### 3. Deploy with Docker Compose

**For Development:**
```bash
docker-compose up -d
```

**For Production:**
```bash
# Production mode is now the default with secure TLS settings
# Set any custom overrides if needed
export OPAMP_ENVIRONMENT=production  # Default
export OPAMP_TLS_INSECURE_SKIP_VERIFY=false  # Default
export OPAMP_TLS_REQUIRE_CLIENT_CERT=true  # Default

docker-compose up -d
```

### 4. Access Services

- **OpenLIT UI**: http://localhost:3000
- **Fleet Hub**: http://localhost:3000/fleet-hub
- **OpAMP Server**: wss://localhost:4320/v1/opamp
- **OpAMP API**: http://localhost:8080

## Environment Configuration

### Available Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OPAMP_ENVIRONMENT` | `development` | Environment mode (`development`, `production`, `testing`) |
| `OPAMP_TLS_INSECURE_SKIP_VERIFY` | `true` | Skip certificate verification (dev only) |
| `OPAMP_TLS_REQUIRE_CLIENT_CERT` | `false` | Require client certificates |
| `OPAMP_TLS_MIN_VERSION` | `1.2` | Minimum TLS version |
| `OPAMP_TLS_MAX_VERSION` | `1.3` | Maximum TLS version |
| `OPAMP_LOG_LEVEL` | `info` | Logging level |

### Environment Modes

#### Development Mode (Default)
- Certificate verification is skipped (`insecure_skip_verify: true`)
- Client certificates are not required
- Suitable for local development and testing
- **⚠️ Not secure for production use**

#### Production Mode
- Full certificate verification enabled
- Client certificates required by default
- Strict TLS configuration
- Certificate expiry monitoring
- **✅ Production-ready security**

#### Testing Mode
- Similar to development mode
- Optimized for automated testing
- Flexible certificate validation

## Deployment Scenarios

### Scenario 1: Local Development

```bash
# Use default development settings
docker-compose up -d
```

The OpAMP supervisor will automatically connect using `insecure_skip_verify: true`.

### Scenario 2: Production Deployment

```bash
# Set production environment variables
export OPAMP_ENVIRONMENT=production
export OPAMP_TLS_INSECURE_SKIP_VERIFY=false
export OPAMP_TLS_REQUIRE_CLIENT_CERT=true

# Deploy with production settings
docker-compose up -d
```

### Scenario 3: Custom Configuration

Create a `.env` file:

```env
# Custom production configuration
OPAMP_ENVIRONMENT=production
OPAMP_TLS_INSECURE_SKIP_VERIFY=false
OPAMP_TLS_REQUIRE_CLIENT_CERT=true
OPAMP_TLS_MIN_VERSION=1.3
OPAMP_LOG_LEVEL=warn

# Database settings
OPENLIT_DB_PASSWORD=your-secure-password
```

Then deploy:

```bash
docker-compose up -d
```

## Certificate Management

### Automatic Certificate Generation

The deployment automatically generates:
- **CA Certificate**: Self-signed root certificate
- **Server Certificate**: For OpAMP server TLS
- **Client Certificate**: For mutual TLS authentication (required in production)

### Certificate Properties

- **Key Size**: 4096-bit RSA keys
- **Validity**: 10 years (CA), 1 year (server/client)
- **Subject Alternative Names**: Includes localhost, Docker IPs, and container hostnames
- **Extensions**: Proper key usage and extended key usage

### Custom Certificates

For production with custom certificates:

1. **Replace generated certificates** in the container:
   ```bash
   docker cp your-ca.cert.pem openlit:/app/opamp/certs/cert/ca.cert.pem
   docker cp your-server.cert.pem openlit:/app/opamp/certs/server/server.cert.pem
   docker cp your-server.key.pem openlit:/app/opamp/certs/server/server.key.pem
   ```

2. **Restart the container**:
   ```bash
   docker-compose restart openlit
   ```

## Connecting External Supervisors

### Development Connection

For external OpAMP supervisors in development:

```yaml
# supervisor.yaml
server:
  endpoint: wss://your-openlit-host:4320/v1/opamp
  tls:
    insecure_skip_verify: true

agent:
  executable: /path/to/otelcol-contrib
  config_files: 
    - /etc/otel/config.yaml

capabilities:
  accepts_remote_config: true
  reports_effective_config: true
  reports_own_logs: true
  reports_health: true
```

### Production Connection

For production deployments with mutual TLS:

1. **Extract certificates**:
   ```bash
   # Extract CA certificate
   docker cp openlit:/app/opamp/certs/cert/ca.cert.pem ./ca.cert.pem
   
   # Extract client certificates for mutual TLS
   docker cp openlit:/app/opamp/certs/client/client.cert.pem ./client.cert.pem
   docker cp openlit:/app/opamp/certs/client/client.key.pem ./client.key.pem
   ```

2. **Configure supervisor with mutual TLS**:
   ```yaml
   # supervisor.yaml
   server:
     endpoint: wss://your-openlit-host:4320/v1/opamp
     tls:
       insecure_skip_verify: false
       ca_file: /path/to/ca.cert.pem
       cert_file: /path/to/client.cert.pem
       key_file: /path/to/client.key.pem
   ```

3. **Deploy supervisor**:
   ```bash
   ./opampsupervisor --config supervisor.yaml
   ```

### Using the Setup Script

Use the included setup script for automated configuration:

```bash
# Extract setup script from container
docker cp openlit:/app/opamp/setup-supervisor.sh ./

# Generate development configuration
./setup-supervisor.sh --development

# Generate production configuration
./setup-supervisor.sh --production --server-url wss://your-server:4320/v1/opamp
```

## Monitoring and Troubleshooting

### Container Logs

```bash
# View all logs
docker-compose logs -f

# View OpAMP specific logs
docker-compose logs -f openlit | grep -E "(OpAMP|OPAMP)"

# View certificate generation logs
docker exec openlit cat /var/log/certs.log
```

### Health Checks

```bash
# Check OpAMP server status
curl -k https://localhost:8080/health

# Test TLS connection
openssl s_client -connect localhost:4320

# View certificate information
docker exec openlit openssl x509 -in /app/opamp/certs/cert/ca.cert.pem -noout -text
```

### Common Issues

#### "certificate signed by unknown authority"

**Cause**: Supervisor doesn't trust the self-signed CA certificate.

**Solutions**:
1. **Development**: Set `insecure_skip_verify: true` in supervisor config
2. **Production**: Provide CA certificate path in supervisor config

#### "connection refused"

**Cause**: OpAMP server not running or network issues.

**Solutions**:
1. Check container status: `docker-compose ps`
2. Verify port mapping: `docker-compose port openlit 4320`
3. Check firewall rules for port 4320

#### "certificate has expired"

**Cause**: Generated certificates have expired.

**Solutions**:
1. Regenerate certificates: `docker-compose restart openlit`
2. Check certificate expiry: `docker exec openlit openssl x509 -in /app/opamp/certs/server/server.cert.pem -noout -dates`

## Security Considerations

### Development Security

- Uses `insecure_skip_verify: true` by default
- Suitable for local development only
- **Never use in production**

### Production Security

- Full certificate verification enabled
- Mutual TLS support available
- Secure cipher suites and TLS versions
- Certificate expiry monitoring

### Best Practices

1. **Use production mode** for production deployments
2. **Rotate certificates** regularly
3. **Secure CA private key** with proper file permissions
4. **Monitor certificate expiry** and set up alerts
5. **Use strong passwords** for database and other services

## Advanced Configuration

### Mutual TLS Configuration

Mutual TLS is enabled by default in production mode. To disable it:

```bash
export OPAMP_TLS_REQUIRE_CLIENT_CERT=false
docker-compose up -d
```

Supervisor configuration with client certificates (required in production):

```yaml
server:
  tls:
    insecure_skip_verify: false
    ca_file: /path/to/ca.cert.pem
    cert_file: /path/to/client.cert.pem
    key_file: /path/to/client.key.pem
```

### Custom Certificate Validity

Modify certificate validity periods by setting environment variables before deployment:

```bash
export CERT_VALIDITY_DAYS=7300  # 20 years for CA
export SERVER_CERT_DAYS=730     # 2 years for server cert
docker-compose up -d
```

## Migration from Previous Versions

If upgrading from a previous version without OpAMP support:

1. **Backup existing data**:
   ```bash
   docker-compose down
   docker volume create openlit-backup
   docker run --rm -v openlit-data:/source -v openlit-backup:/backup alpine cp -r /source/. /backup/
   ```

2. **Update configuration**:
   ```bash
   # Pull latest changes
   git pull origin main
   
   # Update Docker Compose
   docker-compose pull
   ```

3. **Deploy with new configuration**:
   ```bash
   docker-compose up -d
   ```

## Support and Documentation

- **Fleet Hub Documentation**: [docs/latest/openlit/observability/fleet-hub.mdx](docs/latest/openlit/observability/fleet-hub.mdx)
- **Certificate Management**: [src/opamp-server/CERTIFICATES.md](src/opamp-server/CERTIFICATES.md)
- **OpAMP Specification**: [OpenTelemetry OpAMP](https://github.com/open-telemetry/opamp-spec)

## Environment Variables Reference

### User-Configurable Variables

```bash
# Environment and Security
OPAMP_ENVIRONMENT=development|production|testing
OPAMP_TLS_INSECURE_SKIP_VERIFY=true|false
OPAMP_TLS_REQUIRE_CLIENT_CERT=true|false
OPAMP_TLS_MIN_VERSION=1.0|1.1|1.2|1.3
OPAMP_TLS_MAX_VERSION=1.0|1.1|1.2|1.3
OPAMP_LOG_LEVEL=debug|info|warn|error

# Database Configuration
OPENLIT_DB_NAME=openlit
OPENLIT_DB_USER=default
OPENLIT_DB_PASSWORD=OPENLIT

# Application Configuration
PORT=3000
DOCKER_PORT=3000

# OAuth (Optional)
GOOGLE_CLIENT_ID=your-client-id
GOOGLE_CLIENT_SECRET=your-client-secret
GITHUB_CLIENT_ID=your-client-id
GITHUB_CLIENT_SECRET=your-client-secret
```
