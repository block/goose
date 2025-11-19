---
sidebar_position: 1
title: Session Management
sidebar_label: Session Management
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { AppWindow, PanelLeft, FolderDot, Paperclip, Copy, Edit2, Trash2, Download, Upload } from 'lucide-react';


A session is a single, continuous interaction between you and goose, providing a space to ask questions and prompt action. This guide covers how to manage the session lifecycle.

## Start Session 

:::info First-time setup
In your first session, goose prompts you to [set up an LLM (Large Language Model) provider](/docs/getting-started/installation#set-llm-provider).
:::

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        When you open goose, you'll see the session interface ready for use. Just type&mdash;[or speak](/docs/guides/sessions/in-session-actions#voice-dictation "Learn how to enable voice dictation")&mdash;your questions, requests, or instructions directly into the input field, and goose will immediately get to work. 
        
        When you're ready to work on a new task, you can start a new session in the same directory or a different one. This directory is where goose reads and writes files by default.

        <Tabs>
          <TabItem value="same-directory" label="Same Directory" default>

            To start a session in the same goose window:
            1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
            2. Click `Home` in the sidebar
            3. Send your first prompt from the chat box

            To start a session in a new goose window:
            1. Click the <AppWindow className="inline" size={16} /> button in the top-left
            2. In the new goose window, send your first prompt from the chat box

          </TabItem>
          <TabItem value="diff-directory" label="Different Directory">

            1. Click the <FolderDot className="inline" size={16} /> directory switcher at the bottom of the app
            2. Navigate to the new directory or create a new folder
            3. Click `Open` to open a new goose window for the selected directory
            4. Send your first prompt from the chat box

          </TabItem>
        </Tabs>

        :::tip
        On macOS, you can use the goose dock icon to quickly start sessions:
            - **Drag and drop** a folder onto the goose icon to open a new session in that directory
            - **Right-click** the goose icon and select `New Window` to open a new session in your most recent directory
        :::

        You can also use keyboard shortcuts to start a new session or manage goose windows.
        
        | Action | macOS | Windows/Linux |
        |--------|-------|---------------|
        | New Session with [Quick Launcher](#quick-launcher) | `Cmd+Option+Shift+G` | `Ctrl+Alt+Shift+G` |
        | New Session in Current Directory | `Cmd+N` | `Ctrl+N` |
        | New Session in Current Directory (Same Window) | `Cmd+T` | `Ctrl+T` |
        | New Session in Different Directory | `Cmd+O` | `Ctrl+O` |
        | Toggle Sidebar | `Cmd+B` | `Ctrl+B` |
        | Open Settings | `Cmd+,` | `Ctrl+,` |
        | Keep goose Window Always on Top | `Cmd+Shift+T` | `Ctrl+Shift+T` |
        
        <br />
        #### Quick Launcher
        Start a new session by typing your prompt into a popup:
        1. Press `Cmd+Option+Shift+G` (macOS) or `Ctrl+Alt+Shift+G` (Windows/Linux) to open the popup
        2. Type your prompt and press `Enter`

        The session opens to your most recently opened directory in a new goose window.

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        From your terminal, navigate to the directory from which you'd like to start, and run the [session](/docs/guides/goose-cli-commands#session-options) command:
        ```sh
        goose session 
        ```

        If you want to interact with goose in a web-based chat interface, start a session with the [`web`](/docs/guides/goose-cli-commands#web) command:
        ```sh
        goose web --open
        ```
    </TabItem>
</Tabs>

## Name Session
<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        In the Desktop app, session tiles display auto-generated descriptions based on the context of your initial prompt.

        You can edit session descriptions after they're created:

        1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
        2. Click `History` in the sidebar
        3. Hover over the session you'd like to rename
        4. Click the <Edit2 className="inline" size={16} /> button that appears on the session card
        5. In the "Edit Session Description" modal that opens:
           - Enter your new session description (up to 200 characters)
           - Press `Enter` to save or `Escape` to cancel
           - Or click the `Save` or `Cancel` buttons
        6. A success toast notification will confirm the change

        :::tip
        Session descriptions help you manage multiple goose windows. When you're in the goose chat interface, session descriptions appear in the `Window` menu and in the Dock (macOS) or taskbar (Windows) menu, making it easy to identify and switch between different goose sessions.
        ::: 

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        By default, goose names your session using the current timestamp in the format `YYYYMMDD_<COUNT>`. If you'd like to provide a specific name, this is where you'd do so. For example to name your session `react-migration`, you would run:

        ```
        goose session -n react-migration
        ```

        You'll know your session has started when your terminal looks similar to the following:

        ```
        starting session | provider: openai model: gpt-4o
        session id: react-migration
        working directory: /path/to/your/project
        ```
    </TabItem>
</Tabs>

## Exit Session
Note that sessions are automatically saved when you exit.
<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
    To exit a session, simply close the application.
    </TabItem>    
    <TabItem value="cli" label="goose CLI">
        To exit a session, type `exit`. Alternatively, you exit the session by holding down `Ctrl+C`.

        Your session will be stored in the local SQLite database at `~/.local/share/goose/sessions/sessions.db`.
    </TabItem>
</Tabs>

## Search Sessions

Search allows you to find specific content within sessions or find specific sessions.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

    You can use keyboard shortcuts and search bar buttons to search sessions in goose Desktop.

    | Action | macOS | Windows/Linux |
    |--------|-------|---------------|
    | Open Search | `Cmd+F`  | `Ctrl+F`  |
    | Next Match | `Cmd+G` or `↓` | `Ctrl+G` or `↓` |
    | Previous Match | `Shift+Cmd+G` or `↑` | `Shift+Ctrl+G` or `↑` |
    | Use Selection for Find | `Cmd+E` | n/a |
    | Toggle Case-Sensitivity | `Aa` | `Aa` |
    | Close Search | `Esc` or `X` | `Esc` or `X` |

    The following scenarios are supported:

    #### Search Within Current Session
    
    To find specific content within your current session:

    1. Use `Cmd+F` to open the search bar
    2. Enter your search term
    3. Use shortcuts and search bar buttons to navigate the results

    #### Search For Session By Name or Path
    
    To search all your sessions by name or working directory path:

    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click `History` in the sidebar
    3. Use `Cmd+F` to open the search bar
    4. Enter your search term
    5. Use keyboard shortcuts and search bar buttons to navigate the results (`Cmd+E` not supported)

    This is a metadata-only search. It doesn't search conversation content. Note that searching by session ID (e.g. `20251108_1`) is supported, but this property isn't displayed in the UI.

    :::tip
    You can [rename sessions](#name-session) to give them descriptive names that you'll remember later.
    :::

    #### Search Across All Session Content
    
    To search conversation content across all your sessions, ask goose directly in any chat session. For example:

    - "Find my earlier conversation about React hooks from last week"
    - "Show me sessions where I worked on database migrations"

    goose will search your session history and show relevant conversations with context from matching sessions. 
    
    :::info
    This functionality requires the built-in `Chatrecall` extension to be enabled (it's enabled by default).
    :::

    #### Search Within Historical Session
    
    To find specific content within a historical session:

    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click `History` in the sidebar
    3. Click a specific session tile from the list to view its content
    4. Use `Cmd+F` to open the search bar
    5. Enter your search term
    6. Use keyboard shortcuts and search bar buttons to navigate the results

    :::info No Regex or operator support
    Using regular expressions or search operators in search text isn't supported.
    :::

  </TabItem>
  <TabItem value="cli" label="goose CLI">

    #### Search Within Current Session

    Search functionality is provided by your terminal interface. Use the appropriate shortcut for your environment:

    | Terminal | Operating System | Shortcut |
    |----------|-----------------|-----------|
    | iTerm2 | macOS | `Cmd+F` |
    | Terminal.app | macOS | `Cmd+F` |
    | Windows Terminal | Windows | `Ctrl+F` |
    | Linux Terminal | Linux | `Ctrl+F` |

    To find specific content within your current session:

    1. Use the shortcut to open the search bar
    2. Enter your search term
    3. Use shortcuts and search bar buttons to navigate the results

    :::info
    Your specific terminal emulator may use a different keyboard shortcut. Check your terminal's documentation or settings for the search command.
    :::

    #### Search Across All Session Content
    
    To search conversation content across all your sessions, start a goose session and ask directly:

    - "Find my earlier conversation about React hooks from last week"
    - "Show me sessions where I worked on database migrations"

    goose will search your session history and show relevant conversations with context from matching sessions.
    
    :::info
    This functionality requires the built-in `Chatrecall` extension to be enabled (it's enabled by default).
    :::

    #### Search Session Data Directly
    
    The [`session list`](/docs/guides/goose-cli-commands#session-list-options) subcommand with supported options can be useful for some search operations.

    You can also query the SQLite database directly:

    ```bash
    # Search session descriptions
    sqlite3 ~/.local/share/goose/sessions/sessions.db \
      "SELECT id, description FROM sessions WHERE description LIKE '%your search term%';"

    # Search by working directory
    sqlite3 ~/.local/share/goose/sessions/sessions.db \
      "SELECT id, description, working_dir FROM sessions WHERE working_dir LIKE '%project-name%';"

    # List recent sessions
    sqlite3 ~/.local/share/goose/sessions/sessions.db \
      "SELECT id, description, created_at FROM sessions ORDER BY created_at DESC LIMIT 10;"
    ```

    :::info Session Storage Migration
    Starting with version 1.10.0, goose uses a SQLite database (`sessions.db`) instead of individual `.jsonl` files. Legacy `.jsonl` files remain on disk but are no longer managed by goose.
    :::

  </TabItem>
</Tabs>

## Resume Session

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click `History` in the sidebar
    3. Click the session you'd like to resume. goose provides [search features](#search-sessions) to help you find the session.
    4. Choose how to resume:
       - Click `Resume` to continue in the current window
       - Click `New Window` to open in a new window
    
    :::tip
    You can also quickly resume one of your three most recent sessions by clicking it in the `Recent chats` section on the `Home` page.
    :::

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        To resume your latest session, you can run the following command:

        ```
         goose session -r
        ```

        To resume a specific session, run the following command: 

        ```
        goose session -r --name <name>
        ```
        For example, to resume the session named `react-migration`, you would run:

        ```
        goose session -r --name react-migration
        ```

        :::tip
        While you can resume sessions using the commands above, we recommend creating new sessions for new tasks to reduce the chance of [doom spiraling](/docs/troubleshooting/known-issues#stuck-in-a-loop-or-unresponsive).
        :::
    </TabItem>
</Tabs>

### Resume Session Across Interfaces

You can resume a CLI session in Desktop.

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
    All saved sessions are listed in the Desktop app, even CLI sessions. To resume a CLI session within the Desktop:

    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click `History` in the sidebar
    3. Click the session you'd like to resume
    4. Choose how to resume:
       - Click `Resume` to continue in the current window
       - Click `New Window` to open in a new window

    </TabItem>
    <TabItem value="cli" label="goose CLI">
    Currently, you cannot resume a Desktop session within the CLI.
    </TabItem>
</Tabs>

### Resume Project-Based Sessions

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        Project-based sessions are only available through the CLI.
    </TabItem>
    <TabItem value="cli" label="goose CLI">
        You can use the [`project`](/docs/guides/goose-cli-commands#project) and [`projects`](/docs/guides/goose-cli-commands#projects) commands to start or resume sessions from a project, which is a tracked working directory with session metadata. For a complete guide to using Projects, see [Managing Projects Guide](/docs/guides/managing-projects).
    </TabItem>
</Tabs>

## Delete Sessions

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        You can delete sessions directly from the Desktop app:

        1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
        2. Click `History` in the sidebar
        3. Find the session you want to delete
        4. Hover over the session card to reveal the action buttons
        5. Click the <Trash2 className="inline" size={16} /> button that appears
        6. Confirm the deletion in the modal that appears

        :::warning Permanent deletion
        Deleting a session from goose Desktop will also delete it from the CLI. This action cannot be undone.
        :::

        The session will be immediately removed from your session history and the underlying session record will be deleted from local storage.
    </TabItem>
    <TabItem value="cli" label="goose CLI">
        You can remove sessions using CLI commands. For detailed instructions on session removal, see the [CLI Commands documentation](/docs/guides/goose-cli-commands#session-remove-options).
    </TabItem>
</Tabs>

## Import Sessions

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        Import complete sessions from JSON files to restore, share, or migrate sessions between goose instances. Importing creates a new session with a new ID rather than overwriting existing sessions.

        1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
        2. Click `History` in the sidebar
        3. Click the <Upload className="inline" size={16} /> `Import Session` button in the top-right corner
        4. Select a `.json` session file that was previously exported from goose
        5. The session will be imported with a new session ID
        6. A success notification will confirm the import

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        Session import is currently only available through the Desktop app.
    </TabItem>
</Tabs>

## Export Sessions

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        Export complete sessions as JSON files for backup, sharing, migration, or archival. Exported files preserve all session data including conversation history, metadata, and settings.

        1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
        2. Click `History` in the sidebar
        3. Find the session you want to export
        4. Hover over the session card to reveal the action buttons
        5. Click the <Download className="inline" size={16} /> button that appears
        6. The session will be downloaded as a `.json` file named after the session description

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        Export sessions for backup, sharing, migration, or documentation purposes. You can export as JSON files to preserve complete session data including conversation history, metadata, and settings, or as Markdown files to get a formatted, readable version of the conversation.

        From your terminal, run the [`session export`](/docs/guides/goose-cli-commands#session-export-options) subcommand:
        
        ```bash
        goose session export
        ```

    </TabItem>
</Tabs>