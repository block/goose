# Setting Up Your Own Matrix Synapse Server

## Quick Docker Setup

### 1. Create docker-compose.yml
```yaml
version: '3.8'
services:
  synapse:
    image: matrixdotorg/synapse:latest
    container_name: synapse
    ports:
      - "8008:8008"
    volumes:
      - ./synapse-data:/data
    environment:
      - SYNAPSE_SERVER_NAME=localhost
      - SYNAPSE_REPORT_STATS=no
    command: |
      bash -c "
        python -m synapse.app.homeserver \
          --server-name=localhost \
          --config-path=/data/homeserver.yaml \
          --generate-config \
          --report-stats=no || true
        python -m synapse.app.homeserver --config-path=/data/homeserver.yaml
      "

  postgres:
    image: postgres:13
    container_name: synapse-postgres
    environment:
      - POSTGRES_DB=synapse
      - POSTGRES_USER=synapse
      - POSTGRES_PASSWORD=changeme
    volumes:
      - ./postgres-data:/var/lib/postgresql/data
```

### 2. Start the Server
```bash
# Create directories
mkdir synapse-data postgres-data

# Start services
docker-compose up -d

# Check logs
docker-compose logs -f synapse
```

### 3. Enable Registration
```bash
# Edit the config to allow registration
docker exec -it synapse bash
# Edit /data/homeserver.yaml
# Set: enable_registration: true
# Restart: docker-compose restart synapse
```

### 4. Update Your MatrixService
```typescript
export const matrixService = new MatrixService({
  homeserverUrl: 'http://localhost:8008',
});
```

## Production Deployment

For production, you'll want:
- Proper domain name and SSL certificates
- PostgreSQL database
- Reverse proxy (nginx)
- Backup strategy

See: https://matrix-org.github.io/synapse/latest/setup/installation.html
