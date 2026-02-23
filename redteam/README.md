# Red Team Goose Distribution

This is a custom distribution of Goose focused on Network Security and Red Team operations.
It is based on Kali Linux and comes pre-loaded with essential security tools.

## Included Tools

*   **Nmap**: Network exploration and security auditing.
*   **Exploit-DB (searchsploit)**: Searchable archive of exploits.
*   **Metasploit Framework**: Penetration testing platform.
*   **Wireshark (tshark)**: Network protocol analyzer.
*   **Burp Suite**: Web vulnerability scanner (Community Edition).
*   **Simple Port Scanner**: Python-based port scanner.

## Building the Docker Image

To build the Red Team Goose image, run the following command from the **root** of the repository:

```bash
docker build -f redteam/Dockerfile -t goose-redteam .
```

This uses a multi-stage build to compile the Goose binary and then package it into a Kali Linux environment.

## Running the Container

To run the container interactively:

```bash
docker run -it --rm goose-redteam
```

This will start Goose with the "Red Team Assistant" recipe enabled.

### Advanced Usage

You can override the default command to use other recipes or arguments:

```bash
docker run -it --rm goose-redteam run --instruction "Scan the network 192.168.1.0/24"
```

## Recipe Details

The configuration is defined in `redteam/recipe.yaml`. It exposes the security tools via a custom MCP server wrapper located at `/opt/redteam/security_tools.py`.

## Notes

*   **Metasploit**: The `metasploit_command` tool runs `msfconsole` non-interactively. For complex sessions, you might need to use `developer` tools to spawn a shell.
*   **Burp Suite**: While installed, Burp Suite is a GUI application. To use it fully, you may need to forward X11 or use VNC, which is outside the scope of the default headless configuration.
*   **Permissions**: Some network tools (like Nmap or Tshark) may require elevated privileges for certain operations (e.g., raw socket access). The container runs as a non-root user `goose` by default. If you need root access, run the container with `--user root`.
