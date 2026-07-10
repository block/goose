# Goose Sonar bridge

`goose-sonar-bridge` isolates Sonar Core and its persistent Marmot/MLS storage from the Goose process. This boundary is required because the two dependency graphs currently link incompatible `libsqlite3-sys` versions.

The bridge is an implementation detail of the feature-gated Goose Sonar gateway. It communicates over a versioned JSON-lines protocol on stdin/stdout; logs are written only to stderr.

## Build

From the Goose repository root:

```bash
cargo build --release --manifest-path integrations/sonar-bridge/Cargo.toml
cargo build --release -p goose-cli --features sonar-gateway
```

Pass the resulting bridge binary to `goose gateway start sonar --bridge-path ...`, or install it as `goose-sonar-bridge` on `PATH`.

Initialize the bridge identity and print the npub that should be invited to a Sonar group:

```bash
goose-sonar-bridge --home /path/to/state identity
```

## State and recovery

The bridge creates a Nostr identity, SQLCipher key, Marmot database, and command ledger under its state directory. The default is the Goose data directory's `sonar-gateway` subdirectory; `--home` or `GOOSE_SONAR_HOME` can override it.

Back up the whole directory while the gateway is stopped. The Nostr secret key and database key in `config.json` are sensitive. Losing this directory loses the bridge identity and may make existing MLS group history unrecoverable from relays. Never commit or share it.

On Unix, state directories are mode `0700` and JSON state is mode `0600`. One bridge process holds an exclusive lock for each state directory.

The durable ledger checkpoints existing history when a group first appears. It records a new inbound message before Goose executes it and records completion afterward. If the process exits during execution, the message is not replayed automatically; the group receives a request to resubmit it.

## Trust boundary

Sonar Core authenticates MLS senders. The bridge accepts group invitations only when the welcome was sent by a configured controller, and the Goose gateway requires the same explicit controller npub allowlist for commands. Group membership grants access to the conversation, not permission to run local tools. All group members can currently administer MLS membership, so only share a group with people who may read its future traffic.

Sonar Core is pinned by commit in `Cargo.toml` and `Cargo.lock`. Review upstream protocol and storage changes before updating that revision.
