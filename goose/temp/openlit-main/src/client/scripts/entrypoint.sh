#!/bin/bash
set -e

# Generate and persist NextAuth.js secret to ensure session persistence across restarts
NEXTAUTH_SECRET_FILE="/app/client/data/.nextauth_secret"

if [ -f "$NEXTAUTH_SECRET_FILE" ]; then
    # Load existing secret from persistent volume
    export NEXTAUTH_SECRET=$(cat "$NEXTAUTH_SECRET_FILE")
    echo "✅ Loaded existing NextAuth secret from persistent storage"
else
    # Generate new secret and save it to persistent volume
    export NEXTAUTH_SECRET=$(openssl rand -base64 32)
    echo "$NEXTAUTH_SECRET" > "$NEXTAUTH_SECRET_FILE"
    chmod 600 "$NEXTAUTH_SECRET_FILE"
    echo "✅ Generated and saved new NextAuth secret to persistent storage"
fi

# Set NextAuth.js environment variables
# remove any existing NEXTAUTH_SECRET line, then append the current value
if [ -w /etc/environment ]; then
    sed -i '/^NEXTAUTH_SECRET=/d' /etc/environment
    echo "NEXTAUTH_SECRET=$NEXTAUTH_SECRET" >> /etc/environment
else
    echo "WARNING: /etc/environment is not writable; NEXTAUTH_SECRET will not be persisted there." >&2
fi

echo "NEXTAUTH_URL=http://localhost:${DOCKER_PORT:-3000}" >> /etc/environment
echo "SQLITE_DATABASE_URL=${SQLITE_DATABASE_URL:-file:../data/data.db}" >> /etc/environment
echo "PATH=./node_modules/.bin:$PATH" >> /etc/environment

# Environment variables for DB config
echo "INIT_DB_USERNAME=${INIT_DB_USERNAME}" >> /etc/environment
echo "INIT_DB_PASSWORD=${INIT_DB_PASSWORD}" >> /etc/environment
echo "INIT_DB_HOST=${INIT_DB_HOST}" >> /etc/environment
echo "INIT_DB_PORT=${INIT_DB_PORT}" >> /etc/environment
echo "INIT_DB_DATABASE=${INIT_DB_DATABASE}" >> /etc/environment

# Load the environment variables
. /etc/environment

# Run Prisma migrations and generate prisma client
prisma migrate deploy
prisma generate

# Run the seed 
prisma db seed

# Run crond in the background
service cron start

# Set OpAMP environment variables for entrypoint
export OPAMP_ENVIRONMENT=${OPAMP_ENVIRONMENT:-production}
export OPAMP_CERTS_DIR=${OPAMP_CERTS_DIR:-/app/opamp/certs}
export OPAMP_TLS_INSECURE_SKIP_VERIFY=${OPAMP_TLS_INSECURE_SKIP_VERIFY:-false}

echo "OpAMP Configuration:"
echo "  Environment: $OPAMP_ENVIRONMENT"
echo "  Certificates Directory: $OPAMP_CERTS_DIR"
echo "  TLS Insecure Skip Verify: $OPAMP_TLS_INSECURE_SKIP_VERIFY"

# Generate certificates using enhanced script
echo "Generating OpAMP certificates..."
chmod +x /app/opamp/certs/clear.sh
chmod +x /app/opamp/certs/generate.sh
cd /app/opamp/certs

# Generate certificates with proper logging
if ./generate.sh > /var/log/certs.log 2>&1; then
    echo "✅ Certificates generated successfully"
else
    echo "❌ Certificate generation failed, check /var/log/certs.log"
    cat /var/log/certs.log
fi

# Generate supervisor configuration based on environment
echo "Configuring OpAMP supervisor for $OPAMP_ENVIRONMENT environment..."
cd /app/opamp

# Use a runtime config location to avoid overwriting mounted files
SUPERVISOR_CONFIG_PATH="/app/opamp/supervisor-runtime.yaml"

# Create supervisor configuration dynamically
cat > "$SUPERVISOR_CONFIG_PATH" << EOF
server:
  endpoint: wss://localhost:4320/v1/opamp
  tls:
EOF

if [[ "$OPAMP_ENVIRONMENT" == "production" ]]; then
    cat >> "$SUPERVISOR_CONFIG_PATH" << EOF
    insecure_skip_verify: false
    ca_file: /app/opamp/certs/cert/ca.cert.pem
    cert_file: /app/opamp/certs/client/client.cert.pem
    key_file: /app/opamp/certs/client/client.key.pem
EOF
    echo "  Production mode: Using CA certificate verification with client certificates"
else
    cat >> "$SUPERVISOR_CONFIG_PATH" << EOF
    insecure_skip_verify: true
EOF
    echo "  Development mode: Skipping certificate verification"
fi

cat >> "$SUPERVISOR_CONFIG_PATH" << EOF

agent:
  executable: /app/opamp/otelcontribcol
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
  directory: /app/client/data/supervisor-storage
EOF

echo "✅ Supervisor configuration generated at $SUPERVISOR_CONFIG_PATH"

# Starting OpAMP Server
echo "Starting OpAMP Server..."
/app/opamp/opamp-server &
OPAMP_SERVER_PID=$!

# Wait a moment for the server to start
sleep 2

# Starting Supervisor for the OTEL Collector
echo "Starting OpAMP Supervisor..."
/app/opamp/opampsupervisor --config="$SUPERVISOR_CONFIG_PATH" &
SUPERVISOR_PID=$!


# Starting the OpenLIT UI Server
export PORT=${DOCKER_PORT:-3000} 
# Start the Next.js application
exec node --max_old_space_size=512 /app/client/server.js