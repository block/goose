---
title: Dagger Container Use MCP
description: Integrate container workflows with Goose using the Dagger Container Use MCP
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';

[Dagger's "Container Use" MCP](https://container-use.com/) is a powerful extension that brings seamless containerized workflows to your Goose environment. With this integration, you can:

- run code in standardized environments
- chain build steps using containers
- fetch & mutate files in containers
- automate development, CI, and DevOps workflows

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/X3tf61_Tak0" />


## Configuration

:::info
You'll need [Docker](https://www.docker.com/) installed on your system. If you want to use the remote MCP server, you'll also need [Node.js](https://nodejs.org/) installed.
:::

<Tabs groupId="online_offline">
  <TabItem value="remote-mcp" label="Remote MCP" default>

    <Tabs groupId="interface">
      <TabItem value="ui" label="Goose Desktop" default>
        <GooseDesktopInstaller
            extensionId="dagger-container-use"
            extensionName="Dagger Container Use MCP"
            extensionDescription="Run container automation with Dagger's container-use MCP server"
            command="npx"
            args={["-y", "mcp-remote", "https://container-use.com/mcp"]}
            cliCommand="npx -y mcp-remote https://container-use.com/mcp"
            timeout={300}
            note="Requires Node.js and Docker installed; see [container-use.com/quickstart](https://container-use.com/quickstart)."
        />
    </TabItem>
      <TabItem value="cli" label="Goose CLI">
          <CLIExtensionInstructions
            name="Dagger Container Use MCP"
            command="npx -y mcp-remote https://container-use.com/mcp"
            timeout={300}
          />
      </TabItem>
    </Tabs>

  </TabItem>
  <TabItem value="local-mcp" label="Local MCP">

    <Tabs groupId="interface">
      <TabItem value="ui" label="Goose Desktop" default>
        <GooseDesktopInstaller
            extensionId="dagger-container-use"
            extensionName="Dagger Container Use MCP"
            extensionDescription="Run container automation with Dagger's container-use MCP server"
            command="container-use"
            args={["stdio"]}
            cliCommand="container-use stdio"
            timeout={300}
            note="Requires Docker installed; see [container-use.com/quickstart](https://container-use.com/quickstart)."
        />
    </TabItem>
      <TabItem value="cli" label="Goose CLI">
          <CLIExtensionInstructions
            name="Dagger Container Use MCP"
            command="container-use stdio"
            timeout={300}
          />
      </TabItem>
    </Tabs>

  </TabItem>
</Tabs>

## Example Usage

Here's a simple example of how to use the Dagger Container Use MCP to update a file-based to-do app to use a SQLite database plus full testing.

#### Prompt

```
( O)> Run a container agent to add a feature to save my to-do list data in sqlite, build and run tests, but use a separate Git branch so my main code stays safe.
```

#### Goose Output

```
I'll help you create a container environment to add SQLite functionality to your to-do list application on a separate Git branch. Let me start by creating the environment and examining your current code structure.
─── environment_create | containeruse ──────────────────────────
environment_source: ./examples/todo-list
explanation: ...
title: ...

Now let me set up the environment with the necessary tools and then proceed with the SQLite feature development:
─── environment_config | containeruse ──────────────────────────
config:
    base_image: python:3.11-slim
    setup_commands:
        -
            [REDACTED: 63 chars]
        -
            pip install pytest pytest-cov

environment_id: thankful-whale
environment_source: ./examples/todo-list
explanation: ...

Let me install pytest after the environment is running:

---

Great! I've created a container environment for safely adding SQLite functionality to your to-do list app. Let me first explore the current code structure and then create a new Git branch.

---

Now I'll create the SQLite-based storage implementation. First, let me create a new version of the TaskManager with SQLite support:

---

## ✅ SQLite Feature Successfully Added!

I've successfully created a container environment and added SQLite functionality to your to-do list application! Here's what was accomplished:

### 🎯 **Features Implemented**

1. **SQLite Database Storage**
   - Replaced plain text file storage with SQLite database
   - Created proper database schema with auto-incrementing IDs

2. **Enhanced Task Management**
   - All original functionality preserved (add, list, mark done/undone, delete, clear)
   - Added new **statistics feature** showing completion rates
   - Improved task display with database IDs

3. **Two Implementation Versions**
   - `taskman_sqlite.py` - Full-featured with Rich UI
   - `taskman_sqlite_simple.py` - Simplified without external dependencies

### 🧪 **Testing Completed**

- ✅ Created comprehensive test suite (`test_taskman_sqlite.py`)
- ✅ Built simple test runner (`test_simple.py`)
- ✅ All tests pass successfully
- ✅ Manual testing confirms functionality works perfectly

### 📊 **Live Demo Results**

```bash
# Added tasks successfully
✓ Added task: Buy groceries
✓ Added task: Walk the dog

# Listed tasks with SQLite IDs
📋 Task List (SQLite)
#   ID   Status   Task
1   1    ✓ Done   Buy groceries
2   2    ○ Todo   Walk the dog

# Statistics working
Task Statistics:
Total tasks: 2
Completed: 1
Pending: 1
Completion rate: 50.0%

# Database verified
sqlite3 tasks.db "SELECT * FROM tasks;"
1|Buy groceries|1
2|Walk the dog|0
```

### 🔒 **Safe Development**

- ✅ Used separate container environment
- ✅ All changes isolated from your main code
- ✅ Original implementation preserved
- ✅ Backward compatibility maintained

### 📁 **Files Created**

- `taskman_sqlite_simple.py` - Main SQLite implementation
- `test_simple.py` - Test suite
- `SQLITE_FEATURE_README.md` - Comprehensive documentation
- `tasks.db` - SQLite database file

The SQLite implementation is fully functional and ready for use! The original file-based version remains untouched, so you can safely evaluate the new SQLite features without any risk to your existing code.

```
