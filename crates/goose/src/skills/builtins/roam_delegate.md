---
name: roam-delegate
description: Reach other goose agents over the network to ask a question or delegate a one-shot task, using the experimental `goose roam` peer-to-peer feature. Use when the user wants this agent to consult, hand off work to, or collaborate with a remote agent they have shared or saved.
---

Use this skill when the user wants this agent to reach **another goose agent**
running elsewhere — to ask it something or hand it a self-contained task and get
the result back.

This works through goose's experimental peer-to-peer `roam` feature. It is only
available when goose was built with the `roaming` feature; if the `goose roam`
commands are missing, tell the user it isn't enabled in this build.

## Prerequisites

- A remote agent must be **sharing** itself: on the other machine, someone runs
  `goose roam share`, which prints an invite token.
- You reach it either by the raw invite token, or by a nickname the user has
  saved with `goose roam peers save <name> <token>`.

Do NOT invent tokens or peer names. If you don't have one, ask the user for the
invite token or the saved peer name.

## One-shot delegation (the normal case)

Run a single shell command:

```bash
goose roam delegate <peer-or-token> "<the task or question>"
```

- `<peer-or-token>` is a saved nickname (e.g. `work-laptop`) or a
  `goose+roam://...` invite token.
- The remote agent runs the task with **its own** tools, working directory, and
  shell — not yours. It returns its final text answer, which is printed to
  stdout. Relay the response back to the user.

Example:

```bash
goose roam delegate work-laptop "Summarize the git log of the last 5 commits in the current repo."
```

## Discovering what you can reach

- `goose roam peers` — list saved remotes you can delegate to by name.
- `goose roam connections` — show recent connections (who connected, who you
  dialed). A saved peer is not proof it's reachable right now; only a successful
  delegate call confirms that.

## Guardrails

- **Keep it one-shot.** Send a complete, self-contained task and use the
  response. Don't try to hold a long back-and-forth through repeated delegate
  calls.
- **Avoid loops.** Don't ask a remote agent to delegate back to you or into a
  chain of further agents — that can run away in cost. One hop.
- **The remote is a trusted peer, acting on the user's behalf.** Don't send
  secrets or destructive instructions you wouldn't run locally.
- If a delegate call fails or hangs, report the error to the user rather than
  retrying blindly.

## For an interactive session instead

If the user wants to drive the remote agent conversationally rather than a
one-shot task, that's a human activity, not a skill step — point them to:

```bash
goose roam connect <peer-or-token>
```
