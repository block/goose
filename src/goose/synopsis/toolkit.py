# janky global state for now, think about it
from collections import defaultdict
import re
import tempfile
from typing import Dict, Optional

from exchange import Message
import httpx
from goose.synopsis.bash import Bash
from goose.synopsis.text_editor import TextEditor, TextEditorCommand
from goose.synopsis.process_manager import ProcessManager, ProcessManagerCommand
from goose.toolkit.base import Toolkit, tool


class SynopsisDeveloper(Toolkit):
    """Provides shell and file operation tools using OperatingSystem."""

    def __init__(self, *args: object, **kwargs: Dict[str, object]) -> None:
        super().__init__(*args, **kwargs)
        self._file_history = defaultdict(list)

    def system(self) -> str:
        """Retrieve system configuration details for developer"""
        system_prompt = Message.load("developer.md").text
        return system_prompt

    @tool
    def bash(
        self,
        command: Optional[str] = None,
        working_dir: Optional[str] = None,
        source_path: Optional[str] = None,
    ) -> str:
        """
        Run commands in a bash shell.

        Perform bash-related operations in a specific order:
        1. Change the working directory (if provided)
        2. Source a file (if provided)
        3. Run a shell command (if provided)

        At least one of the parameters must be provided.

        Args:
            command (str, optional):The bash shell command to run.
            working_dir (str, optional): The directory to change to.
            source_path (str, optional): The file to source before running the command.
        """
        assert any(
            [command, working_dir, source_path]
        ), "At least one of the parameters for bash shell must be provided."

        bash_tool = Bash(notifier=self.notifier, exchange_view=self.exchange_view)
        outputs = []

        if working_dir:
            _out = bash_tool._change_dir(working_dir)
            outputs.append(_out)

        if source_path:
            _out = bash_tool._source(source_path)
            outputs.append(_out)

        if command:
            _out = bash_tool._shell(command)
            outputs.append(_out)

        return "\n".join(outputs)

    @tool
    def text_editor(
        self,
        command: TextEditorCommand,
        path: str,
        file_text: Optional[str] = None,
        insert_line: Optional[int] = None,
        new_str: Optional[str] = None,
        old_str: Optional[str] = None,
        view_range: Optional[list[int]] = None,
    ) -> str:
        """
        Perform text editing operations on files.

        The `command` parameter specifies the operation to perform. Allowed options are:
        - `view`: View the content of a file or directory.
        - `create`: Create a new file with the given content.
        - `str_replace`: Replace a string in a file with a new string.
        - `insert`: Insert a string into a file after a specific line number.
        - `undo_edit`: Undo the last edit made to a file.

        Args:
            command (str): The commands to run.
                Allowed options are: `view`, `create`, `str_replace`, `insert`, `undo_edit`.
            path (str): Absolute path (or relative path against cwd) to file or directory,
                e.g. `/repo/file.py` or `/repo` or `curr_dir_file.py`.
            file_text (str, optional): Required parameter of `create` command, with the content
                of the file to be created.
            insert_line (int, optional): Required parameter of `insert` command.
                The `new_str` will be inserted AFTER the line `insert_line` of `path`.
            new_str (str, optional): Optional parameter of `str_replace` command
                containing the new string (if not given, no string will be added).
                Required parameter of `insert` command containing the string to insert.
            old_str (str, optional): Required parameter of `str_replace` command containing the
                string in `path` to replace.
            view_range (list, optional): Optional parameter of `view` command when `path` points to a file.
                If none is given, the full file is shown. If provided, the file will be shown in the indicated line
                number range, e.g. [11, 12] will show lines 11 and 12. Indexing at 1 to start.
                Setting `[start_line, -1]` shows all lines from `start_line` to the end of the file.
        """
        text_editor_instance = TextEditor(notifier=self.notifier)
        return text_editor_instance.run_command(
            command=command,
            path=path,
            file_text=file_text,
            insert_line=insert_line,
            new_str=new_str,
            old_str=old_str,
            view_range=view_range,
        )

    @tool
    def process_manager(
        self,
        command: ProcessManagerCommand,
        shell_command: Optional[str] = None,
        process_id: Optional[int] = None,
    ) -> str:
        """
        Manage background processes.

        The `command` parameter specifies the operation to perform. Allowed options are:
        - `start`: Start a background process by running a shell command.
        - `list`: List all currently running background processes with their IDs and commands.
        - `view_output`: View the output of a running background process by providing its ID.
        - `cancel`: Cancel a running background process by providing its ID.

        Args:
            command (str): The command to run.
                Allowed options are: `start`, `list`, `view_output`, `cancel`.
            shell_command (str, optional): Required parameter for the `start` command, representing
                the shell command to be executed in the background.
                Example: `"python -m http.server &"` to start a web server in the background.
            process_id (int, optional): Required parameter for `view_output` and `cancel` commands,
                representing the process ID of the background process to manage.
        """
        process_manager_instance = ProcessManager(notifier=self.notifier)
        return process_manager_instance.run_command(
            command=command,
            shell_command=shell_command,
            process_id=process_id,
        )

    @tool
    def fetch_web_content(self, url: str) -> str:
        """
        Fetch content from a URL using httpx.

        Args:
            url (str): url of the site to visit.
        Returns:
            (dict): A dictionary with two keys:
                - 'html_file_path' (str): Path to a html file which has the content of the page. It will be very large so use rg to search it or head in chunks. Will contain meta data and links and markup.
                - 'text_file_path' (str): Path to a plain text file which has the some of the content of the page. It will be large so use rg to search it or head in chunks. If content isn't there, try the html variant.
        """  # noqa
        friendly_name = re.sub(r"[^a-zA-Z0-9]", "_", url)[:50]  # Limit length to prevent filenames from being too long

        try:
            result = httpx.get(url, follow_redirects=True).text
            with tempfile.NamedTemporaryFile(delete=False, mode="w", suffix=f"_{friendly_name}.html") as tmp_file:
                tmp_file.write(result)
                tmp_text_file_path = tmp_file.name.replace(".html", ".txt")
                plain_text = re.sub(
                    r"<head.*?>.*?</head>|<script.*?>.*?</script>|<style.*?>.*?</style>|<[^>]+>",
                    "",
                    result,
                    flags=re.DOTALL,
                )  # Remove head, script, and style tags/content, then any other tags
                with open(tmp_text_file_path, "w") as text_file:
                    text_file.write(plain_text)
                return {"html_file_path": tmp_file.name, "text_file_path": tmp_text_file_path}
        except httpx.HTTPStatusError as exc:
            self.notifier.log(f"Failed fetching with HTTP error: {exc.response.status_code}")
        except Exception as exc:
            self.notifier.log(f"Failed fetching with error: {str(exc)}")
