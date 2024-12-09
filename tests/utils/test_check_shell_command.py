import pytest
from goose.utils.command_checker import add_dangerous_command_patterns, is_dangerous_command


@pytest.mark.parametrize(
    "command",
    [
        "rm -rf /",
        "git push origin master",
        "sudo reboot",
        "mv /etc/passwd /tmp/",
        "chmod 777 /etc/passwd",
        "chown root:root /etc/passwd",
        "mkfs -t ext4 /dev/sda1",
        "systemctl stop nginx",
        "reboot",
        "shutdown now",
        "cat ~/.hello.txt",
        "cat ~/.config/example.txt",
        "pkill -f gradle",
        "fuser -k -n tcp 80",
    ],
)
def test_dangerous_commands(command):
    assert is_dangerous_command(command)


@pytest.mark.parametrize(
    "command",
    [
        "ls -la",
        'echo "Hello World"',
        "cp ~/folder/file.txt /tmp/",
        "echo hello > ~/toplevel/sublevel.txt",
        "cat hello.txt",
        "cat ~/config/example.txt",
        "ls -la path/to/visible/file",
        "echo 'file.with.dot.txt'",
    ],
)
def test_safe_commands(command):
    assert not is_dangerous_command(command)


def test_add_dangerous_patterns():
    add_dangerous_command_patterns(["echo hello"])
    assert is_dangerous_command("echo hello")

    # and that the original commands are still flagged
    assert is_dangerous_command("rm -rf /")
