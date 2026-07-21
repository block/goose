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

## Remote modes at a glance

All modes share the same foundation: one machine hosts the real agent, and the
wire protocol between machines is [ACP](/docs/guides/acp-clients). They differ in
what sits on the connecting side.

| Mode | Command | Connecting side is… | Use it to… |
|------|---------|--------------------|------------|
| Interactive | `roam connect` | a built-in terminal chat UI | drive a remote agent yourself |
| One-shot | `roam delegate` | a non-interactive caller | send one task, get the answer back |
| Bridge | `roam bridge` | a transparent ACP proxy | point *any* ACP client (goose desktop, Zed, an editor) at a remote agent |
| Resume | `roam share --session` | (host side) a replayed session | share an existing local session |
| Watch / co-drive | `roam share --scope observe\|attach` | one or more extra peers | let others listen in on, or help steer, one live session |

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

Configure your ACP client to run `goose roam bridge '<token>'` as its agent
command. It will speak ACP on the process's stdin/stdout, and every request is
forwarded to the remote agent.

Or bridge over a local TCP port, for a client that connects to an address:

```bash
goose roam bridge laptop --listen 127.0.0.1:8900
```

This accepts a single ACP connection on that address and proxies it to the
remote agent. Saved peer names work here too.

:::note
A bridge serves one client connection and inherits the invite's scope — the
remote host still runs the agent, imposes its own working directory, and
authorizes the connection.
:::

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
By default a shared agent grants **full control** — the connecting peer can run
the agent's tools, including its shell. Only share `control` with machines and
people you trust, and prefer `--allow-key` or `--pair` over bearer invites when
you can. Use a narrower `--scope` (below) when the peer doesn't need to drive.
:::

## Watching and co-driving a session

Several peers can attach to **one** live session at the same time. The invite's
scope decides what each may do:

| `--scope` | Can watch updates | Can prompt / steer | Answers tool-permission prompts |
|-----------|:-----------------:|:------------------:|:-------------------------------:|
| `control` (default) | ✅ | ✅ | ✅ |
| `attach` | ✅ | ✅ | ❌ |
| `observe` | ✅ | ❌ | ❌ |

Only one peer controls the session at a time (the host, or whoever it hands
control to); everyone else watches the same `session/update` stream live. So you
can keep a `control` invite for yourself and hand out `observe` invites for
others to **listen in**:

```bash
# Host keeps control for itself and mints a read-only invite to share around:
goose roam share --scope observe
```

Observers and attachers connect exactly like a controller — `goose roam connect
'<token>'` — but their prompts are refused (`observe`) and permission prompts are
never routed to them; those always go to the controller. This is the natural fit
for pairing, demos, or an over-the-shoulder review of a running agent.

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
  raw token.
- On macOS, if a session still appears to hang on connect, set
  `GOOSE_DISABLE_KEYRING=1` to skip the keychain entirely.
