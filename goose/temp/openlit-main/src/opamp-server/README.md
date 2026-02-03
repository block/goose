# Standalone OpAMP Server

This is a simplified version of the OpenTelemetry OpAMP server that can be used independently. It provides the core functionality of the OpAMP protocol without the UI server component.

## Features

- OpAMP protocol support
- Agent management
- Connection handling
- Status updates
- Configuration management

## Getting Started

1. Make sure you have Go installed (1.16 or later)

2. Create a new directory for your project and copy these files into it

3. Initialize the Go module:
   ```bash
   go mod init opamp-server
   ```

4. Install dependencies:
   ```bash
   go mod tidy
   ```

5. Run the server:
   ```bash
   go run .
   ```

The server will start and listen on `0.0.0.0:4320` by default.

## Configuration

The server is configured to run without TLS for simplicity. In a production environment, you should enable TLS by modifying the `Start()` function in `server/server.go`.

## API Endpoints

- `/v1/opamp` - The main OpAMP endpoint that agents connect to

## Integration with Other Services

You can integrate this server with your own services by:

1. Implementing custom configuration management
2. Adding authentication
3. Implementing custom agent status handling
4. Adding metrics collection

## License

This project uses the same license as the original OpenTelemetry OpAMP Go implementation.
