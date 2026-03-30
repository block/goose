---
name: explore-desktop-app
description: Explore and test the running Goose desktop app using agent-browser. Use when the user wants to do UI exploratory testing or verify UI behaviours
---

# Explore App

## Steps

### Step 1 — Start & Connect

Run `just e2e-setup` to ensure goosed is built and dependencies are installed.

Use the `e2e-app` skill to start a fresh app instance. Note the session name and CDP port.

From ui/desktop, install agent-browser and connect:

```bash
cd ui/desktop
pnpm exec agent-browser install
pnpm exec agent-browser --session <session_name> connect <cdp_port>
```

Pass `--session <session_name>` to every agent-browser command.

Take an initial snapshot to confirm connection.

Create an output directory for this session: `/tmp/goose-e2e/explore/<datetime>/` (e.g. `/tmp/goose-e2e/explore/20260330-143201/`).
Save all screenshots and video to this directory.

### Step 2 — Plan

Based on the goal, break it down into specific checks.
Print them as a numbered list.

### Step 3 — Explore

For each check:
- Snapshot to see current state
- Navigate and interact
- Verify with wait --text, wait --fn, eval
- Screenshot interesting states and any issues
- Print ✓ or ✗ with a one-line summary per check

### Step 4 — Clean Replay with Recording

Now that the flow is known, do a clean pass for a concise video:

1. Restart the app using the `e2e-app` skill (fresh state)
2. Connect agent-browser to the new instance
3. Start video recording: `record restart /tmp/goose-e2e/explore/<datetime>/video.webm`
4. Replay only the successful steps from Step 3 — no debugging, no snapshots, just the actions
5. Stop video recording: `record stop`

### Step 5 — Report

Summary:
- What was tested
- What passed (✓)
- What failed (✗) with screenshots
- Any unexpected behavior observed
- Video and screenshot file paths

### Step 6 — Diagnose Failures

If any checks failed, diagnose them using Steps 3-5 from the `debug-e2e` skill (Diagnose, Classify, Report). Skip Steps 1-2 (Collect artifacts, Reproduce) — you already have the failure context from your exploration.

Rules:
- **Run `snapshot -i` before every interaction.** Refs like `@e37` are only valid for the snapshot that produced them. (This includes after `scrollintoview`.)
- **Verify after every interaction.** Use `wait --text` or `wait --fn` to confirm the expected result before moving on. Prefer `wait --text` over `wait <ms>`.
- Run `pnpm exec agent-browser --help` or `pnpm exec agent-browser <command> --help` to learn unfamiliar commands.


