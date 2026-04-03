from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path
from unittest.mock import patch


MODULE_PATH = Path(__file__).resolve().parent.parent / "ssh_stateless.py"
MODULE_SPEC = importlib.util.spec_from_file_location("ssh_stateless_under_test", MODULE_PATH)
if MODULE_SPEC is None or MODULE_SPEC.loader is None:
    raise RuntimeError(f"Unable to load ssh_stateless.py from {MODULE_PATH}")
ssh_stateless = importlib.util.module_from_spec(MODULE_SPEC)
MODULE_SPEC.loader.exec_module(ssh_stateless)


class FakeStream:
    def __init__(self, payload: bytes) -> None:
        self.payload = payload

    def read(self) -> bytes:
        return self.payload


class FakeSSHClient:
    instances: list["FakeSSHClient"] = []
    stdout_payload = b""
    stderr_payload = b""

    def __init__(self) -> None:
        self.connect_calls: list[dict[str, object]] = []
        self.exec_calls: list[str] = []
        self.closed = False
        self.policy = None
        FakeSSHClient.instances.append(self)

    def set_missing_host_key_policy(self, policy: object) -> None:
        self.policy = policy

    def connect(self, **kwargs: object) -> None:
        self.connect_calls.append(kwargs)

    def exec_command(self, command: str) -> tuple[None, FakeStream, FakeStream]:
        self.exec_calls.append(command)
        return None, FakeStream(self.stdout_payload), FakeStream(self.stderr_payload)

    def close(self) -> None:
        self.closed = True


class SSHStatelessTests(unittest.TestCase):
    def setUp(self) -> None:
        FakeSSHClient.instances.clear()
        FakeSSHClient.stdout_payload = b"command output\n"
        FakeSSHClient.stderr_payload = b""

    def test_password_auth_runs_command_and_closes(self) -> None:
        with patch.object(ssh_stateless.paramiko, "SSHClient", FakeSSHClient):
            result = ssh_stateless.ssh_exec(
                host="example.com",
                username="alice",
                command="whoami",
                password="secret",
            )

        self.assertEqual(result, "command output")
        self.assertEqual(len(FakeSSHClient.instances), 1)

        client = FakeSSHClient.instances[0]
        self.assertTrue(client.closed)
        self.assertEqual(client.exec_calls, ["whoami"])
        self.assertEqual(
            client.connect_calls,
            [
                {
                    "hostname": "example.com",
                    "port": 22,
                    "username": "alice",
                    "allow_agent": False,
                    "look_for_keys": False,
                    "timeout": 15,
                    "banner_timeout": 15,
                    "auth_timeout": 15,
                    "password": "secret",
                }
            ],
        )

    def test_key_auth_expands_key_path(self) -> None:
        expected_key_path = str(Path("~/.ssh/id_ed25519").expanduser())

        with patch.object(ssh_stateless.paramiko, "SSHClient", FakeSSHClient):
            ssh_stateless.ssh_exec(
                host="example.com",
                username="alice",
                command="hostname",
                key_path="~/.ssh/id_ed25519",
                port=2200,
            )

        client = FakeSSHClient.instances[0]
        self.assertEqual(client.exec_calls, ["hostname"])
        self.assertEqual(client.connect_calls[0]["key_filename"], expected_key_path)
        self.assertEqual(client.connect_calls[0]["port"], 2200)
        self.assertTrue(client.closed)

    def test_missing_auth_returns_error_without_connecting(self) -> None:
        with patch.object(ssh_stateless.paramiko, "SSHClient", FakeSSHClient):
            result = ssh_stateless.ssh_exec(
                host="example.com",
                username="alice",
                command="pwd",
            )

        self.assertEqual(result, "Error: Provide password or key_path")
        self.assertEqual(FakeSSHClient.instances, [])

    def test_each_tool_call_uses_a_new_client(self) -> None:
        with patch.object(ssh_stateless.paramiko, "SSHClient", FakeSSHClient):
            ssh_stateless.ssh_exec(
                host="example.com",
                username="alice",
                command="date",
                password="one",
            )
            ssh_stateless.ssh_exec(
                host="example.com",
                username="alice",
                command="uptime",
                password="two",
            )

        self.assertEqual(len(FakeSSHClient.instances), 2)
        self.assertIsNot(FakeSSHClient.instances[0], FakeSSHClient.instances[1])
        self.assertTrue(all(client.closed for client in FakeSSHClient.instances))


if __name__ == "__main__":
    unittest.main()
