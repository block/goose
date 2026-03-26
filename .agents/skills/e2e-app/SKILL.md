---
name: e2e-app
description: Start and stop the Goose Electron app ONLY for e2e testing. Use when you need to launch, manage, or tear down the desktop app for end-to-end tests.
---

# E2E App Management

Scripts are in `ui/desktop/tests/e2e-tests/scripts/`.

## Starting the App

The start script blocks (runs Electron in foreground), so use `screen` to background it.
The script self-activates hermit for `pnpm`/`node`, but needs `ANTHROPIC_API_KEY` in the environment.

```bash
TEST_SESSION_NAME=$(date +"%y%m%d-%H%M%S")
screen -dmS $TEST_SESSION_NAME bash -c "bash ui/desktop/tests/e2e-tests/scripts/e2e-start.sh $TEST_SESSION_NAME"
```

Then wait for the port file and verify the app is listening:

```bash
# Wait for port file and app to be ready (up to 30s)
for i in $(seq 1 30); do
  if [[ -f "/tmp/goose-e2e/sessions/$TEST_SESSION_NAME/.port" ]]; then
    CDP_PORT=$(cat /tmp/goose-e2e/sessions/$TEST_SESSION_NAME/.port)
    if lsof -i :"$CDP_PORT" &>/dev/null; then
      echo "App ready — Test session name: $TEST_SESSION_NAME, CDP port: $CDP_PORT"
      break
    fi
  fi
  sleep 1
done
```

If the app doesn't start, check the screen log:
```bash
screen -ls                    # verify screen session exists
screen -r $TEST_SESSION_NAME  # attach to see errors (Ctrl-A D to detach)
```

Common startup failures:
- `ANTHROPIC_API_KEY must be set` — create `~/.config/goose/e2e.env` with your provider config (see e2e-start.sh)
- `pnpm: not found` — hermit activation failed; the script does this automatically now
- Screen session dies immediately — check `screen -ls`; if no session, run the script directly to see errors

## Stopping the App

```bash
bash ui/desktop/tests/e2e-tests/scripts/e2e-stop.sh <session-name>
```
