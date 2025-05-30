# Temporal Service

This is a Go service that connects to a Temporal server and provides an HTTP API for managing scheduled Goose jobs.

## Prerequisites

You need to have the Temporal CLI installed and a Temporal server running:

1. **Install Temporal CLI**:
   ```bash
   # macOS
   brew install temporal
   
   # Or download from https://github.com/temporalio/cli/releases
   ```

2. **Start Temporal Server**:
   ```bash
   temporal server start-dev
   ```
   This starts a development server on `localhost:7233` with a Web UI on `localhost:8233`.

## Features

- **Temporal Integration**: Uses Temporal for reliable job scheduling and execution
- **HTTP API**: RESTful API for job management
- **Job Scheduling**: Create, delete, pause, unpause, and list scheduled jobs
- **Manual Execution**: Run jobs immediately on demand
- **Persistent Storage**: Jobs survive service restarts via Temporal

## Building

```bash
./build.sh
```

This will create a `temporal-service` binary in the current directory.

## Running

**Important**: Make sure Temporal server is running first:
```bash
# Terminal 1: Start Temporal server
temporal server start-dev

# Terminal 2: Start the service
./temporal-service
```

### Environment Variables

- `PORT`: HTTP server port (default: `8080`)

## API Endpoints

### Health Check
```
GET /health
```

### Job Management
```
POST /jobs
```

#### Create Schedule
```json
{
  "action": "create",
  "job_id": "my-job",
  "cron": "0 */6 * * *",
  "recipe_path": "/path/to/recipe.yaml"
}
```

#### List Schedules
```json
{
  "action": "list"
}
```

#### Delete Schedule
```json
{
  "action": "delete",
  "job_id": "my-job"
}
```

#### Pause Schedule
```json
{
  "action": "pause",
  "job_id": "my-job"
}
```

#### Unpause Schedule
```json
{
  "action": "unpause",
  "job_id": "my-job"
}
```

#### Run Job Now
```json
{
  "action": "run_now",
  "job_id": "my-job"
}
```

## Response Format

All responses follow this format:
```json
{
  "success": true,
  "message": "Operation completed",
  "jobs": [...],  // For list operations
  "data": {...}   // For specific data like session IDs
}
```

## Integration with Goose

The service expects a `goose-scheduler-executor` binary to be available in the PATH. This binary should:

1. Accept two arguments: `job_id` and `recipe_path`
2. Execute the Goose recipe
3. Output the session ID to stdout
4. Exit with code 0 on success, non-zero on failure

## Temporal Server Details

- **Frontend gRPC**: `127.0.0.1:7233`
- **Database**: SQLite in `{GOOSE_DATA_DIR}/temporal.db`
- **Namespace**: `default`
- **Task Queue**: `goose-task-queue`

## Logging

The service logs to stdout. Key events logged:
- Service startup/shutdown
- Job creation/deletion/execution
- Errors and warnings

## Graceful Shutdown

The service handles SIGINT and SIGTERM signals for graceful shutdown:
1. Stops accepting new HTTP requests
2. Completes in-flight requests (30s timeout)
3. Stops Temporal worker
4. Closes Temporal client
5. Stops Temporal server