---
name: e2e-app
description: Start and stop the Goose Electron app ONLY for e2e testing. Use when you need to launch, manage, or tear down the desktop app for end-to-end tests.
---

# E2E App Management

Scripts are in `ui/desktop/tests/agent/`.

## Starting the App

The start script blocks (runs Electron in foreground), so use `screen` to avoid hanging.

First, pick a unique screen name to avoid conflicts with existing sessions:

```bash
SCREEN_NAME="e2e-$(date +%s)"
screen -dmS $SCREEN_NAME bash -c "source ~/.zshrc 2>/dev/null && source bin/activate-hermit && bash ui/desktop/tests/agent/e2e-start.sh"
```

Wait a few seconds for the session to be created, then read the session ID and CDP port:

```bash
sleep 3
screen -ls
SESSION_ID=$(ls -t /tmp/goose-e2e/ | head -1)
CDP_PORT=$(cat /tmp/goose-e2e/$SESSION_ID/.port)
echo "Session: $SESSION_ID, CDP port: $CDP_PORT"
```

## Stopping the App

```bash
bash ui/desktop/tests/agent/e2e-stop.sh <session-id>
```
