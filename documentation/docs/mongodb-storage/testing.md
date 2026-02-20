---
sidebar_position: 2
title: Testing
sidebar_label: Testing
---

The MongoDB storage backend (`mongodb-storage` feature) requires a running MongoDB instance to test against. Tests are skipped when `GOOSE_MONGODB_URI` is not set.

## Prerequisites

- Docker
- Rust toolchain with cargo

## Start a MongoDB Test Instance

```bash
docker run -d \
  --name goose-mongo-test \
  -p 27017:27017 \
  mongo:6
```

## Run the Tests

```bash
GOOSE_MONGODB_URI=mongodb://localhost:27017 \
  cargo test -p goose --features mongodb-storage --lib mongodb_storage::tests
```

:::warning
If `GOOSE_MONGODB_URI` is not set, every test returns early and reports `ok` without actually running. This is by design so the default `cargo test` is not broken, but it means **a passing test run without the env var proves nothing**.
:::

## Test Isolation

Each test creates a unique database (`goose_test_<uuid>`) and drops it after completion. Tests do not interfere with each other or with any existing data.

## Stop the Test Instance

```bash
docker stop goose-mongo-test && docker rm goose-mongo-test
```

## CI

In GitHub Actions, add a MongoDB service container:

```yaml
services:
  mongodb:
    image: mongo:6
    ports:
      - 27017:27017

env:
  GOOSE_MONGODB_URI: mongodb://localhost:27017
```

Then run:

```bash
cargo test -p goose --features mongodb-storage --lib mongodb_storage::tests
```
