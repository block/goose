#!/bin/bash

# OpenLIT OpAMP Supervisor Setup Script
# This script helps configure the OpAMP supervisor for secure communication with the OpenLIT server

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERTS_DIR="${SCRIPT_DIR}/certs"
ASSETS_DIR="${SCRIPT_DIR}/../../assets"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

print_banner() {
    echo -e "${BLUE}"
    echo "╔══════════════════════════════════════════════════════════════════════════════════════╗"
    echo "║                          OpenLIT OpAMP Supervisor Setup                             ║"
    echo "║                                                                                      ║"
    echo "║  This script configures the OpAMP supervisor for secure communication with          ║"
    echo "║  your OpenLIT OpAMP server using proper TLS certificates.                          ║"
    echo "╚══════════════════════════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --development     Configure for development (insecure_skip_verify=true)"
    echo "  --production      Configure for production (with CA certificate)"
    echo "  --server-url URL  Set the OpAMP server URL (default: wss://localhost:4320/v1/opamp)"
    echo "  --output-dir DIR  Output directory for supervisor config (default: ./supervisor-config)"
    echo "  --version VER     OpAMP supervisor version (default: v0.142.0 or from OPAMP_SUPERVISOR_VERSION env)"
    echo "  --arch ARCH       Target architecture (default: amd64 or from TARGETARCH env)"
    echo "  --help           Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --development"
    echo "  $0 --production --server-url wss://my-openlit-server:4320/v1/opamp"
    echo "  $0 --production --version v0.142.0 --arch arm64"
}

# Default values (can be overridden by environment variables or command line)
ENVIRONMENT="development"
SERVER_URL="wss://localhost:4320/v1/opamp"
OUTPUT_DIR="./supervisor-config"
OPAMP_SUPERVISOR_VERSION="${OPAMP_SUPERVISOR_VERSION:-v0.142.0}"

# Auto-detect architecture if not provided
if [[ -z "${TARGETARCH}" ]]; then
    ARCH=$(uname -m)
    case "${ARCH}" in
        x86_64)
            TARGETARCH="amd64"
            ;;
        aarch64|arm64)
            TARGETARCH="arm64"
            ;;
        armv7l)
            TARGETARCH="arm"
            ;;
        *)
            log_warn "Unknown architecture: ${ARCH}, defaulting to amd64"
            TARGETARCH="amd64"
            ;;
    esac
else
    TARGETARCH="${TARGETARCH}"
fi

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --development)
            ENVIRONMENT="development"
            shift
            ;;
        --production)
            ENVIRONMENT="production"
            shift
            ;;
        --server-url)
            SERVER_URL="$2"
            shift 2
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --version)
            OPAMP_SUPERVISOR_VERSION="$2"
            shift 2
            ;;
        --arch)
            TARGETARCH="$2"
            shift 2
            ;;
        --help)
            show_usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

print_banner

log_step "Configuring OpAMP supervisor for ${ENVIRONMENT} environment"
log_info "OpAMP Supervisor Version: ${OPAMP_SUPERVISOR_VERSION}"
log_info "Target Architecture: ${TARGETARCH}"

# Create output directory
mkdir -p "${OUTPUT_DIR}"

# Check if certificates exist
if [[ "${ENVIRONMENT}" == "production" ]]; then
    if [[ ! -f "${CERTS_DIR}/cert/ca.cert.pem" ]]; then
        log_error "CA certificate not found at ${CERTS_DIR}/cert/ca.cert.pem"
        log_info "Please run the certificate generation script first:"
        log_info "  cd ${CERTS_DIR} && ./generate.sh"
        exit 1
    fi
    
    if [[ ! -f "${CERTS_DIR}/client/client.cert.pem" ]]; then
        log_warn "Client certificate not found at ${CERTS_DIR}/client/client.cert.pem"
        log_info "Client certificates are optional but recommended for mutual TLS"
    fi
fi

# Generate supervisor configuration
SUPERVISOR_CONFIG="${OUTPUT_DIR}/supervisor.yaml"

log_info "Generating supervisor configuration..."

cat > "${SUPERVISOR_CONFIG}" << EOF
server:
  # The endpoint of the OpAMP server
  endpoint: ${SERVER_URL}
  tls:
EOF

if [[ "${ENVIRONMENT}" == "development" ]]; then
    cat >> "${SUPERVISOR_CONFIG}" << EOF
    # Development configuration - skips certificate verification
    # WARNING: This is insecure and should only be used for development!
    insecure_skip_verify: true
EOF
else
    cat >> "${SUPERVISOR_CONFIG}" << EOF
    # Production configuration with proper certificate verification
    insecure_skip_verify: false
    
    # Path to the CA certificate that signed the server certificate
    ca_file: /app/opamp/certs/cert/ca.cert.pem
    
    # Optional: Client certificate for mutual TLS authentication
    # Uncomment these lines if the server requires client certificates
    # cert_file: /app/opamp/certs/client/client.cert.pem
    # key_file: /app/opamp/certs/client/client.key.pem
EOF
fi

cat >> "${SUPERVISOR_CONFIG}" << EOF

agent:
  # The path to the OpenTelemetry Collector binary managed by this supervisor
  executable: /app/opamp/otelcontribcol

  # Reference to external configuration file
  config_files: 
    - /etc/otel/otel-collector-config.yaml

capabilities:
  accepts_remote_config: true
  reports_effective_config: true
  reports_own_metrics: false
  reports_own_logs: true
  reports_own_traces: false
  reports_health: true
  reports_remote_config: true

storage:
  directory: ./storage
EOF

log_info "Supervisor configuration generated: ${SUPERVISOR_CONFIG}"

# Copy certificates if in production mode
if [[ "${ENVIRONMENT}" == "production" ]]; then
    log_info "Copying certificates to output directory..."
    
    mkdir -p "${OUTPUT_DIR}/certs/cert"
    mkdir -p "${OUTPUT_DIR}/certs/client"
    
    cp "${CERTS_DIR}/cert/ca.cert.pem" "${OUTPUT_DIR}/certs/cert/"
    
    if [[ -f "${CERTS_DIR}/client/client.cert.pem" ]]; then
        cp "${CERTS_DIR}/client/client.cert.pem" "${OUTPUT_DIR}/certs/client/"
        cp "${CERTS_DIR}/client/client.key.pem" "${OUTPUT_DIR}/certs/client/"
        log_info "Client certificates copied for mutual TLS"
    fi
    
    log_info "Certificates copied to ${OUTPUT_DIR}/certs/"
fi

# Generate deployment instructions
INSTRUCTIONS_FILE="${OUTPUT_DIR}/DEPLOYMENT.md"

log_info "Generating deployment instructions..."

cat > "${INSTRUCTIONS_FILE}" << EOF
# OpAMP Supervisor Deployment Instructions

## Configuration Generated
- **Environment**: ${ENVIRONMENT}
- **Server URL**: ${SERVER_URL}
- **Configuration File**: supervisor.yaml

## Deployment Steps

### 1. Download OpAMP Supervisor

\`\`\`bash
# Download the OpAMP supervisor binary
# Version: ${OPAMP_SUPERVISOR_VERSION}
# Architecture: ${TARGETARCH}
OPAMP_VERSION="${OPAMP_SUPERVISOR_VERSION}"
OPAMP_ARCH="${TARGETARCH}"
OPAMP_VERSION_NO_V="${OPAMP_SUPERVISOR_VERSION#v}"

curl --proto '=https' --tlsv1.2 -fL -o "opampsupervisor_linux_\${OPAMP_ARCH}" \
    "https://github.com/open-telemetry/opentelemetry-collector-releases/releases/download/cmd%2Fopampsupervisor%2F\${OPAMP_VERSION}/opampsupervisor_\${OPAMP_VERSION_NO_V}_linux_\${OPAMP_ARCH}"

# Make it executable
chmod +x "opampsupervisor_linux_\${OPAMP_ARCH}"
\`\`\`

### 2. Deploy Configuration

\`\`\`bash
# Copy the supervisor configuration
cp supervisor.yaml /etc/opamp/supervisor.yaml
EOF

if [[ "${ENVIRONMENT}" == "production" ]]; then
    cat >> "${INSTRUCTIONS_FILE}" << EOF

# Copy certificates (production only)
sudo mkdir -p /app/opamp/certs
sudo cp -r certs/* /app/opamp/certs/
sudo chown -R root:root /app/opamp/certs
sudo chmod 644 /app/opamp/certs/cert/ca.cert.pem
sudo chmod 600 /app/opamp/certs/client/client.key.pem 2>/dev/null || true
EOF
fi

cat >> "${INSTRUCTIONS_FILE}" << EOF
\`\`\`

### 3. Start the Supervisor

\`\`\`bash
# Start the OpAMP supervisor
./opampsupervisor_linux_${TARGETARCH} --config /etc/opamp/supervisor.yaml
\`\`\`

### 4. Verify Connection

Check the supervisor logs for successful connection:
- Look for "Connected to OpAMP server" messages
- Verify no TLS certificate errors

## Troubleshooting

### Certificate Issues
EOF

if [[ "${ENVIRONMENT}" == "production" ]]; then
    cat >> "${INSTRUCTIONS_FILE}" << EOF
- Ensure CA certificate is accessible at the configured path
- Verify certificate file permissions (644 for certs, 600 for keys)
- Check that the server certificate includes the correct hostname/IP in SAN
EOF
else
    cat >> "${INSTRUCTIONS_FILE}" << EOF
- Development mode uses insecure_skip_verify=true
- For production, use --production flag to generate secure configuration
EOF
fi

cat >> "${INSTRUCTIONS_FILE}" << EOF

### Network Issues
- Verify the server URL is correct and accessible
- Check firewall rules for port 4320
- Ensure WebSocket connections are allowed

### Server Issues
- Verify the OpAMP server is running and listening on the correct port
- Check server logs for connection attempts and errors

## Security Notes
EOF

if [[ "${ENVIRONMENT}" == "production" ]]; then
    cat >> "${INSTRUCTIONS_FILE}" << EOF
- This configuration uses proper TLS certificate verification
- CA certificate must be distributed securely to all supervisor instances
- Consider using client certificates for mutual TLS authentication
- Regularly rotate certificates before expiration
EOF
else
    cat >> "${INSTRUCTIONS_FILE}" << EOF
- **WARNING**: Development configuration skips certificate verification
- This is insecure and should NEVER be used in production
- Use --production flag for secure production configuration
EOF
fi

cat >> "${INSTRUCTIONS_FILE}" << EOF

## Environment Variables

You can override configuration using environment variables:
- \`OPAMP_SERVER_URL\`: Override the server endpoint
- \`OPAMP_CA_FILE\`: Override the CA certificate path
- \`OPAMP_CLIENT_CERT\`: Override the client certificate path
- \`OPAMP_CLIENT_KEY\`: Override the client key path
EOF

log_info "Deployment instructions generated: ${INSTRUCTIONS_FILE}"

# Summary
echo ""
log_step "Setup Complete!"
echo ""
log_info "Generated files:"
log_info "  📄 Configuration: ${SUPERVISOR_CONFIG}"
log_info "  📋 Instructions:  ${INSTRUCTIONS_FILE}"

if [[ "${ENVIRONMENT}" == "production" ]]; then
    log_info "  🔐 Certificates:  ${OUTPUT_DIR}/certs/"
fi

echo ""
log_info "Next steps:"
log_info "  1. Review the generated configuration"
log_info "  2. Follow the deployment instructions in ${INSTRUCTIONS_FILE}"
log_info "  3. Start the OpAMP supervisor with the new configuration"

if [[ "${ENVIRONMENT}" == "development" ]]; then
    echo ""
    log_warn "Development mode is configured with insecure_skip_verify=true"
    log_warn "This is NOT secure and should only be used for development!"
    log_info "For production, run: $0 --production --server-url wss://your-server:4320/v1/opamp"
fi

echo ""
log_info "🎉 OpAMP supervisor setup completed successfully!"
