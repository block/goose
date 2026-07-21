---
sidebar_position: 95
title: Roaming Agents
sidebar_label: Roaming Agents
---

Roaming agents let you reach a running goose agent from another machine over a
peer-to-peer connection — no open ports, no VPN, no server to host. It's built
on [iroh](https://iroh.computer) (QUIC), so two machines can connect directly or
via a public relay, typically without any firewall changes.

Use it to drive your laptop's agent from another device, hand a one-shot task to
a remote agent, or expose a remote agent to any local ACP client (like the goose
desktop app or an editor).

## The core idea: roaming is an ACP transport

Roaming does exactly one thing: it provides an **authenticated, peer-to-peer
[ACP](/docs/guides/acp-clients) transport**. The host runs goose's real ACP
server; the connecting side is an ACP client. That's it.

Everything that feels "session-shaped" is therefore just plain ACP that happens
to run over a roaming connection — not a bespoke roaming feature:

| You want to… | It's just ACP… | Command |
|--------------|----------------|---------|
| List the remote's sessions | `session/list` | `roam delegate <target> --list-sessions` |
| Continue a specific session | `session/load` | `roam delegate <target> --session <id> "…"` |
| Run a fresh one-shot task | `session/new` + `session/prompt` | `roam delegate <target> "…"` |
| Drive a remote agent from a real UI | full ACP surface | `roam bridge` → goose desktop, Zed, an editor |
| Quick interactive peek | a built-in REPL | `roam connect` |

Because the connection carries the full ACP surface, the connecting side can
enumerate, create, and resume the host's sessions with no roaming-specific
protocol. Higher-level behaviours (saved peers, co-driving one live session) sit
*above* the transport and are described below.

:::note
Roaming is an optional, experimental feature. It's available when goose is built
with the `roaming` feature (`cargo build -p goose-cli --features roaming`).
:::

## How it works: cards and mutual acceptance

Trust is a **mutual, public-key relationship** — like WireGuard or SSH
known-hosts, and deliberately infrastructural. Each node has one long-lived
identity and produces a **connection card**: a shareable string containing its
public key and how to reach it (relay URLs). *Nothing in a card is secret* —
possessing one grants no access.

To let a peer reach you, you each:

1. **Swap cards** (`goose roam id` prints yours; send it over any channel).
2. **Accept the other's key** (`goose roam peers accept …`).

A connection only succeeds when the **host has accepted the dialer's key**. The
transport (iroh QUIC-TLS) proves each side holds the private key for the identity
in its card, so no one can impersonate a key, and a leaked card lets no one in.
There is no bearer token that grants access by possession.

```
┌────────────┐    swap cards     ┌────────────┐
│  Machine A │ ◀───────────────▶ │  Machine B │
│            │  each accepts the │            │
│  roam share│  other's key      │ roam connect│
│  (agent)   │ ◀═══ ACP over ══▶ │ /delegate/ │
└────────────┘   iroh + relay    │  bridge    │
                                 └────────────┘
```

By default each connecting client gets its **own** agent and drives its **own**
sessions over the full ACP surface. (Co-driving *one shared* live session is a
separate, opt-in mode — see [Watching and co-driving](#watching-and-co-driving-a-session).)

## Quick start

Say machine B wants to drive machine A's agent. Both run `goose roam id` and send
each other the card it prints. Then:

**On machine A (the host):** add B's card and accept its key.

```bash
goose roam peers add 'goose+roam://…B…' laptop-b
goose roam peers accept laptop-b          # grants control by default
goose roam share                          # serve to accepted peers
```

`share` keeps running and prints A's card too. The agent runs in the directory
`share` was started in (override with `--cwd <dir>`); the connecting side's own
directory is always ignored.

**On machine B (the client):** add A's card and connect.

```bash
goose roam peers add 'goose+roam://…A…' laptop-a
goose roam connect laptop-a
```

You get an interactive prompt that drives the agent on machine A. Type a message
and press enter; `/quit` or Ctrl-D to leave.

`connect` is a minimal built-in chat loop — handy for a quick sanity check. For
real work, prefer `bridge` (drive the remote agent from a full ACP client) or
`delegate` (scriptable one-shot tasks).

:::tip
Compare the short **fingerprint** shown by `roam id` / `peers accept` out of band
(e.g. read it aloud) to be sure you accepted the key you meant to.
:::

## One-shot delegation

To send a single task and get the answer back — no interactive session:

```bash
goose roam delegate 'goose+roam://…' "Summarize the last 5 commits in this repo."
```

The remote agent runs the task with its own tools and prints its final response.
`delegate` is a thin ACP client, so it can also work with the remote's existing
sessions — all plain ACP under the hood:

```bash
# List the remote agent's sessions (session/list)
goose roam delegate 'goose+roam://…' --list-sessions

# Continue a specific session instead of starting fresh (session/load)
goose roam delegate 'goose+roam://…' --session <SESSION_ID> "Now fix the first failure."
```

## Bridging to any ACP client

`connect` and `delegate` embed goose's own ACP client. `bridge` does the
opposite: it exposes a remote agent as a **local ACP endpoint**, so any ACP
client — the goose desktop app, [Zed](/docs/guides/acp-clients), or another
editor — can drive it as if it were running locally. It runs no UI and no agent
of its own; it transparently proxies ACP bytes between the local client and the
remote agent.

Bridge over stdio (the default — for a client that launches goose as a
subprocess):

```bash
goose roam bridge 'goose+roam://…'
```

Configure your ACP client to run `goose roam bridge '<card>'` as its agent
command. It will speak ACP on the process's stdin/stdout, and every request is
forwarded to the remote agent.

Or bridge over a local TCP port, for a client that connects to an address:

```bash
goose roam bridge laptop --listen 127.0.0.1:8900
```

This accepts a single ACP connection on that address and proxies it to the
remote agent. Saved peer names work here too.

Because a default `share` serves the full ACP surface, a bridged client gets
everything — it can list, create, and load the host's sessions, not just a
single pre-selected one.

:::note
A bridge serves one client connection. The remote host still runs the agent,
imposes its own working directory, and authorizes the connection.
:::

## Saved peers

Save a peer's card under a nickname so you don't paste cards each time. A saved
card is just an address-book entry — it does **not** let that peer connect to
you (use `peers accept` for that):

```bash
goose roam peers add 'goose+roam://…' laptop   # save to the address book
goose roam connect laptop
goose roam delegate laptop "run the tests and report failures"

goose roam peers list      # show saved peers + which keys you accept
goose roam connections     # show observed connections
goose roam id              # print this node's connection card
```

## Controlling who can connect

Access is granted **only** by accepting a peer's public key — there is no bearer
token that works by possession. You accept a peer (by saved name or inline card)
and choose what it may do:

```bash
goose roam peers accept laptop                    # grant control (default)
goose roam peers accept watcher --scope observe   # read-only (co-drive only, see below)
goose roam peers accept 'goose+roam://…'          # accept an inline card (also saves it)

goose roam peers list                             # see who is accepted, and their scope
goose roam peers revoke laptop                    # stop accepting (name, card, or raw id)
```

Acceptance is **durable** and **live**: it is stored on disk, and a running
`share` re-reads it on each connection — so `accept`/`revoke` take effect against
a live share without a restart. Revoking takes effect on the next connection (an
already-open session continues until it disconnects).

:::note
`attach`/`observe` scopes only apply when co-driving one live session
(`roam share --session`, below). A **default** `share` serves the full ACP
surface, which only makes sense for a `control` peer — it therefore *refuses* an
`attach`/`observe` peer, directing it to co-drive instead.
:::

Because trust is keyed on the peer's public key and the transport authenticates
that key cryptographically, a card can be shared over any channel — it is not a
secret, and a leaked card lets no one in.

:::warning
Accepting a peer with `control` grants **full control** — the peer can run the
agent's tools, including its shell. Only accept machines and people you trust,
and verify the fingerprint out of band. For a read-only or steer-only peer, grant
a narrower `--scope` (see below), which only takes effect when co-driving a
session.
:::

## Watching and co-driving a session

The default `share` gives each accepted peer its own agent. To instead let
several peers attach to **one** live session at the same time — for pairing,
demos, or an over-the-shoulder review — co-drive a specific session with
`--session`. The scope you granted each peer (`peers accept --scope`) then decides
what it may do:

```bash
goose roam sessions                          # list local session ids
goose roam peers accept watcher --scope observe   # grant a peer read-only
goose roam share --session <SESSION_ID>      # co-drive that live session
```

| Granted scope | Can watch updates | Can prompt / steer | Answers tool-permission prompts |
|-----------|:-----------------:|:------------------:|:-------------------------------:|
| `control` | ✅ | ✅ | ✅ |
| `attach` | ✅ | ✅ | ❌ |
| `observe` | ✅ | ❌ | ❌ |

Only one peer controls the session at a time (the host, or whoever it hands
control to); everyone else watches the same `session/update` stream live. So you
can accept yourself/one peer with `control` and others with `observe` so they can
**listen in**. Observers and attachers connect exactly like a controller —
`goose roam connect '<card>'` — but their prompts are refused (`observe`) and
permission prompts are never routed to them; those always go to the controller.

Scope only shapes behaviour in this co-drive mode; a default `share` (no
`--session`) lets each accepted peer drive its own sessions.

:::note
Co-drive resumes the session's history into the hosted agent at share time, and
runs in that session's own working directory (`--cwd` is ignored). A peer that
connects later sees new activity from the point it attaches.
:::

## Letting the agent reach other agents

With the roaming feature enabled, goose can delegate to other agents itself. Ask
it to, and it can run `goose roam delegate <peer> "<task>"` via its shell — for
example, "delegate this to my work laptop and summarize what it finds." It sends
one self-contained task and relays the response.

Because saved peers are just an address book, the agent can discover what
remotes it has available (`goose roam peers list`) and route work to the right
one — e.g. run a build on the machine that has the toolchain, then bring the
result back. Each delegation is a self-contained task with a bounded response,
so this composes into multi-machine workflows without any shared state.

## Notes and limits

- Peers connect directly when NAT hole-punching succeeds and fall back to
  iroh's public relays otherwise. The default relays are rate-limited and
  best-effort; self-hosting relays is possible but not covered here.
- `connect`, `delegate`, and `bridge` all accept either a saved peer name or a
  raw `goose+roam://…` card. Remember the peer must also have accepted your key.
- On macOS, if a session still appears to hang on connect, set
  `GOOSE_DISABLE_KEYRING=1` to skip the keychain entirely.
