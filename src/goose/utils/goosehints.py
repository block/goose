from pathlib import Path

from goose.toolkit.utils import render_template


def fetch_goosehints() -> str:
    hints = []
    dirs = [Path.cwd()] + list(Path.cwd().parents)
    # reverse to go from parent to child
    dirs.reverse()

    for dir in dirs:
        hints_path = dir / ".goosehints"
        if hints_path.is_file():
            hints.append(render_template(hints_path))

    home_hints_path = Path.home() / ".config/goose/.goosehints"
    if home_hints_path.is_file():
        hints.append(render_template(home_hints_path))

    return "\n\n".join(hints)
