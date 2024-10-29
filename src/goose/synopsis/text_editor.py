from typing import Optional, Literal
from pathlib import Path
from rich.markdown import Markdown
from rich.rule import Rule
from goose.notifier import Notifier
from goose.synopsis.system import system
from goose.toolkit.utils import RULEPREFIX, RULESTYLE, get_language

TextEditorCommand = Literal["view", "create", "str_replace", "insert", "undo_edit"]


class TextEditor:
    def __init__(self, notifier: Notifier) -> None:
        self.notifier = notifier
        self._file_history = {}

        # Command dispatch dictionary
        self.command_dispatch = {
            "view": self._view_file_or_directory,
            "create": self._create_file,
            "str_replace": self._replace_string,
            "insert": self._insert_string,
            "undo_edit": self._undo_edit,
        }

    def _write_file(self, path: str, content: str) -> str:
        """Write content to the file at path."""
        patho = system.to_patho(path)

        if patho.exists() and not system.is_active(path):
            raise ValueError(f"You must view {path} using read_file before you overwrite it")

        self._save_file_history(patho)
        patho.parent.mkdir(parents=True, exist_ok=True)
        patho.write_text(content)
        system.remember_file(path)

        language = get_language(path)
        self._log_file_operation(path, content, language)
        return f"Successfully wrote to {path}"

    def _patch_file(self, path: str, before: str, after: str) -> str:
        """Patch the file by replacing 'before' with 'after'."""
        patho = system.to_patho(path)

        if not patho.exists():
            raise ValueError(f"You can't patch {path} - it does not exist yet")
        if not system.is_active(path):
            raise ValueError(f"You must view {path} using read_file before you patch it")

        content = patho.read_text()

        if content.count(before) != 1:
            raise ValueError("The 'before' content must appear exactly once in the file.")

        self._save_file_history(patho)
        content = content.replace(before, after)
        system.remember_file(path)
        patho.write_text(content)

        self._log_file_operation(path, f"{before} -> {after}", get_language(path))
        return "Successfully replaced before with after."

    def _save_file_history(self, patho: Path) -> None:
        """Save the current content of the file to history for undo functionality."""
        content = patho.read_text() if patho.exists() else ""
        self._file_history[str(patho)] = content

    def _undo_edit(self, path: str, **kwargs: dict) -> str:
        """Undo the last edit made to a file."""
        patho = system.to_patho(path)

        if not patho.exists() or str(patho) not in self._file_history:
            raise ValueError(f"No edit history available to undo changes on {path}.")

        previous_content = self._file_history.pop(str(patho))
        patho.write_text(previous_content)
        system.remember_file(path)

        self._log_file_operation(path, "Undo edit", get_language(path))
        return f"Successfully undid the last edit on {path}"

    def _view_file_or_directory(self, path: str, view_range: Optional[list[int]] = None, **kwargs: dict) -> str:
        """View the content of a file or directory."""
        patho = system.to_patho(path)

        if patho.is_file():
            return self._view_file(patho, view_range)
        elif patho.is_dir():
            return self._view_directory(patho)
        else:
            raise ValueError(f"The path {path} does not exist.")

    def _view_file(self, patho: Path, view_range: Optional[list[int]]) -> str:
        if not patho.exists():
            raise ValueError(f"The file {patho} does not exist.")

        with open(patho, "r") as f:
            content = f.readlines()

        if view_range:
            start_line, end_line = view_range
            if start_line < 1 or end_line < start_line:
                raise ValueError("Invalid view range.")
            content = content[start_line - 1 : (end_line if end_line != -1 else len(content))]

        system.remember_file(str(patho))
        return f"Displayed content of {str(patho)}"

    def _view_directory(self, patho: Path) -> str:
        files = [str(p) for p in patho.iterdir()]
        dir_content = "\n".join(files)
        return f"The contents of directory {str(patho)}:\n{dir_content}"

    def _insert_string(self, path: str, insert_line: int, new_str: str, **kwargs: dict) -> str:
        """Insert a string into the file after a specific line number."""
        patho = system.to_patho(path)
        if not patho.exists() or not system.is_active(path):
            raise ValueError(f"You must view {path} before editing.")

        self._save_file_history(patho)
        with open(patho, "r") as f:
            lines = f.readlines()

        if insert_line < 0 or insert_line > len(lines):
            raise ValueError("Insert line is out of range.")

        lines.insert(insert_line, new_str + "\n")
        with open(patho, "w") as f:
            f.writelines(lines)

        system.remember_file(path)
        self._log_file_operation(path, new_str, get_language(path))
        return f"Successfully inserted new_str into {path} after line {insert_line}"

    def _create_file(self, path: str, file_text: str, **kwargs: dict) -> str:
        """Create a new file with the given content."""
        return self._write_file(path, file_text)

    def _replace_string(self, path: str, old_str: str, new_str: str, **kwargs: dict) -> str:
        """Replace a string in a file."""
        return self._patch_file(path, old_str, new_str)

    def _log_file_operation(self, path: str, content: str, language: Optional[str]) -> None:
        """Log the file operation in markdown format."""
        md_content = f"```{language}\n{content}\n```" if language else f"```\n{content}\n```"
        self.notifier.log("")
        self.notifier.log(Rule(RULEPREFIX + path, style=RULESTYLE, align="left"))
        self.notifier.log(Markdown(md_content))
        self.notifier.log("")

    def run_command(self, command: TextEditorCommand, path: str, **kwargs: dict) -> str:
        """Dispatch text editing operations to the appropriate handler."""
        if command not in self.command_dispatch:
            raise ValueError(f"Unknown command '{command}'.")

        return self.command_dispatch[command](path, **kwargs)
