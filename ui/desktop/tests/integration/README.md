# Goosed Integration Tests

This test suite validates the `goosed` binary by issuing requests through the TypeScript API client.

## Prerequisites

1. Build the goosed binary:
   ```bash
   # in the project root
   cargo build --bin goosed
   ```

2. Install npm dependencies:
   ```bash
   cd ui/desktop
   npm install
   ```

## Running Tests

```bash
# Run all integration tests
npm run test:integration

# Run in watch mode (re-runs on file changes)
npm run test:integration:watch

# Run with debug output
npm run test:integration:debug
```
