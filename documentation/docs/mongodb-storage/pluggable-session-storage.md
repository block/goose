---
sidebar_position: 1
title: Pluggable Session Storage
sidebar_label: Pluggable Session Storage
---

goose persists session data (conversations, messages, metadata, token usage) through a pluggable storage backend. By default, goose uses SQLite, which stores sessions locally in `~/.config/goose/sessions/sessions.db`. The MongoDB backend is an alternative that persists sessions to a centralized MongoDB instance, enabling shared storage for multi-node deployments.

## Selecting a Backend

Set the `GOOSE_SESSION_STORAGE` environment variable to choose a backend:

| Value | Backend | Notes |
|-------|---------|-------|
| _(unset)_ | SQLite | Default, no configuration required |
| `mongodb` | MongoDB | Requires `mongodb-storage` feature at build time |

**Example:**

```bash
export GOOSE_SESSION_STORAGE=mongodb
```

:::warning
If `GOOSE_SESSION_STORAGE=mongodb` is set but goose was not built with the `mongodb-storage` feature, goose will exit with an error on startup.
:::

## MongoDB Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `GOOSE_SESSION_STORAGE` | Yes | `sqlite` | Set to `mongodb` to enable |
| `GOOSE_MONGODB_URI` | Yes | — | MongoDB connection string |
| `GOOSE_MONGODB_DATABASE` | No | `goose` | Database name |
| `GOOSE_MONGODB_SESSIONS_COLLECTION` | No | `sessions` | Sessions collection name |
| `GOOSE_MONGODB_MESSAGES_COLLECTION` | No | `messages` | Messages collection name |
| `GOOSE_MONGODB_MAX_POOL_SIZE` | No | driver default | Connection pool size |
| `GOOSE_MONGODB_CONNECT_TIMEOUT_MS` | No | driver default | Connection timeout in milliseconds |
| `GOOSE_MONGODB_SERVER_SELECTION_TIMEOUT_MS` | No | driver default | Server selection timeout in milliseconds |

**Example:**

```bash
export GOOSE_SESSION_STORAGE=mongodb
export GOOSE_MONGODB_URI=mongodb://user:pass@host:27017/goose?authSource=admin
export GOOSE_MONGODB_DATABASE=goose
```

## Building with MongoDB Support

MongoDB support is behind an optional feature flag to avoid adding the MongoDB driver to default builds:

```bash
cargo build --release --features mongodb-storage
```

## Document Format

MongoDB documents use the same field names and structure as goose's `export_session` JSON output. This means a `mongoexport` of the sessions and messages collections produces JSON that can be consumed by the same tools that work with exported sessions.

Structured fields like `extension_data`, `recipe`, `model_config`, message `content`, and message `metadata` are stored as native BSON documents rather than serialized JSON strings.

