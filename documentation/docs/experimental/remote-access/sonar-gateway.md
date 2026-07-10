---
title: Sonar Gateway
sidebar_position: 3
sidebar_label: Sonar Gateway
description: Control a Goose session from a shared, end-to-end encrypted Sonar group.
---

The Sonar Gateway binds one Sonar/Marmot MLS group to one Goose session. You can use the Sonar app today and build a compatible Goose Mobile fork on the same Sonar Core transport later.

:::warning Experimental feature
The gateway and bridge protocol are experimental. Run one Goose backend owner for the controlled session; independently running desktop, ACP, and gateway processes do not yet coordinate session execution.
:::

## Security model

Sonar provides authenticated, end-to-end encrypted group transport over Nostr relays. Goose adds two local authorization checks:

- The group must be paired to a Goose session with a short-lived code.
- The message sender's authenticated npub must be in the gateway's controller allowlist.
- The bridge accepts new group invitations only when an allowed controller sent the welcome.

Joining the group does not grant permission to run Goose. Observers can read group traffic but their commands are rejected. Sonar currently allows every MLS group member to administer membership, so share a control group only with people who may read its future messages.

The controller allowlist is local configuration. Group messages cannot add controllers or change Goose's permission policy.

## Build

Sonar Core uses a SQLite dependency that cannot currently be linked into the Goose binary. Build the isolated bridge and the feature-enabled CLI separately:

```bash
cargo build --release --manifest-path integrations/sonar-bridge/Cargo.toml
cargo build --release -p goose-cli --features sonar-gateway
```

Install `goose-sonar-bridge` on `PATH`, or pass its path when starting the gateway.

## Set up a group

1. Get the controller npub from the Sonar app identity you will use to send commands.
2. Initialize the bridge state and print the bridge npub with `goose-sonar-bridge --home /path/to/state identity`.
3. From the allowed controller identity, add the bridge npub to a Sonar group. The bridge accepts the invite and announces that the group is ready to pair.
4. Generate a pairing code for the Goose session.
5. Send that code to the Sonar group from an allowed controller.

```bash
goose gateway start sonar \
  --bridge-path /absolute/path/to/goose-sonar-bridge \
  --sonar-home /path/to/state \
  --controller npub1yourcontroller \
  --relay wss://relay.example.com

goose gateway pair sonar --session-id SESSION_ID
```

`--controller` and `--relay` are repeatable. At least one controller is required. If no relays are supplied, the bridge uses its default public relay set.

Omit `--session-id` to create a dedicated gateway session during pairing. With `--session-id`, the existing session keeps its model, provider, extensions, working directory, and mode. A session can be paired to only one Sonar group at a time.

## Sharing and revoking access

Invite another person to the Sonar group to share the transcript. They remain an observer until their npub is added with another `--controller` argument and the gateway is restarted. Remove their npub and restart to revoke command authority; remove them from the Sonar group as well to revoke access to future group traffic.

## State, backup, and replay behavior

The bridge stores its Nostr secret, database key, encrypted MLS database, and command ledger under the Goose data directory's `sonar-gateway` folder by default. Use `--sonar-home` to choose another location.

Back up the entire state directory while the gateway is stopped. Treat `config.json` as a secret. Relays alone may not be sufficient to recover the MLS session if the bridge identity or database key is lost.

Commands are persisted before execution. A command interrupted by a restart is never replayed automatically, because its tool side effects may already have occurred. The bridge marks it complete and asks the controller to resubmit if needed. Existing group history is checkpointed at first discovery and is not executed as new input.

## Limitations

- Transport uses Nostr relays and Marmot MLS, not BLE mesh.
- Commands and responses are plain group text in this first protocol version.
- Rich ACP events, steering, and cancellation are reserved for a later typed mobile control envelope.
- The bridge must remain running, and the host must stay awake and connected to configured relays.
- The completed-command ledger grows with received history and should be retained with the MLS state.
