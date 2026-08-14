---
name: browser-use
description: Control a real Chrome browser to navigate pages, click, fill forms, run JavaScript, and capture screenshots. Use for tasks that require a live browser session — logging in, interacting with JS-heavy pages, or visual verification.
---

## Requirements

- `uv` must be installed (`curl -LsSf https://astral.sh/uv/install.sh | sh`)
- Google Chrome must be installed
- **One-time setup:** launch Chrome with remote debugging enabled (run once, keep the window open):
  ```bash
  # macOS
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --remote-debugging-port=9222 \
    --user-data-dir=/tmp/goose-chrome-profile &
  # Linux
  google-chrome --remote-debugging-port=9222 --user-data-dir=/tmp/goose-chrome-profile &
  ```
  Confirm it is ready: `curl -s http://localhost:9222/json/version | head -3`

## Running browser commands

Pipe Python to `uvx browser-use`. The following helpers are pre-imported and available without any import:
- `new_tab(url)` — open URL in a new tab and wait for load
- `js(code)` — run JavaScript in the active tab, returns the result
- `fill_input(selector, text)` — fill a CSS/XPath selector with text
- `capture_screenshot()` — take a screenshot, returns the image path
- `cdp(command, params)` — send a raw Chrome DevTools Protocol command

```bash
cat <<'PYEOF' | uvx browser-use
new_tab("https://example.com")
title = js("document.title")
print("Page title:", title)
path = capture_screenshot()
print("Screenshot saved to:", path)
PYEOF
```

## State model

- **Browser session persists** across multiple `uvx browser-use` runs while Chrome stays open.
- **Python variables do not persist** between runs — each invocation starts a fresh Python interpreter.
- To pass data between runs, write to a file or use `js("window.myVar = 'value'")` to store state in the page.

## Safety rules

- Stop immediately at any login wall or CAPTCHA. Report the URL to the user; do not attempt to bypass it.
- Use the dedicated `/tmp/goose-chrome-profile` user-data-dir — do not connect to the user's default Chrome profile.
- Do not store or log passwords, tokens, or any credentials that appear on the page.
- When a task involves a site the user has not explicitly authorised, confirm before proceeding.

## Troubleshooting

| Symptom | Fix |
|---|---|
| `Connection refused` on port 9222 | Chrome is not running with `--remote-debugging-port=9222` — run the setup command above |
| Page never loads | Add a sleep after navigation: `new_tab(url); import time; time.sleep(3)` |
| `uvx` not found | Install uv: `curl -LsSf https://astral.sh/uv/install.sh \| sh` |
