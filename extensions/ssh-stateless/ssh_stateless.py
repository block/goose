from __future__ import annotations

from pathlib import Path
from typing import Any

import paramiko
from fastmcp import FastMCP

mcp = FastMCP("ssh-stateless")


def _build_connect_kwargs(
    host: str,
    username: str,
    password: str | None,
    key_path: str | None,
    port: int,
) -> dict[str, Any] | None:
    if password is None and key_path is None:
        return None

    connect_kwargs: dict[str, Any] = {
        "hostname": host,
        "port": port,
        "username": username,
        # Keep every invocation self-contained instead of using ambient SSH agent state.
        "allow_agent": False,
        "look_for_keys": False,
        "timeout": 15,
        "banner_timeout": 15,
        "auth_timeout": 15,
    }

    if password is not None:
        connect_kwargs["password"] = password
    else:
        connect_kwargs["key_filename"] = str(Path(key_path).expanduser())

    return connect_kwargs


def _read_stream(stream: Any) -> str:
    data = stream.read()
    if isinstance(data, bytes):
        return data.decode(errors="replace")
    return str(data)


@mcp.tool()
def ssh_exec(
    host: str,
    username: str,
    command: str,
    password: str | None = None,
    key_path: str | None = None,
    port: int = 22,
) -> str:
    """Run a single SSH command over a fresh connection with no persisted session state."""
    connect_kwargs = _build_connect_kwargs(host, username, password, key_path, port)
    if connect_kwargs is None:
        return "Error: Provide password or key_path"

    client = paramiko.SSHClient()
    # Accept new host keys only in memory for this process; nothing is written back to disk.
    client.set_missing_host_key_policy(paramiko.AutoAddPolicy())

    try:
        client.connect(**connect_kwargs)
        _, stdout, stderr = client.exec_command(command)
        parts = [_read_stream(stdout).strip(), _read_stream(stderr).strip()]
        return "\n".join(part for part in parts if part).strip()
    finally:
        client.close()


def main() -> None:
    mcp.run()


if __name__ == "__main__":
    main()
