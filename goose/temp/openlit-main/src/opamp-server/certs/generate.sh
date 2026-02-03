#!/bin/bash

# OpenLIT OpAMP Server Certificate Generation Script
# This script creates a complete PKI infrastructure for secure OpAMP communication
# including CA, server, and client certificates with proper production settings.

set -e  # Exit on any error

# Configuration
CERT_VALIDITY_DAYS=${CERT_VALIDITY_DAYS:-3650}  # 10 years for CA
SERVER_CERT_DAYS=${SERVER_CERT_DAYS:-365}       # 1 year for server cert
CLIENT_CERT_DAYS=${CLIENT_CERT_DAYS:-365}       # 1 year for client cert

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
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

# Check if OpenSSL is available
if ! command -v openssl &> /dev/null; then
    log_error "OpenSSL is not installed or not in PATH"
    exit 1
fi

log_info "Starting certificate generation for OpenLIT OpAMP Server..."

# Create necessary directories
log_info "Creating certificate directories..."
mkdir -p cert server client private
chmod 755 cert server client
chmod 700 private

# Clean up existing certificates if requested
if [[ "${1:-}" == "--clean" ]] || [[ ! -f "index.txt" ]]; then
    log_info "Cleaning up existing certificates..."
    ./clear.sh
fi

# Initialize CA database if it doesn't exist
if [[ ! -f "index.txt" ]]; then
    touch index.txt
    echo 01 > serial
fi

# Generate CA private key
log_info "Generating CA private key..."
if [[ ! -f "private/ca.key.pem" ]]; then
    openssl genrsa -out private/ca.key.pem 4096
    chmod 600 private/ca.key.pem
    log_info "CA private key generated successfully"
else
    log_warn "CA private key already exists, skipping generation"
fi

# Generate CA certificate
log_info "Generating CA certificate..."
if [[ ! -f "cert/ca.cert.pem" ]]; then
    openssl req -new -x509 -days $CERT_VALIDITY_DAYS -key private/ca.key.pem -out cert/ca.cert.pem -config openssl.conf
    chmod 644 cert/ca.cert.pem
    log_info "CA certificate generated successfully"
    
    # Display CA certificate info
    log_info "CA Certificate Information:"
    openssl x509 -in cert/ca.cert.pem -noout -subject -dates
else
    log_warn "CA certificate already exists, skipping generation"
fi

# Generate client private key
log_info "Generating client private key..."
if [[ ! -f "client/client.key.pem" ]]; then
    openssl genrsa -out client/client.key.pem 4096
    chmod 600 client/client.key.pem
    log_info "Client private key generated successfully"
else
    log_warn "Client private key already exists, skipping generation"
fi

# Generate client certificate signing request
log_info "Generating client certificate signing request..."
if [[ ! -f "client/client.csr" ]]; then
    openssl req -new -key client/client.key.pem -out client/client.csr -config client.conf
    chmod 600 client/client.csr
    log_info "Client CSR generated successfully"
else
    log_warn "Client CSR already exists, skipping generation"
fi

# Generate client certificate
log_info "Generating client certificate..."
if [[ ! -f "client/client.cert.pem" ]]; then
    openssl ca -config openssl.conf -days $CLIENT_CERT_DAYS -notext -batch -in client/client.csr -out client/client.cert.pem
    chmod 644 client/client.cert.pem
    log_info "Client certificate generated successfully"
    
    # Display client certificate info
    log_info "Client Certificate Information:"
    openssl x509 -in client/client.cert.pem -noout -subject -dates
else
    log_warn "Client certificate already exists, skipping generation"
fi

# Generate server private key
log_info "Generating server private key..."
if [[ ! -f "server/server.key.pem" ]]; then
    openssl genrsa -out server/server.key.pem 4096
    chmod 600 server/server.key.pem
    log_info "Server private key generated successfully"
else
    log_warn "Server private key already exists, skipping generation"
fi

# Generate server certificate signing request
log_info "Generating server certificate signing request..."
if [[ ! -f "server/server.csr" ]]; then
    openssl req -new -key server/server.key.pem -out server/server.csr -config server.conf
    chmod 600 server/server.csr
    log_info "Server CSR generated successfully"
else
    log_warn "Server CSR already exists, skipping generation"
fi

# Generate server certificate
log_info "Generating server certificate..."
if [[ ! -f "server/server.cert.pem" ]]; then
    openssl ca -config openssl.conf -extfile server_ext.conf -days $SERVER_CERT_DAYS -notext -batch -in server/server.csr -out server/server.cert.pem
    chmod 644 server/server.cert.pem
    log_info "Server certificate generated successfully"
    
    # Display server certificate info
    log_info "Server Certificate Information:"
    openssl x509 -in server/server.cert.pem -noout -subject -dates
    log_info "Server Certificate Subject Alternative Names:"
    openssl x509 -in server/server.cert.pem -noout -text | grep -A 1 "Subject Alternative Name" || echo "No SAN found"
else
    log_warn "Server certificate already exists, skipping generation"
fi

# Verify certificate chain
log_info "Verifying certificate chain..."
if openssl verify -CAfile cert/ca.cert.pem server/server.cert.pem > /dev/null 2>&1; then
    log_info "Server certificate verification: PASSED"
else
    log_error "Server certificate verification: FAILED"
    exit 1
fi

if openssl verify -CAfile cert/ca.cert.pem client/client.cert.pem > /dev/null 2>&1; then
    log_info "Client certificate verification: PASSED"
else
    log_error "Client certificate verification: FAILED"
    exit 1
fi

# Set proper file permissions
log_info "Setting proper file permissions..."
find . -name "*.key.pem" -exec chmod 600 {} \;
find . -name "*.cert.pem" -exec chmod 644 {} \;
find . -name "*.csr" -exec chmod 600 {} \;
chmod 700 private
chmod 755 cert server client

log_info "Certificate generation completed successfully!"
log_info "Generated files:"
log_info "  CA Certificate: cert/ca.cert.pem"
log_info "  CA Private Key: private/ca.key.pem"
log_info "  Server Certificate: server/server.cert.pem"
log_info "  Server Private Key: server/server.key.pem"
log_info "  Client Certificate: client/client.cert.pem"
log_info "  Client Private Key: client/client.key.pem"

# Check certificate expiration
log_info "Certificate expiration dates:"
echo "CA Certificate:"
openssl x509 -in cert/ca.cert.pem -noout -dates | grep "notAfter"
echo "Server Certificate:"
openssl x509 -in server/server.cert.pem -noout -dates | grep "notAfter"
echo "Client Certificate:"
openssl x509 -in client/client.cert.pem -noout -dates | grep "notAfter"
