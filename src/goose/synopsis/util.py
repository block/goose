from goose.notifier import Notifier
from goose.toolkit.utils import RULEPREFIX, RULESTYLE
from rich.markdown import Markdown
from rich.rule import Rule


def log_command(notifier: Notifier, command: str, path: str, title: str = "shell") -> None:
    notifier.log("")
    notifier.log(Rule(RULEPREFIX + f"{title} | [dim magenta]{path}[/]", style=RULESTYLE, align="left"))
    notifier.log(Markdown(f"```bash\n{command}\n```"))
    notifier.log("")
