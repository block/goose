#!/usr/bin/env -S uv run --quiet --script
# /// script
# dependencies = ["textual>=0.87.0", "pyperclip"]
# ///
"""
WARNING: entirely vibe coded. use as a throwaway tool


Diagnostics Viewer - Browse and inspect Goose diagnostics bundles.

Scans for diagnostics zip files, displays their sessions, and provides
an interactive viewer for examining session data, logs, and other files.
"""
import json
import sys
import zipfile
from pathlib import Path
from typing import Optional, Any

import pyperclip

from textual.app import App, ComposeResult
from textual.widgets import Header, Footer, Static, Tree, ListView, ListItem, Label, Input
from textual.containers import Horizontal, Vertical, VerticalScroll, Container
from textual.binding import Binding
from textual.message import Message
from textual.screen import ModalScreen


def truncate_string(s: str, max_len: int = 100, edge_len: int = 35) -> str:
    """Truncate a string if it's longer than max_len."""
    if len(s) <= max_len:
        return s

    omitted = len(s) - (2 * edge_len)
    return f"{s[:edge_len]}[{omitted} more]{s[-edge_len:]}"


class JsonTreeView(Tree):
    """A tree widget for displaying collapsible JSON."""

    BINDINGS = [
        Binding("ctrl+o", "toggle_all", "Toggle All", show=True),
    ]

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.json_data = None
        self.show_root = False
        self.all_expanded = False

    def load_json(self, data: Any, label: str = "JSON"):
        """Load JSON data into the tree."""
        self.json_data = data
        self.clear()
        self.root.label = label
        self._build_tree(self.root, data)
        # Expand all nodes by default
        self.root.expand_all()

    def action_toggle_all(self):
        """Toggle expansion of all nodes."""
        self.all_expanded = not self.all_expanded
        if self.all_expanded:
            self.root.expand_all()
        else:
            self.root.collapse_all()
            self.root.expand()  # Keep root expanded

    def on_tree_node_selected(self, event: Tree.NodeSelected):
        """Handle node selection - show modal for truncated strings."""
        node = event.node

        # Check if this is a truncated string node
        if node.data and isinstance(node.data, dict) and node.data.get("truncated"):
            key = node.data["key"]
            value = node.data["value"]

            # Show the full string in a modal
            title = f"Full String Value for '{key}'"
            self.app.push_screen(TextViewerModal(title, value))

            # Prevent default tree expansion behavior
            event.stop()

    def _build_tree(self, node, data, max_depth=10, current_depth=0):
        """Recursively build the tree from JSON data."""
        if current_depth > max_depth:
            node.add_leaf("[dim]...[/dim]")
            return

        if isinstance(data, dict):
            for key, value in data.items():
                if isinstance(value, (dict, list)) and value:
                    # Expand first level by default
                    child = node.add(f"[cyan]{key}[/cyan]: {{...}}" if isinstance(value, dict) else f"[cyan]{key}[/cyan]: [...]", expand=(current_depth == 0))
                    child.data = {"key": key, "value": value, "type": type(value).__name__, "expandable": False}
                    self._build_tree(child, value, max_depth, current_depth + 1)
                elif isinstance(value, str):
                    truncated = truncate_string(value)
                    if truncated != value:
                        # Make truncated strings expandable
                        child = node.add(f"[cyan]{key}[/cyan]: [green]\"{truncated}\"[/green]", expand=False)
                        child.data = {"key": key, "value": value, "type": "str", "truncated": True, "expandable": True}
                        child.allow_expand = False  # Don't show expand icon initially
                    else:
                        node.add_leaf(f"[cyan]{key}[/cyan]: [green]\"{value}\"[/green]")
                elif isinstance(value, bool):
                    # Check bool before int/float since bool is a subclass of int
                    node.add_leaf(f"[cyan]{key}[/cyan]: [magenta]{str(value).lower()}[/magenta]")
                elif isinstance(value, (int, float)):
                    node.add_leaf(f"[cyan]{key}[/cyan]: [yellow]{value}[/yellow]")
                elif value is None:
                    node.add_leaf(f"[cyan]{key}[/cyan]: [dim]null[/dim]")
                else:
                    node.add_leaf(f"[cyan]{key}[/cyan]: {value}")

        elif isinstance(data, list):
            for i, item in enumerate(data):
                if isinstance(item, (dict, list)) and item:
                    # Expand first level by default
                    child = node.add(f"[yellow]{i}[/yellow]: {{...}}" if isinstance(item, dict) else f"[yellow]{i}[/yellow]: [...]", expand=(current_depth == 0))
                    child.data = {"key": i, "value": item, "type": type(item).__name__, "expandable": False}
                    self._build_tree(child, item, max_depth, current_depth + 1)
                elif isinstance(item, str):
                    truncated = truncate_string(item)
                    if truncated != item:
                        # Make truncated strings expandable
                        child = node.add(f"[yellow]{i}[/yellow]: [green]\"{truncated}\"[/green]", expand=False)
                        child.data = {"key": i, "value": item, "type": "str", "truncated": True, "expandable": True}
                        child.allow_expand = False  # Don't show expand icon initially
                    else:
                        node.add_leaf(f"[yellow]{i}[/yellow]: [green]\"{item}\"[/green]")
                elif isinstance(item, bool):
                    # Check bool before int/float since bool is a subclass of int
                    node.add_leaf(f"[yellow]{i}[/yellow]: [magenta]{str(item).lower()}[/magenta]")
                elif isinstance(item, (int, float)):
                    node.add_leaf(f"[yellow]{i}[/yellow]: [yellow]{item}[/yellow]")
                elif item is None:
                    node.add_leaf(f"[yellow]{i}[/yellow]: [dim]null[/dim]")
                else:
                    node.add_leaf(f"[yellow]{i}[/yellow]: {item}")


class TextViewerModal(ModalScreen):
    """Modal screen for viewing long text strings."""

    BINDINGS = [
        Binding("escape,q,enter", "dismiss", "Close", show=True),
        Binding("c", "copy", "Copy", show=True),
    ]

    def __init__(self, title: str, text: str):
        super().__init__()
        self.title = title
        self.text = text

    def compose(self) -> ComposeResult:
        """Compose the modal content."""
        with Vertical(id="modal-container"):
            yield Static(f"[bold]{self.title}[/bold]", id="modal-title")
            with VerticalScroll(id="modal-scroll"):
                yield Static(self.text, id="modal-text")
            yield Static("[dim]Press C to copy, Escape/Q/Enter to close[/dim]", id="modal-footer")

    def action_dismiss(self):
        """Dismiss the modal."""
        self.app.pop_screen()

    def action_copy(self):
        """Copy the text to clipboard."""
        pyperclip.copy(self.text)
        self.notify("Copied to clipboard")


class SearchOverlay(Container):
    """Search overlay widget."""

    def __init__(self):
        super().__init__()
        self.display = False

    def compose(self) -> ComposeResult:
        with Horizontal(id="search-container"):
            yield Static("Search: ", id="search-label")
            yield Input(placeholder="Type to search...", id="search-input")
            yield Static("", id="search-results")


class DiagnosticsSession:
    """Represents a diagnostics bundle."""

    def __init__(self, zip_path: Path):
        self.zip_path = zip_path
        self.name = "Unknown Session"
        self.session_id = zip_path.stem
        self.created_at = zip_path.stat().st_mtime
        self._load_session_name()

    def _load_session_name(self):
        """Extract session name from session.json."""
        try:
            with zipfile.ZipFile(self.zip_path, 'r') as zf:
                # Find session.json
                for name in zf.namelist():
                    if name.endswith('session.json'):
                        with zf.open(name) as f:
                            data = json.load(f)
                            self.name = data.get('name', 'Unknown Session')
                            self.session_id = data.get('id', self.zip_path.stem)
                        break
        except Exception as e:
            self.name = f"Error loading: {e}"

    def get_file_list(self) -> list[str]:
        """Get list of files in the zip, sorted with system.txt first."""
        try:
            with zipfile.ZipFile(self.zip_path, 'r') as zf:
                files = zf.namelist()

                # Sort: system.txt first, then session.json, then alphabetically
                def sort_key(f):
                    if f.endswith('system.txt'):
                        return (0, f)
                    elif f.endswith('session.json'):
                        return (1, f)
                    elif f.endswith('config.yaml'):
                        return (2, f)
                    else:
                        return (3, f)

                return sorted(files, key=sort_key)
        except Exception:
            return []

    def read_file(self, filename: str) -> Optional[str]:
        """Read a file from the zip.

        Returns:
            File content as string, or None if file cannot be read.
        """
        try:
            with zipfile.ZipFile(self.zip_path, 'r') as zf:
                with zf.open(filename) as f:
                    return f.read().decode('utf-8', errors='replace')
        except Exception:
            # File not found or cannot be read
            return None


class FileContentPane(Vertical):
    """A pane that shows either JSON tree or plain text."""

    def __init__(self, title: str):
        super().__init__()
        self.title = title
        self.content_type = "empty"
        self.json_data = None
        self.text_content = ""

    def compose(self) -> ComposeResult:
        """Compose the pane content."""
        if self.content_type == "json":
            tree = JsonTreeView(self.title)
            if self.json_data is not None:
                tree.load_json(self.json_data, self.title)
            yield tree
        elif self.content_type == "text":
            with VerticalScroll():
                yield Static(self.text_content)
        else:
            yield Static("[dim]No content[/dim]")

    def set_json(self, data: Any):
        """Set JSON content."""
        self.content_type = "json"
        self.json_data = data

    def set_text(self, text: str):
        """Set text content."""
        self.content_type = "text"
        self.text_content = text


class FileViewer(Vertical):
    """Widget for viewing file contents."""

    def __init__(self):
        super().__init__()
        self.current_session = None
        self.current_filename = None
        self.current_part = None

    def compose(self) -> ComposeResult:
        """Create child widgets."""
        with Vertical(id="content-area"):
            yield Static("[dim]Select a file to view[/dim]")

        yield SearchOverlay()

    def update_content(self, session: DiagnosticsSession, filename: str, part: str = None):
        """Update the viewer with new file content.

        Args:
            session: The diagnostics session
            filename: The file to display
            part: For JSONL files, either "request" or "responses"
        """
        self.current_session = session
        self.current_filename = filename
        self.current_part = part

        content = session.read_file(filename)
        if content is None:
            self._show_plain(filename, f"[red]Error: Could not read file '{filename}'[/red]")
            return

        # Check if this is a JSONL file
        if filename.endswith('.jsonl') and part:
            self._show_jsonl(filename, content, part)
        elif filename.endswith('.json'):
            self._show_json(filename, content)
        else:
            self._show_plain(filename, content)

        # Auto-focus the content
        self.post_message(self.ContentReady())

    def _show_jsonl(self, filename: str, content: str, part: str):
        """Show JSONL file - either request or responses part."""
        lines = [line.strip() for line in content.strip().split('\n') if line.strip()]

        # Parse lines
        request_data = None
        responses = []

        if len(lines) > 0:
            try:
                request_data = json.loads(lines[0])
            except json.JSONDecodeError:
                # Skip malformed request line; diagnostics may be truncated or corrupted
                pass

        for i in range(1, len(lines)):
            try:
                responses.append(json.loads(lines[i]))
            except json.JSONDecodeError:
                # Skip individual malformed response lines; show only valid JSON entries
                pass

        # Show content
        content_area = self.query_one("#content-area", Vertical)
        content_area.remove_children()

        if part == "request" and request_data:
            tree = JsonTreeView(f"{filename} - request")
            tree.load_json(request_data, f"{filename} - request")
            content_area.mount(tree)
        elif part == "responses" and responses:
            tree = JsonTreeView(f"{filename} - responses")
            if len(responses) == 1:
                tree.load_json(responses[0], f"{filename} - response")
            else:
                tree.load_json(responses, f"{filename} - responses")
            content_area.mount(tree)
        else:
            content_area.mount(Static("[red]No data available for this part[/red]"))

    def _show_json(self, filename: str, content: str):
        """Show JSON file with collapsible tree."""
        # Show content
        content_area = self.query_one("#content-area", Vertical)
        content_area.remove_children()

        tree = JsonTreeView(filename)
        try:
            data = json.loads(content)
            tree.load_json(data, filename)
        except json.JSONDecodeError as e:
            tree.root.add_leaf(f"[red]Error parsing JSON: {e}[/red]")

        content_area.mount(tree)

    def _show_plain(self, filename: str, content: str):
        """Show plain text content."""
        # Show content
        content_area = self.query_one("#content-area", Vertical)
        content_area.remove_children()

        # Create and mount the scroll container with the content
        scroll = VerticalScroll()
        content_area.mount(scroll)
        scroll.mount(Static(content))

    def focus_content(self):
        """Focus the content area."""
        try:
            # Try to focus a tree if present
            tree = self.query_one(JsonTreeView)
            tree.focus()
        except Exception:
            # No JsonTreeView present (e.g., showing plain text), which is fine
            pass

    def action_search(self):
        """Show search overlay.

        TODO: Implement actual search functionality - currently just shows UI.
        """
        overlay = self.query_one(SearchOverlay)
        overlay.display = not overlay.display
        if overlay.display:
            search_input = overlay.query_one("#search-input", Input)
            search_input.focus()

    class ContentReady(Message):
        """Message sent when content is ready to be focused."""
        pass


class SessionViewer(Vertical):
    """Widget for viewing a diagnostics session."""

    BINDINGS = [
        Binding("ctrl+f,cmd+f", "search", "Search", show=True),
        Binding("c", "copy_file", "Copy file", show=True),
    ]

    def __init__(self, session: DiagnosticsSession):
        super().__init__()
        self.session = session

    def compose(self) -> ComposeResult:
        """Create child widgets."""
        yield Static(f"[bold yellow]Session: {self.session.name}[/bold yellow]", id="session-title")

        with Horizontal(id="main-content"):
            # Left side: File browser
            with Vertical(id="file-browser"):
                yield Static("[bold]Files:[/bold]")
                tree = Tree("Files", id="file-tree")
                tree.show_root = False

                # Build file tree
                files = self.session.get_file_list()

                # Group by directory
                dirs = {}
                for file in files:
                    parts = file.split('/')
                    is_jsonl = file.endswith('.jsonl')

                    if len(parts) == 1:
                        # Root file
                        if is_jsonl:
                            # Add two entries for JSONL files
                            tree.root.add_leaf(f"{file} - request", data={"file": file, "part": "request"})
                            tree.root.add_leaf(f"{file} - responses", data={"file": file, "part": "responses"})
                        else:
                            tree.root.add_leaf(file, data={"file": file, "part": None})
                    else:
                        # File in directory
                        dir_name = parts[0]
                        if dir_name not in dirs:
                            dirs[dir_name] = tree.root.add(dir_name, expand=True)

                        file_name = '/'.join(parts[1:])
                        if is_jsonl:
                            # Add two entries for JSONL files
                            dirs[dir_name].add_leaf(f"{file_name} - request", data={"file": file, "part": "request"})
                            dirs[dir_name].add_leaf(f"{file_name} - responses", data={"file": file, "part": "responses"})
                        else:
                            dirs[dir_name].add_leaf(file_name, data={"file": file, "part": None})

                yield tree

            # Right side: File viewer
            yield FileViewer()

    def on_mount(self):
        """Handle mount event."""
        # Show system.txt by default and select it in tree
        files = self.session.get_file_list()
        system_file = next((f for f in files if f.endswith('system.txt')), None)
        if system_file:
            viewer = self.query_one(FileViewer)
            viewer.update_content(self.session, system_file)

            # Select the first node in the tree
            tree = self.query_one("#file-tree", Tree)
            if tree.root.children:
                tree.select_node(tree.root.children[0])

        # Focus the tree initially
        tree = self.query_one("#file-tree", Tree)
        tree.focus()

    def on_tree_node_selected(self, event: Tree.NodeSelected):
        """Handle file selection."""
        # Only handle selections from the file tree, not the JSON tree
        if event.control.id != "file-tree":
            return

        # Make sure it's a file (has dict data), not a directory
        if event.node.data and isinstance(event.node.data, dict) and event.node.parent:
            viewer = self.query_one(FileViewer)
            file_path = event.node.data["file"]
            part = event.node.data["part"]
            viewer.update_content(self.session, file_path, part)

    def on_file_viewer_content_ready(self, event: FileViewer.ContentReady):
        """Handle content ready event by focusing the viewer."""
        viewer = self.query_one(FileViewer)
        viewer.focus_content()

    def action_search(self):
        """Toggle search in the file viewer."""
        viewer = self.query_one(FileViewer)
        viewer.action_search()

    def action_copy_file(self):
        """Copy the current file content to clipboard."""
        viewer = self.query_one(FileViewer)
        if not viewer.current_session or not viewer.current_filename:
            self.app.notify("No file selected")
            return

        content = viewer.current_session.read_file(viewer.current_filename)
        if content is None:
            self.app.notify("Could not read file")
            return

        # For JSONL files with a part, extract just that part and pretty-format
        if viewer.current_filename.endswith('.jsonl') and viewer.current_part:
            lines = [line.strip() for line in content.strip().split('\n') if line.strip()]
            if viewer.current_part == "request" and lines:
                try:
                    data = json.loads(lines[0])
                    content = json.dumps(data, indent=2)
                except json.JSONDecodeError:
                    content = lines[0]
            elif viewer.current_part == "responses" and len(lines) > 1:
                try:
                    responses = [json.loads(line) for line in lines[1:]]
                    if len(responses) == 1:
                        content = json.dumps(responses[0], indent=2)
                    else:
                        content = json.dumps(responses, indent=2)
                except json.JSONDecodeError:
                    content = '\n'.join(lines[1:])
        # Pretty-format regular JSON files too
        elif viewer.current_filename.endswith('.json'):
            try:
                data = json.loads(content)
                content = json.dumps(data, indent=2)
            except json.JSONDecodeError:
                pass

        pyperclip.copy(content)
        self.app.notify("Copied to clipboard")

    def on_key(self, event):
        """Handle left/right navigation between panels."""
        if event.key == "left":
            tree = self.query_one("#file-tree", Tree)
            tree.focus()
        elif event.key == "right":
            viewer = self.query_one(FileViewer)
            viewer.focus_content()


class SessionList(Vertical):
    """Widget for listing available sessions."""

    def __init__(self, sessions: list[DiagnosticsSession]):
        super().__init__()
        self.sessions = sessions

    def compose(self) -> ComposeResult:
        """Create child widgets."""
        yield Static("[bold yellow]Available Diagnostics Sessions[/bold yellow]\n")

        if not self.sessions:
            yield Static("[red]No diagnostics files found[/red]")
        else:
            yield Static(f"[dim]Found {len(self.sessions)} session(s)[/dim]\n")
            yield ListView(id="session-list")

    def on_mount(self):
        """Populate the list after mounting."""
        list_view = self.query_one(ListView)
        for session in self.sessions:
            item = ListItem(
                Label(f"{session.name}\n[dim]{session.zip_path.name}[/dim]"),
                name=session.zip_path.name
            )
            list_view.append(item)


class DiagnosticsApp(App):
    """Diagnostics viewer application."""

    # Disable command palette (Ctrl+\)
    ENABLE_COMMAND_PALETTE = False

    CSS = """
    Screen {
        background: $surface;
    }

    /* Modal styles */
    TextViewerModal {
        align: center middle;
    }

    #modal-container {
        width: 80%;
        height: 80%;
        background: $surface;
        border: thick $primary;
        padding: 1;
    }

    #modal-title {
        background: $primary;
        color: $text;
        padding: 1;
        text-align: center;
        dock: top;
    }

    #modal-scroll {
        height: 1fr;
        border: solid $accent;
        padding: 1;
        margin: 1 0;
    }

    #modal-text {
        width: 100%;
    }

    #modal-footer {
        text-align: center;
        dock: bottom;
    }

    #session-title {
        padding: 1;
        background: $primary;
        color: $text;
        text-align: center;
        height: 3;
    }

    #main-content {
        height: 100%;
    }

    #file-browser {
        width: 30%;
        border-right: solid $primary;
        padding: 1;
    }

    FileViewer {
        width: 70%;
        height: 100%;
    }

    #content-area {
        height: 100%;
        padding: 1;
    }

    JsonTreeView {
        height: 100%;
        scrollbar-gutter: stable;
    }

    #search-container {
        height: 3;
        background: $panel;
        padding: 1;
        border-top: solid $primary;
    }

    #search-label {
        width: auto;
        margin-right: 1;
    }

    #search-input {
        width: 1fr;
        margin-right: 1;
    }

    #search-results {
        width: auto;
    }

    SearchOverlay {
        height: auto;
    }

    #session-list {
        height: 100%;
    }

    ListView {
        background: $surface;
    }

    ListItem {
        padding: 1;
    }

    ListItem:hover {
        background: $primary 30%;
    }

    Tree {
        height: 100%;
    }

    Tree:focus {
        border: solid $accent;
    }
    """

    BINDINGS = [
        Binding("q", "quit", "Quit"),
        Binding("escape", "back", "Back to list"),
        Binding("ctrl+f,cmd+f", "search", "Search", show=False),
    ]

    def __init__(self, diagnostics_dir: Path):
        super().__init__()
        self.diagnostics_dir = diagnostics_dir
        self.sessions = []
        self.current_view = None

    def compose(self) -> ComposeResult:
        """Create child widgets."""
        yield Header()
        yield Footer()

    def on_mount(self):
        """Handle mount event."""
        self.title = "Goose Diagnostics Viewer"
        self.scan_diagnostics()
        self.show_session_list()

    def scan_diagnostics(self):
        """Scan for diagnostics zip files."""
        self.sessions = []

        # Find all diagnostics zip files
        for zip_path in self.diagnostics_dir.glob("diagnostics*.zip"):
            session = DiagnosticsSession(zip_path)
            self.sessions.append(session)

        # Sort by creation time (newest first)
        self.sessions.sort(key=lambda s: s.created_at, reverse=True)

    def show_session_list(self):
        """Show the session list view."""
        if self.current_view:
            self.current_view.remove()

        self.current_view = SessionList(self.sessions)
        self.mount(self.current_view)

    def show_session_viewer(self, session: DiagnosticsSession):
        """Show the session viewer."""
        if self.current_view:
            self.current_view.remove()

        self.current_view = SessionViewer(session)
        self.mount(self.current_view)

    def on_list_view_selected(self, event: ListView.Selected):
        """Handle session selection."""
        # Find the session by zip name
        session_name = event.item.name
        session = next((s for s in self.sessions if s.zip_path.name == session_name), None)
        if session:
            self.show_session_viewer(session)

    def action_back(self):
        """Go back to session list."""
        if isinstance(self.current_view, SessionViewer):
            self.show_session_list()

    def action_quit(self):
        """Quit the application."""
        self.exit()

    def action_search(self):
        """Toggle search."""
        if isinstance(self.current_view, SessionViewer):
            self.current_view.action_search()


def main():
    """Main entry point."""
    # Get diagnostics directory from args or use default
    if len(sys.argv) > 1:
        diagnostics_dir = Path(sys.argv[1]).expanduser()
    else:
        diagnostics_dir = Path.home() / "Downloads"

    if not diagnostics_dir.exists():
        print(f"Error: Directory '{diagnostics_dir}' not found", file=sys.stderr)
        sys.exit(1)

    app = DiagnosticsApp(diagnostics_dir)
    app.run()


if __name__ == "__main__":
    main()
