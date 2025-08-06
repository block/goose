---
sidebar_position: 1
title: Managing Goose Sessions
sidebar_label: Managing Sessions
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { AppWindow, PanelLeft, FolderDot, Paperclip, Copy } from 'lucide-react';


A session is a single, continuous interaction between you and Goose, providing a space to ask questions and prompt action. In this guide, we'll cover how to start, exit, and resume a session. 


## Start Session 

<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
        After choosing an LLM provider, you'll see the session interface ready for use. Type your questions, tasks, or instructions directly into the input field, and Goose will immediately get to work. You can start a new session in the same directory or in a different directory.

        <Tabs>
          <TabItem value="same-directory" label="Same Directory" default>

            To start a session in the same window:
            1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
            2. Click `Home` in the sidebar
            3. Send your first prompt from the chat box

            To start a session in a new window:
            1. Click the <AppWindow className="inline" size={16} /> button in the top-left
            2. In the new Goose window, send your first prompt from the chat box

          </TabItem>
          <TabItem value="diff-directory" label="Different Directory">

            1. Click the <FolderDot className="inline" size={16} /> directory switcher at the bottom of the app
            2. Navigate to the new directory or create a new folder
            3. Click `Open` to open a new Goose window for the selected directory
            4. Send your first prompt from the chat box

          </TabItem>
        </Tabs>

        :::tip
        On macOS, you can drag and drop a folder onto the Goose icon in the dock to open a new session in that directory.
        :::

        You can also use keyboard shortcuts to start a new session or bring focus to open Goose windows.
        
        | Action | macOS | Windows/Linux |
        |--------|-------|---------------|
        | New Session in Current Directory | `Cmd+N`  | `Ctrl+N`  |
        | New Session in Different Directory  | `Cmd+O` | `Ctrl+O` |
        | Focus Goose Window | `Cmd+Option+Shift+G` | `Ctrl+Alt+Shift+G` |

    </TabItem>
    <TabItem value="cli" label="Goose CLI">
        From your terminal, navigate to the directory from which you'd like to start, and run:
        ```sh
        goose session 
        ```

        If you want to interact with Goose in a web-based chat interface, start a session with the [`web`](/docs/guides/goose-cli-commands#web) command:
        ```sh
        goose web --open
        ```
    </TabItem>
</Tabs>

:::info
If this is your first session, Goose will prompt you for an API key to access an LLM (Large Language Model) of your choice. For more information on setting up your API key, see the [Installation Guide](/docs/getting-started/installation#set-llm-provider). Here is the list of [supported LLMs](/docs/getting-started/providers).
:::

## Name Session
<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
        Within the Desktop app, sessions are automatically named based on the context of your initial prompt.
    </TabItem>
    <TabItem value="cli" label="Goose CLI">
        By default, Goose names your session using the current timestamp in the format `YYYYMMDD_HHMMSS`. If you'd like to provide a specific name, this is where you'd do so. For example to name your session `react-migration`, you would run:

        ```
        goose session -n react-migration
        ```

        You'll know your session has started when your terminal looks similar to the following:

        ```
        starting session | provider: openai model: gpt-4o
        logging to ~/.local/share/goose/sessions/react-migration.json1
        ```
    </TabItem>
</Tabs>

## Exit Session
Note that sessions are automatically saved when you exit.
<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
    To exit a session, simply close the application.
    </TabItem>    
    <TabItem value="cli" label="Goose CLI">
        To exit a session, type `exit`. Alternatively, you exit the session by holding down `Ctrl+C`.

        Your session will be stored locally in `~/.local/share/goose/sessions`.
    </TabItem>
</Tabs>

## Search Sessions

Search allows you to find specific content within sessions or find specific sessions.

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>

    You can use keyboard shortcuts and search bar buttons to search sessions in Goose Desktop.

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

     This is a metadata-only search. It doesn't search conversation content. Note that searching by file name is supported (e.g. `20250727_130002.jsonl`), but this property isn't displayed in the UI.

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
  <TabItem value="cli" label="Goose CLI">

    Search functionality is provided by your terminal interface. Use the appropriate shortcut for your environment:

    | Terminal | Operating System | Shortcut |
    |----------|-----------------|-----------|
    | iTerm2 | macOS | `Cmd+F` |
    | Terminal.app | macOS | `Cmd+F` |
    | Windows Terminal | Windows | `Ctrl+F` |
    | Linux Terminal | Linux | `Ctrl+F` |

    :::info
    Your specific terminal emulator may use a different keyboard shortcut. Check your terminal's documentation or settings for the search command.
    :::

    The Goose CLI supports [listing session history](/docs/guides/goose-cli-commands#session-list-options) but doesn't provide search functionality. As a workaround, you can use your terminal's search capabilities (including regex support) to search for specific content within sessions or find specific sessions. 
    
    Examples for macOS:

    ```bash
    # Search session IDs (filenames)
    ls ~/.local/share/goose/sessions/ | grep "full or partial session id"

    # List sessions modified in last 7 days
    find ~/.local/share/goose/sessions/ -mtime -7 -name "*.jsonl"

    # Show first line (metadata) of each session file
    for f in ~/.local/share/goose/sessions/*.jsonl; do
        head -n1 "$f" | grep "your search term" && echo "Found in: $(basename "$f" .jsonl)"
    done

    # Find search term in session content
    rg "your search term" ~/.local/share/goose/sessions/

    # Search and show session IDs that contain search term
    for f in ~/.local/share/goose/sessions/*.jsonl; do
        if grep -q "your search term" "$f"; then
        echo "Found in session: $(basename "$f" .jsonl)"
        fi
    done
    ```

  </TabItem>
</Tabs>

## Resume Session

<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click `History` in the sidebar
    3. Click the session you'd like to resume. Goose provides [search features](#search-sessions) to help you find the session.
    4. Choose how to resume:
       - Click `Resume` to continue in the current window
       - Click `New Window` to open in a new window
    
    :::tip
    You can also quickly resume one of your three most recent sessions by clicking it in the `Recent chats` section on the `Home` page.
    :::

    </TabItem>
    <TabItem value="cli" label="Goose CLI">
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
        While you can resume sessions using the commands above, we recommend creating new sessions for new tasks to reduce the chance of [doom spiraling](/docs/troubleshooting#stuck-in-a-loop-or-unresponsive).
        :::
    </TabItem>
</Tabs>

### Resume Session Across Interfaces

You can resume a CLI session in Desktop.

<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
    All saved sessions are listed in the Desktop app, even CLI sessions. To resume a CLI session within the Desktop:

    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
    2. Click `History` in the sidebar
    3. Click the session you'd like to resume
    4. Choose how to resume:
       - Click `Resume` to continue in the current window
       - Click `New Window` to open in a new window

    </TabItem>
    <TabItem value="cli" label="Goose CLI">
    Currently, you cannot resume a Desktop session within the CLI.
    </TabItem>
</Tabs>

## Project-Based Sessions

<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
        Project-based sessions are only available through the CLI.
    </TabItem>
    <TabItem value="cli" label="Goose CLI">
        You can use the [`project`](/docs/guides/goose-cli-commands#project) and [`projects`](/docs/guides/goose-cli-commands#projects) commands to start or resume sessions from a project, which is a tracked working directory with session metadata. For a complete guide to using Projects, see [Managing Projects Guide](/docs/guides/managing-projects).
    </TabItem>
</Tabs>

## Remove Sessions

<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
        Removing sessions is only available through the CLI.
    </TabItem>
    <TabItem value="cli" label="Goose CLI">
        You can remove sessions using CLI commands. For detailed instructions on session removal, see the [CLI Commands documentation](/docs/guides/goose-cli-commands#session-remove-options).
    </TabItem>
</Tabs>

## Export Sessions

Export sessions to Markdown to share with your team, create documentation, archive conversations, or review them in a readable format.

<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
        Session export is currently only available through the CLI.
    </TabItem>
    <TabItem value="cli" label="Goose CLI">
        Export sessions using the `export` subcommand:

        ```bash
        # Interactive export - prompts you to select a session
        goose session export
        ```
        
        For more details on export options, available flags, and output formats, see the [CLI commands documentation](/docs/guides/goose-cli-commands#session-export-options).
    </TabItem>
</Tabs>

## Voice Dictation
Speak to Goose directly instead of typing your prompts.

<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
    To enable voice dictation:
        1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
        2. Click `Settings` in the sidebar
        3. Click `Chat`
        4. Under `Voice Dictation`, toggle `Enable Voice Dictation` on
        5. Choose between `OpenAI Whisper` or `ElevenLabs` as your dictation provider
        6. Enter your API key for the provider you chose 

    To use voice dictation:
        1. Return to the chat interface (click `Chat` in the sidebar)
        2. Click the microphone on the right of the chat box and begin speaking
        
        The first time you use voice dictation, Goose will request access to your microphone. While recording, you'll see a live waveform of your audio in the input field, a timer, and the current size of your recording. Click the microphone button again to finish recording. 

        **If you don't see the microphone**, check the [models you have configured](/docs/getting-started/providers.md). ElevenLabs can be used as a dictation provider alongside any LLM, but OpenAI Whisper requires that you have an OpenAI model configured in Goose, even if using another LLM provider for chat.  

       #### Important Notes
        * You can record up to 10 minutes or 25MB of audio.
        * The audio is processed by your chosen provider (OpenAI or ElevenLabs).
        * Voice input is appended to any existing text in the text input field, so you can combine typing and speaking your prompts.
        * Recordings are not stored locally after transcription.

  </TabItem>
    <TabItem value="cli" label="Goose CLI">
        Voice dictation is not available in the Goose CLI. 
    </TabItem>
</Tabs>

## Share Files in Session

<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
        Share files with Goose in several ways:

        1. **Drag and Drop**: Simply drag files from your computer's file explorer/finder and drop them anywhere in the chat window. The file paths will be automatically added to your message.

        2. **File Browser**: Click the <Paperclip className="inline" size={16} /> button at the bottom of the app to open your system's file browser and select files.

        3. **Manual Path**: Type or paste the file path directly into the chat input.

        4. **Quick File Search**: Use the [`@` shortcut key](/docs/guides/file-management#quick-file-search-in-goose-desktop) to quickly find and include files.
    </TabItem>
    <TabItem value="cli" label="Goose CLI">
        You can reference files by their paths directly in your messages. Since you're already in a terminal, you can use standard shell commands to help with file paths:

        ```bash
        # Reference a specific file
        What does this code do? ./src/main.rs

        # Use tab completion
        Can you explain the function in ./src/lib<tab>

        # Use shell expansion
        Review these test files: ./tests/*.rs
        ```
    </TabItem>
</Tabs>