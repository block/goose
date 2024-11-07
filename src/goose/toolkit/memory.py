import re
from pathlib import Path
from typing import Optional

from exchange import Message

from goose.toolkit.base import Toolkit, tool


class Memory(Toolkit):
    """Memory toolkit for storing important information in .goosehints"""

    def __init__(self, *args: object, **kwargs: dict[str, object]) -> None:
        super().__init__(*args, **kwargs)
        self.local_hints = Path(".goosehints")
        self.global_hints = Path.home() / ".config/goose/.goosehints"
        self._ensure_hints_files()

    def system(self) -> str:
        """Get the memory-specific additions to the system prompt"""
        return Message.load("prompts/memory.jinja").text

    def _ensure_hints_files(self) -> None:
        """Ensure the .goosehints files exist"""
        # Create global hints directory and file if needed
        self.global_hints.parent.mkdir(parents=True, exist_ok=True)
        if not self.global_hints.exists():
            self.global_hints.write_text("")

    @tool
    def remember(self, key: str, value: str, scope: str = "global") -> str:
        """Save a piece of information to .goosehints

        Args:
            key (str): The label/name for this information
            value (str): The information to remember
            scope (str): Where to store the hint - 'global' (in ~/.config/goose) or 'local' (in current directory)
        """
        hints_file = self.global_hints if scope == "global" else self.local_hints
        hint = f"{{% set {key} = '{value}' %}}\n"

        # Create or append to the hints file
        if hints_file.exists():
            current_content = hints_file.read_text()
            if f"set {key} =" in current_content:
                return f"I already have information stored about {key}"
            hints_file.write_text(current_content + hint)
        else:
            hints_file.write_text(hint)

        return f"I'll remember that {key} is {value} in {scope} hints"

    @tool
    def forget(self, key: str, scope: str = "global") -> str:
        """Remove a stored piece of information from .goosehints

        Args:
            key (str): The label/name of the information to remove
            scope (str): Where to remove from - 'global' (in ~/.config/goose) or 'local' (in current directory)
        """
        hints_file = self.global_hints if scope == "global" else self.local_hints

        if not hints_file.exists():
            return f"No hints file found in {scope} scope"

        content = hints_file.read_text()
        pattern = rf"{{% *set *{key} *= *'[^']*' *%}}"

        if not re.search(pattern, content):
            return f"No information found for key '{key}' in {scope} hints"

        new_content = re.sub(pattern, "", content).strip()
        hints_file.write_text(new_content)

        return f"Successfully removed information for '{key}' from {scope} hints"

    @tool
    def list_hints(self, scope: Optional[str] = None) -> str:
        """List all hints in the specified scope(s)

        Args:
            scope (str, optional): Which hints to list - 'global', 'local', or None (both)
        """
        hints = []

        if scope in (None, "local") and self.local_hints.exists():
            content = self.local_hints.read_text().strip()
            if content:
                hints.append("Local hints (.goosehints):")
                hints.append(content)

        if scope in (None, "global") and self.global_hints.exists():
            content = self.global_hints.read_text().strip()
            if content:
                hints.append("Global hints (~/.config/goose/.goosehints):")
                hints.append(content)

        if not hints:
            return "No hints found in the specified scope(s)"

        return "\n".join(hints)
