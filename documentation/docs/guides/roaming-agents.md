---
sidebar_position: 95
title: Roaming Agents
sidebar_label: Roaming Agents
---

Roaming agents let you reach a running goose agent from another machine over a
peer-to-peer connection — no open ports, no VPN, no server to host. It's built
on [iroh](https://iroh.computer) (QUIC), so two machines can connect directly or
via a public relay, typically without any firewall changes.

Use it to drive your laptop's agent from another device, or hand a one-shot task
to a remote agent.

:::note
Roaming is an optional, experimental feature. It's available when goose is built
with the `roaming` feature (`cargo build -p goose-cli --features roaming`).
:::

## How it works

One machine **shares** its agent and prints a signed invite token. Another
machine **connects** with that token. The connection is authorized by the
invite, and the remote client speaks the same [ACP](/docs/guides/acp-clients)
protocol goose already uses — so the host runs the real agent (its tools,
files, and shell) while the connecting side is just a window onto it.

```
┌────────────┐   invite token   ┌────────────┐
│  Machine A │ ───────────────▶ │  Machine B │
│  roam share│                  │ roam connect│
│  (agent)   │ ◀═══ ACP over ══▶ │  (client)  │
└────────────┘   iroh + relay    └────────────┘
```

## Quick start

**On the host (machine A):**

```bash
goose roam share
```

This prints an invite token like `goose+roam://…` and keeps running. The agent
runs in the directory `share` was started in (override with `--cwd <dir>`). The
connecting side's own directory is always ignored — all work happens in the
host's directory.

**On the client (machine B):**

```bash
goose roam connect 'goose+roam://…'
```

You get an interactive prompt that drives the agent on machine A. Type a message
and press enter; `/quit` or Ctrl-D to leave.

## Resuming an existing session

Instead of starting fresh, you can share an existing local session — its
conversation history is replayed into the hosted agent, which runs in that
session's own working directory:

```bash
goose roam sessions                       # list local session ids
goose roam share --session <SESSION_ID>   # resume and share it
```

`--cwd` is ignored when `--session` is given (a resumed session keeps its own
directory). History replay reaches the hosted agent at share time, so a peer
that connects later sees new activity from the point it attaches rather than a
re-rendered transcript.

## One-shot delegation

To send a single task and get the answer back — no interactive session:

```bash
goose roam delegate 'goose+roam://…' "Summarize the last 5 commits in this repo."
```

The remote agent runs the task with its own tools and prints its final response.

## Saved peers

Save an invite under a nickname so you don't paste tokens each time:

```bash
goose roam peers save laptop 'goose+roam://…'
goose roam connect laptop
goose roam delegate laptop "run the tests and report failures"

goose roam peers list      # show saved peers (also: remove, rename)
goose roam connections     # show observed connections
goose roam id              # print this machine's endpoint ids
```

## Controlling who can connect

By default an invite is **bearer**: anyone holding the token can connect, until
it expires (`--ttl <seconds>`, default 1 hour). For tighter control:

| Goal | How |
|------|-----|
| Only a specific key may connect | `goose roam share --allow-key <client-id>` — use the connecting machine's **client** id from its `goose roam id`. Repeatable for multiple keys. |
| Pair once, then lock to that key | `goose roam share --pair` — the invite is single-use and pins the client key of whoever redeems it first |
| Limit the window | `goose roam share --ttl 300` |

:::warning
A shared agent grants **full control** — the connecting peer can run the agent's
tools, including its shell. Only share with machines and people you trust, and
prefer `--allow-key` or `--pair` over bearer invites when you can.
:::

## Letting the agent reach other agents

With the roaming feature enabled, goose can delegate to other agents itself. Ask
it to, and it can run `goose roam delegate <peer> "<task>"` via its shell — for
example, "delegate this to my work laptop and summarize what it finds." It sends
one self-contained task and relays the response.

## Notes and limits

- Peers connect directly when NAT hole-punching succeeds and fall back to
  iroh's public relays otherwise. The default relays are rate-limited and
  best-effort; self-hosting relays is possible but not covered here.
- `connect` and `delegate` accept either a saved peer name or a raw token.
- On macOS, if a session still appears to hang on connect, set
  `GOOSE_DISABLE_KEYRING=1` to skip the keychain entirely.
