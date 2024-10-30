import subprocess
import os
from typing import Literal, Dict
from rich.markdown import Markdown
from rich.rule import Rule
from goose.notifier import Notifier
from goose.synopsis.system import system
from goose.synopsis.util import log_command
from goose.toolkit.utils import RULEPREFIX, RULESTYLE
from goose.utils.shell import is_dangerous_command, keep_unsafe_command_prompt

ProcessManagerCommand = Literal["start", "list", "view_output", "cancel"]


class ProcessManager:
    def __init__(self, notifier: Notifier) -> None:
        self.notifier = notifier

        # Command dispatch dictionary
        self.command_dispatch = {
            "start": self._start_process,
            "list": self._list_processes,
            "view_output": self._view_process_output,
            "cancel": self._cancel_process,
        }

    def _logshell(self, command: str, title: str = "background") -> None:
        log_command(self.notifier, command, path=os.path.abspath(system.cwd), title=title)

    def _start_process(self, shell_command: str, **kwargs: dict) -> int:
        """Start a background process running the specified command."""
        self._logshell(shell_command, title="background")

        if is_dangerous_command(shell_command):
            self.notifier.stop()
            if not keep_unsafe_command_prompt(shell_command):
                raise RuntimeError(f"The command {shell_command} was rejected as dangerous.")
            self.notifier.start()

        process = subprocess.Popen(
            shell_command,
            shell=True,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            cwd=system.cwd,
            env=system.env,
        )
        process_id = system.add_process(process)
        return process_id

    def _list_processes(self, **kwargs: dict) -> Dict[int, str]:
        """List all running background processes."""
        processes = system.get_processes()
        process_list = "```\n" + "\n".join(f"id: {pid}, command: {cmd}" for pid, cmd in processes.items()) + "\n```"
        self.notifier.log("")
        self.notifier.log(Rule(RULEPREFIX + "processes", style=RULESTYLE, align="left"))
        self.notifier.log(Markdown(process_list))
        self.notifier.log("")
        return processes

    def _view_process_output(self, process_id: int, **kwargs: dict) -> str:
        """View the output of a running background process."""
        self.notifier.log("")
        self.notifier.log(Rule(RULEPREFIX + "processes", style=RULESTYLE, align="left"))
        self.notifier.log(Markdown(f"```\nreading {process_id}\n```"))
        self.notifier.log("")
        output = system.view_process_output(process_id)
        return output

    def _cancel_process(self, process_id: int, **kwargs: dict) -> str:
        """Cancel the background process with the specified ID."""
        result = system.cancel_process(process_id)
        self._logshell(f"kill {process_id}")
        if result:
            return f"Process {process_id} cancelled"
        else:
            return f"No known process with ID {process_id}"

    def run_command(self, command: ProcessManagerCommand, **kwargs: dict) -> str:
        """
        Dispatch process management commands.

        Args:
            command (ProcessManagerCommand): The process management command to execute.
            **kwargs: Additional arguments for the commands, such as shell_command or process_id.

        Returns:
            str: The result of the process management operation.
        """
        if command not in self.command_dispatch:
            raise ValueError(f"Unknown command '{command}'.")

        return self.command_dispatch[command](**kwargs)
