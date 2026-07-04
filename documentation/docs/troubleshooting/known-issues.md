---
title: Known Issues
sidebar_label: Known Issues
description: Comprehensive troubleshooting guide for common gosling problems with step-by-step solutions.
---

gosling, like any system, may run into occasional issues. This guide provides solutions for common problems.

:::tip Need help with an issue not listed here?
Our [Discord community](https://discord.gg/gosling-oss) is here to help! For the fastest support, consider generating a [diagnostic report](/docs/troubleshooting/diagnostics-and-reporting) - it helps us understand your setup quickly.
:::

### gosling Edits Files
gosling can and will edit files as part of its workflow. To avoid losing personal changes, use version control to stage your personal edits. Leave gosling edits unstaged until reviewed. Consider separate commits for gosling's edits so you can easily revert them if needed.

---

### Interrupting gosling
If gosling is heading in the wrong direction or gets stuck, you can [interrupt it](/docs/guides/sessions/in-session-actions#interrupt-task) to correct its actions or provide additional information.

---

### Stuck in a Loop or Unresponsive
In rare cases, gosling may enter a "doom spiral" or become unresponsive during a long session. This is often resolved by ending the current session, and starting a new session.

1. Hold down `Ctrl+C` to cancel
2. Start a new session:
  ```sh
  gosling session
  ```
:::tip
For particularly large or complex tasks, consider breaking them into smaller sessions.
:::

---

### Preventing Long-Running Commands

If you use gosling CLI and work with web development projects, you may encounter commands that cause gosling to hang indefinitely. Commands like `npm run dev`, `python -m http.server`, or `webpack serve` start development servers that never exit on their own.

You can prevent these issues by customizing your shell to handle these commands differently when gosling runs them. See [Customizing Shell Behavior](/docs/guides/environment-variables#customizing-shell-behavior) for details on using the `GOSLING_TERMINAL` environment variable.

---



### Context Length Exceeded Error

This error occurs when the input provided to gosling exceeds the maximum token limit of the LLM being used. To resolve this, try breaking down your input into smaller parts. You can also use [`.goslinghints`][goslinghints] as a way to provide gosling with detailed context and use [message queues](/docs/guides/sessions/in-session-actions#queue-messages) in gosling Desktop.

---

### Using Ollama Provider

Ollama provides local LLMs, which means you must first [download Ollama and run a model](/docs/getting-started/providers#local-llms) before attempting to use this provider with gosling. If you do not have the model downloaded, you'll run into the following error:

> ExecutionError("error sending request for url (http://localhost:11434/v1/chat/completions)")


Another thing to note is that the DeepSeek models do not support tool calling, so all gosling [extensions must be disabled](/docs/getting-started/using-extensions#enablingdisabling-extensions) to use one of these models. Unfortunately, without the use of tools, there is not much gosling will be able to do autonomously if using DeepSeek. However, Ollama's other models such as `qwen2.5` do support tool calling and can be used with gosling extensions.

---

### Handling Rate Limit Errors
gosling may encounter a `429 error` (rate limit exceeded) when interacting with LLM providers. The recommended solution is to use a provider that provides built-in rate limiting. See [Handling LLM Rate Limits][handling-rate-limits] for more info.

---

### Hermit Errors

If you see an issue installing an extension in the app that says "hermit:fatal", you may need to reset your hermit cache. We use
a copy of hermit to ensure npx and uvx are consistently available. If you have already used an older version of hermit, you may
need to cleanup the cache - on Mac this cache is at

```
sudo rm -rf ~/Library/Caches/hermit
```

---

### API Errors

Users may run into an error like the one below when there are issues with their LLM API tokens, such as running out of credits or incorrect configuration:

```sh
Traceback (most recent call last):
  File "/Users/admin/.local/pipx/venvs/gosling-ai/lib/python3.13/site-packages/exchange/providers/utils.py",
line 30, in raise_for_status
    response.raise_for_status()
    ~~~~~~~~~~~~~~~~~~~~~~~~~^^
  File "/Users/admin/.local/pipx/venvs/gosling-ai/lib/python3.13/site-packages/httpx/_models.py",
line 829, in raise_for_status
    raise HTTPStatusError(message, request=request, response=self)
httpx.HTTPStatusError: Client error '404 Not Found' for url
'https://api.openai.com/v1/chat/completions'

...
```
This error typically occurs when LLM API credits are exhausted or your API key is invalid. To resolve this issue:

1. Check Your API Credits:
    - Log into your LLM provider's dashboard
    - Verify that you have enough credits. If not, refill them
2. Verify API Key:
    - Run the following command to reconfigure your API key:
    ```sh
    gosling configure
    ```
For detailed steps on updating your LLM provider, refer to the [Installation][installation] Guide.

---

### GitHub Copilot Provider Configuration

If you encounter errors when configuring GitHub Copilot as your provider, try these workarounds for common scenarios.

#### Container and Keyring Issues

gosling tries to use the system keyring (typically via Secret Service over DBus) to securely store your GitHub Copilot token. In containerized or headless environments, DBus and/or a desktop keyring service may not be available (and some setups fail with X11-based DBus autolaunch errors), so keyring access can fail.

If you're running gosling in Docker containers or Linux environments without keyring support, gosling may automatically fall back to [file-based secret storage](#keyring-cannot-be-accessed-automatic-fallback).

If you still receive authentication/keyring errors like the following, gosling was not able to fall back automatically. This can happen if gosling doesn’t recognize the keyring failure as an access issue, or if the error is unrelated to keyring access.
```
Failed to save token: Failed to access keyring: <error message>
```

To resolve this (or to proactively force file-based secret storage), set `GOSLING_DISABLE_KEYRING` to any value. This example sets it only while running `gosling configure`:

```bash
GOSLING_DISABLE_KEYRING=1 gosling configure
```

If you prefer not to store secrets in `secrets.yaml`, set the token via environment variables instead.

See [Keychain/Keyring Errors](#keychainkeyring-errors) for more details on keyring alternatives.

---

### Uninstall gosling or Remove Cached Data

You may need to uninstall gosling or clear existing data before re-installing. gosling stores data in different locations depending on your operating system. Secrets, such as API keys, are stored in the system keychain/keyring by default (or in `secrets.yaml` when file-based secret storage is in use).

#### macOS

**Data Locations**

- **Logs and Config**: `~/.config/gosling`
- **Application Data**: `~/Library/Application Support/Gosling`
- **Secrets**: macOS Keychain (credential named "gosling").

#### Removal Steps

1. Stop any copies of gosling running (CLI or GUI)

  - Consider confirming you've stopped them all via Activity Monitor

2. Open Keychain Access and delete the credential called "gosling", which contains all secrets stored by gosling
3. Remove data directories:

```
rm -rf ~/.config/gosling
rm -rf ~/Library/Application\ Support/gosling
```
4. Delete the "gosling" app from your Applications folder (if using gosling Desktop).

#### Linux
**Data Locations**

- **Data/Sessions**: `~/.local/share/gosling/`
- **Logs**: `~/.local/state/gosling/`
- **Config**: `~/.config/gosling/`
- **Secrets**: System keyring (if available)

#### Removal Steps

- Stop any copies of gosling running (CLI or GUI)
- Clear secrets from your system keyring (if applicable)
- Remove data directories:

```
rm -rf ~/.local/share/gosling/
rm -rf ~/.local/state/gosling/
rm -rf ~/.config/gosling/
```
#### Windows

**Data Locations**
- **Configuration and Data**: `%APPDATA%\Block\gosling\`
- **Local Application Data**: `%LOCALAPPDATA%\Block\gosling\`
- **Secrets**: Windows Credential Manager

#### Removal Steps

1. Stop any copies of gosling running (CLI or GUI)

  - Check Task Manager to confirm all instances are closed

2. Open Windows Credential Manager and delete credentials related to "gosling"
3. Remove data directories:
```
rmdir /s /q "%APPDATA%\Block\gosling"
rmdir /s /q "%LOCALAPPDATA%\Block\gosling"
```
4. Uninstall the gosling Desktop app from Settings > Apps (if applicable)

> After this cleanup, if you are looking to try out a fresh install of gosling, you can now start from the usual install instructions.
---

### Keychain/Keyring Errors

gosling tries to use the system keyring (keychain on macOS) to store secrets by default. In environments where the keyring cannot be accessed (or there is no keyring support), you may see an error like:

```bash
Error Failed to access secure storage (keyring): Platform secure storage failure: DBus error: The name org.freedesktop.secrets was not provided by any .service files
Please check your system keychain and run 'gosling configure' again.
If your system is unable to use the keyring, please try setting secret key(s) via environment variables.
```

In some cases, gosling may automatically fall back to file-based secret storage. See [Keyring Cannot Be Accessed (Automatic Fallback)](#keyring-cannot-be-accessed-automatic-fallback).

If you still receive keyring errors, use one of the following options:

- **Set provider-specific environment variable(s)**, which can be found at [Supported LLM Providers][configure-llm-provider].
  - For the duration of your session: `export GOOGLE_API_KEY=$YOUR_KEY_HERE`
  - To persist across new shells: add it to your `~/.bashrc` or `~/.zshrc` (or equivalents)

    Then select the `No` option when prompted to save the value to your keyring.

  ```bash
  $ gosling configure

  Welcome to gosling! Let's get you set up with a provider.
    you can rerun this command later to update your configuration

  ┌   gosling-configure
  │
  ◇  Which model provider should we use?
  │  Google Gemini
  │
  ◇  GOOGLE_API_KEY is set via environment variable
  │
  ◇  Would you like to save this value to your keyring?
  │  No
  │
  ◇  Enter a model from that provider:
  │  gemini-2.0-flash-exp
  ```

- **If you need to disable the keyring**, set `GOSLING_DISABLE_KEYRING` to any value to force file-based secret storage. The actual value doesn't matter, only whether the variable is set.

  This example sets it only while running `gosling configure`:

  ```bash
  GOSLING_DISABLE_KEYRING=1 gosling configure
  ```

When the keyring is disabled (or cannot be accessed and gosling falls back to file-based secret storage), secrets are stored here:

- macOS/Linux: `~/.config/gosling/secrets.yaml`
- Windows: `%APPDATA%\Block\gosling\config\secrets.yaml`

See [Configuration Files](/docs/guides/config-files) for details.

---

### Keyring Cannot Be Accessed (Automatic Fallback)

gosling uses the system keyring (keychain on macOS) to store secrets by default. If the keyring cannot be accessed, gosling may fall back to file-based secret storage.

In this case, you may see a warning like the following in logs:

```text
Keyring unavailable. Using file storage for secrets.
```

Automatic fallback only applies to the current gosling process. When you start a new session, gosling will try to use the keyring again.

If you need to force file-based secret storage (for example, in containers or headless environments), set `GOSLING_DISABLE_KEYRING` to any value:

```bash
GOSLING_DISABLE_KEYRING=1 gosling configure
```

See [Configuration Files](/docs/guides/config-files) for `secrets.yaml` details.

### Package Runners

Many of the external extensions require a package runner. For example, if you run into an error like this one:

```
Failed to start extension `{extension name}`: Could not run extension command (`{extension command}`): No such file or directory (os error 2)
Please check extension configuration for {extension name}.
```

... it signals that the extension may not have been installed and you need the package runner in order to do so.

An example is the GitHub extension whose command is `npx -y @modelcontextprotocol/server-github`. You'd need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.

---

### Node.js Extensions Not Activating on Windows

If you encounter the error `Node.js installer script not found` when trying to activate Node.js-based extensions on Windows, this is likely due to gosling not finding Node.js in the expected system path.

#### Symptoms:
- Node.js is installed and working (verified with `node -v` and `npm -v`)
- Other extensions (like Python-based ones) work fine
- Error occurs specifically when activating Node.js extensions

#### Solution:
This issue typically occurs when Node.js is installed in a non-standard location. gosling expects to find Node.js in `C:\Program Files\nodejs\`, but it may be installed elsewhere (e.g., `D:\Program Files\nodejs\`).

1. **Check your Node.js installation path:**
   ```powershell
   where.exe node
   ```

2. **If Node.js is not in `C:\Program Files\nodejs\`, create a symbolic link:**
   - Open PowerShell as Administrator
   - Create a symbolic link to redirect gosling to your actual Node.js installation:
   ```powershell
   mklink /D "C:\Program Files\nodejs" "D:\Program Files\nodejs"
   ```
   (Replace `D:\Program Files\nodejs` with your actual Node.js installation path)

3. **Restart gosling** and try activating the extension again.

This creates a symbolic link that allows gosling to find Node.js in the expected location while keeping your actual installation intact.

---

### Malicious Package Detected 

If you see an error about a "blocked malicious package" when trying to use an extension, it means the extension was blocked because malware was detected in a package used by the extension. The error message will contain details about the package, for example:

```
Blocked malicious package: package-name@1.0.0 (npm). OSV MAL advisories: MAL-2024-1234
```

Steps to resolve:
1. **Find an alternative**: Look for similar extensions in the [extensions directory][extensions-directory] or [PulseMCP](https://www.pulsemcp.com/servers)
2. **Optional verification**: Verify the source of the blocked extension or the package name/publisher
3. **Report false positives**: If you believe this is an error, please [open an issue](https://github.com/repo-makeover/gosling/issues)

This security check only applies to locally-executed external extensions that use PyPI (`uvx`) or NPM (`npx`). The check uses real-time data from the OSV database; if the security service is unavailable, extensions will still install normally.

As a best practice, only install extensions from trusted, official sources.

---

### macOS Permission Issues

If you encounter an issue where the gosling Desktop app shows no window on launch, it may be due to file and folder permissions. This typically happens because gosling needs read and write access to the `~/.config` directory to create its log directory and file. 
Similarly, if tools fail to create files or directories during use, it could be caused by the same permission issue.

#### How to Check and Fix Permissions:

1. Open Terminal.
2. Run the following command to check the current permissions for ~/.config:
  ```sh
  ls -ld ~/.config
  ```
**Example output:**
  ```sh
  drwx------  7 yourusername  staff  224 Jan 15 12:00 /Users/yourusername/.config
  ```
`rwx` indicates you have read (r), write (w), and execute (x) permissions for your user. If you do not see `rwx` for your user, follow the steps below.

#### How to Grant Read and Write Permissions:

1. To add the correct permissions, run the following commands:
    ```sh
    chmod u+rw ~/.config
    ```
    If the ~/.config directory does not exist, create it and then assign permissions:
      ```sh
      mkdir -p ~/.config
      chmod u+rw ~/.config
      ```
2. Verify the change:
    ```sh
    ls -ld ~/.config
    ```

If you still experience issues after fixing permissions, try launching gosling with superuser (admin) privileges:
```sh
sudo /Applications/Gosling.app/Contents/MacOS/Gosling
```

:::note
Running gosling with sudo may create files owned by root, which could lead to further permission issues. Use this as a troubleshooting step rather than a permanent fix.
:::

#### Update permission in System Settings (macOs)
1. Go to `System Settings` -> `Privacy & Security` -> `Files & Folders`
2. Grant gosling access

---

### Connection Error with Ollama Provider on WSL

If you encounter an error like this when setting up Ollama as the provider in gosling:
    ```
    Execution error: error sending request for url (http://localhost:11434/v1/chat/completions)
    ```
This likely means that the local host address is not accessible from WSL.
1. Check if the service is running:
    ```
    curl http://localhost:11434/api/tags
    ```
    If you receive a `failed to connect` error, it’s possible that WSL is using a different IP for localhost. In that case, run the following command to find the correct IP address for WSL:
    ```
    ip route show | grep -i default | awk '{ print $3 }'
    ```
2. Once you get the IP address, use it in your gosling configuration instead of localhost. For example:
    ```
    http://172.24.80.1:11434
    ```
    
If you still encounter a `failed to connect` error, you can try using WSL's [Mirrored Networking](https://learn.microsoft.com/en-us/windows/wsl/networking#mirrored-mode-networking) setting if you using Windows 11 22H2 or higher 

---

### Corporate Proxy or Firewall Issues

If you're behind a corporate proxy or firewall and gosling cannot connect to your LLM provider, you may see errors like:

```
error sending request for url (https://api.openai.com/...)
failed to connect to api.openai.com
```

gosling supports HTTP/HTTPS proxy configuration through standard environment variables and system proxy settings. Environment variables take precedence when both are configured.

**Solution:**

1. **Configure [proxy environment variables](/docs/guides/environment-variables#network-configuration):**
   ```bash
   export HTTPS_PROXY="http://proxy.company.com:8080"
   export NO_PROXY="localhost,127.0.0.1,.internal"
   ```

2. **Or use system proxy settings:**
   - **macOS**: System Settings → Network → [select connection] → Details → Proxies
   - **Windows**: Settings → Network & Internet → Proxy

3. **Restart gosling** after configuring proxy settings

If you continue to experience connection issues, verify:
- Your proxy URL and port are correct
- You have authentication credentials if required (format: `http://username:password@proxy:port`)
- Your proxy allows HTTPS connections to your LLM provider's domain

---

### Airgapped/Offline Environment Issues

If you're working in an airgapped, offline, or corporate-restricted environment, you may encounter issues where MCP server extensions fail to activate or download their runtime dependencies.

#### Symptoms:
- Extensions fail to activate with error messages about missing runtime environments
- Errors containing "hermit:fatal" or failed internet downloads
- Extensions work on personal machines but fail in corporate/restricted networks
- Error messages like: `Failed to start extension: Could not run extension command`

#### Solution:
gosling Desktop uses **"shims"** (packaged versions of `npx` and `uvx`) that automatically download runtime environments via Hermit. In restricted networks, these downloads fail.

**Workaround - Use Custom Command Names:**

1. **Create alternatively named versions of package runners on your system:**
   ```bash
   # For uvx (Python packages)
   ln -s /usr/local/bin/uvx /usr/local/bin/runuv
   
   # For npx (Node.js packages)  
   ln -s /usr/local/bin/npx /usr/local/bin/runnpx
   ```

2. **Update your MCP server configurations to use the custom names:**
   
   Instead of:
   ```yaml
   extensions:
     example:
       cmd: uvx
       args: [mcp-server-example]
   ```
   
   Use:
   ```yaml
   extensions:
     example:
       cmd: runuv  # This bypasses gosling's shims
       args: [mcp-server-example]
   ```

3. **Why this works:** gosling only replaces known command names (`npx`, `uvx`, `jbang`, etc.) with its packaged shims. Custom names are passed through unchanged to your system's actual executables.

4. **Require more changes**: In a corporate proxy environment or airgapped environment where the above doesn't work, it is recommended that you customize and package up gosling Desktop with shims/config that will work given the network constraints you have (for example, TLS certificate limitations, [proxy configuration](/docs/guides/environment-variables#network-configuration), inability to download required content etc).

---
### Need Further Help? 

Still running into issues? We're here to help! Join our [Discord Community][discord] where the gosling team and community members are happy to assist.

:::tip
If you can share a [diagnostic report](/docs/troubleshooting/diagnostics-and-reporting#diagnostics-system) along with your question, it helps us understand your setup and provide more targeted solutions.
:::



[handling-rate-limits]: /docs/guides/handling-llm-rate-limits-with-gosling
[installation]: /docs/getting-started/installation
[discord]: https://discord.gg/gosling-oss
[goslinghints]: /docs/guides/context-engineering/using-goslinghints
[configure-llm-provider]: /docs/getting-started/providers
[extensions-directory]: /extensions
