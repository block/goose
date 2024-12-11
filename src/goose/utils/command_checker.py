import re
from typing import List

_dangerous_patterns = [
    # Commands that are generally unsafe
    r"\brm\b",  # rm command
    r"\bgit\s+push\b",  # git push command
    r"\bsudo\b",  # sudo command
    r"\bmv\b",  # mv command
    r"\bchmod\b",  # chmod command
    r"\bchown\b",  # chown command
    r"\bmkfs\b",  # mkfs command
    r"\bsystemctl\b",  # systemctl command
    r"\breboot\b",  # reboot command
    r"\bshutdown\b",  # shutdown command
    # Commands that kill processes
    r"\b(kill|pkill|killall|xkill|skill)\b",
    r"\bfuser\b\s*-[kK]",  # fuser -k command
    # Target files that are unsafe
    r"\b~\/\.|\/\.\w+",  # commands that point to files or dirs in home that start with a dot (dotfiles)
]
_compiled_patterns = [re.compile(pattern) for pattern in _dangerous_patterns]


def is_dangerous_command(command: str) -> bool:
    """
    Check if the command matches any dangerous patterns.

    Dangerous patterns in this function are defined as commands that may present risk to system stability.

    Args:
        command (str): The shell command to check.

    Returns:
        bool: True if the command is dangerous, False otherwise.
    """
    return any(pattern.search(command) for pattern in _compiled_patterns)


def add_dangerous_command_patterns(patterns: List[str]) -> None:
    """
    Add additional dangerous patterns to the command checker. Intended to be
    called in plugins that add additional high-specificity dangerous commands.

    Args:
        patterns (List[str]): The regex patterns to add to the dangerous patterns list.
    """
    _dangerous_patterns.extend(patterns)
    _compiled_patterns.extend([re.compile(pattern) for pattern in patterns])
