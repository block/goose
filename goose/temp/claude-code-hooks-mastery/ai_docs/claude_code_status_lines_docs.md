[Skip to main content](https://code.claude.com/docs/en/statusline#content-area)

[Claude Code Docs home page![light logo](https://mintcdn.com/claude-code/o69F7a6qoW9vboof/logo/light.svg?fit=max&auto=format&n=o69F7a6qoW9vboof&q=85&s=536eade682636e84231afce2577f9509)![dark logo](https://mintcdn.com/claude-code/o69F7a6qoW9vboof/logo/dark.svg?fit=max&auto=format&n=o69F7a6qoW9vboof&q=85&s=0766b3221061e80143e9f300733e640b)](https://code.claude.com/docs)

![US](https://d3gk2c5xim1je2.cloudfront.net/flags/US.svg)

English

Search...

Ctrl KAsk AI

Search...

Navigation

Configuration

Status line configuration

[Getting started](https://code.claude.com/docs/en/overview) [Build with Claude Code](https://code.claude.com/docs/en/sub-agents) [Deployment](https://code.claude.com/docs/en/third-party-integrations) [Administration](https://code.claude.com/docs/en/setup) [Configuration](https://code.claude.com/docs/en/settings) [Reference](https://code.claude.com/docs/en/cli-reference) [Resources](https://code.claude.com/docs/en/legal-and-compliance)

On this page

- [Create a custom status line](https://code.claude.com/docs/en/statusline#create-a-custom-status-line)
- [How it Works](https://code.claude.com/docs/en/statusline#how-it-works)
- [JSON Input Structure](https://code.claude.com/docs/en/statusline#json-input-structure)
- [Example Scripts](https://code.claude.com/docs/en/statusline#example-scripts)
- [Simple Status Line](https://code.claude.com/docs/en/statusline#simple-status-line)
- [Git-Aware Status Line](https://code.claude.com/docs/en/statusline#git-aware-status-line)
- [Python Example](https://code.claude.com/docs/en/statusline#python-example)
- [Node.js Example](https://code.claude.com/docs/en/statusline#node-js-example)
- [Helper Function Approach](https://code.claude.com/docs/en/statusline#helper-function-approach)
- [Context Window Usage](https://code.claude.com/docs/en/statusline#context-window-usage)
- [Tips](https://code.claude.com/docs/en/statusline#tips)
- [Troubleshooting](https://code.claude.com/docs/en/statusline#troubleshooting)

Make Claude Code your own with a custom status line that displays at the bottom of the Claude Code interface, similar to how terminal prompts (PS1) work in shells like Oh-my-zsh.

## [​](https://code.claude.com/docs/en/statusline\#create-a-custom-status-line)  Create a custom status line

You can either:

- Run `/statusline` to ask Claude Code to help you set up a custom status line. By default, it will try to reproduce your terminal’s prompt, but you can provide additional instructions about the behavior you want to Claude Code, such as `/statusline show the model name in orange`
- Directly add a `statusLine` command to your `.claude/settings.json`:

Copy

Ask AI

```
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/statusline.sh",
    "padding": 0 // Optional: set to 0 to let status line go to edge
  }
}
```

## [​](https://code.claude.com/docs/en/statusline\#how-it-works)  How it Works

- The status line is updated when the conversation messages update
- Updates run at most every 300 ms
- The first line of stdout from your command becomes the status line text
- ANSI color codes are supported for styling your status line
- Claude Code passes contextual information about the current session (model, directories, etc.) as JSON to your script via stdin

## [​](https://code.claude.com/docs/en/statusline\#json-input-structure)  JSON Input Structure

Your status line command receives structured data via stdin in JSON format:

Copy

Ask AI

```
{
  "hook_event_name": "Status",
  "session_id": "abc123...",
  "transcript_path": "/path/to/transcript.json",
  "cwd": "/current/working/directory",
  "model": {
    "id": "claude-opus-4-1",
    "display_name": "Opus"
  },
  "workspace": {
    "current_dir": "/current/working/directory",
    "project_dir": "/original/project/directory"
  },
  "version": "1.0.80",
  "output_style": {
    "name": "default"
  },
  "cost": {
    "total_cost_usd": 0.01234,
    "total_duration_ms": 45000,
    "total_api_duration_ms": 2300,
    "total_lines_added": 156,
    "total_lines_removed": 23
  },
  "context_window": {
    "total_input_tokens": 15234,
    "total_output_tokens": 4521,
    "context_window_size": 200000,
    "used_percentage": 42.5,
    "remaining_percentage": 57.5,
    "current_usage": {
      "input_tokens": 8500,
      "output_tokens": 1200,
      "cache_creation_input_tokens": 5000,
      "cache_read_input_tokens": 2000
    }
  }
}
```

## [​](https://code.claude.com/docs/en/statusline\#example-scripts)  Example Scripts

### [​](https://code.claude.com/docs/en/statusline\#simple-status-line)  Simple Status Line

Copy

Ask AI

```
#!/bin/bash
# Read JSON input from stdin
input=$(cat)

# Extract values using jq
MODEL_DISPLAY=$(echo "$input" | jq -r '.model.display_name')
CURRENT_DIR=$(echo "$input" | jq -r '.workspace.current_dir')

echo "[$MODEL_DISPLAY] 📁 ${CURRENT_DIR##*/}"
```

### [​](https://code.claude.com/docs/en/statusline\#git-aware-status-line)  Git-Aware Status Line

Copy

Ask AI

```
#!/bin/bash
# Read JSON input from stdin
input=$(cat)

# Extract values using jq
MODEL_DISPLAY=$(echo "$input" | jq -r '.model.display_name')
CURRENT_DIR=$(echo "$input" | jq -r '.workspace.current_dir')

# Show git branch if in a git repo
GIT_BRANCH=""
if git rev-parse --git-dir > /dev/null 2>&1; then
    BRANCH=$(git branch --show-current 2>/dev/null)
    if [ -n "$BRANCH" ]; then
        GIT_BRANCH=" | 🌿 $BRANCH"
    fi
fi

echo "[$MODEL_DISPLAY] 📁 ${CURRENT_DIR##*/}$GIT_BRANCH"
```

### [​](https://code.claude.com/docs/en/statusline\#python-example)  Python Example

Copy

Ask AI

```
#!/usr/bin/env python3
import json
import sys
import os

# Read JSON from stdin
data = json.load(sys.stdin)

# Extract values
model = data['model']['display_name']
current_dir = os.path.basename(data['workspace']['current_dir'])

# Check for git branch
git_branch = ""
if os.path.exists('.git'):
    try:
        with open('.git/HEAD', 'r') as f:
            ref = f.read().strip()
            if ref.startswith('ref: refs/heads/'):
                git_branch = f" | 🌿 {ref.replace('ref: refs/heads/', '')}"
    except:
        pass

print(f"[{model}] 📁 {current_dir}{git_branch}")
```

### [​](https://code.claude.com/docs/en/statusline\#node-js-example)  Node.js Example

Copy

Ask AI

```
#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

// Read JSON from stdin
let input = '';
process.stdin.on('data', chunk => input += chunk);
process.stdin.on('end', () => {
    const data = JSON.parse(input);

    // Extract values
    const model = data.model.display_name;
    const currentDir = path.basename(data.workspace.current_dir);

    // Check for git branch
    let gitBranch = '';
    try {
        const headContent = fs.readFileSync('.git/HEAD', 'utf8').trim();
        if (headContent.startsWith('ref: refs/heads/')) {
            gitBranch = ` | 🌿 ${headContent.replace('ref: refs/heads/', '')}`;
        }
    } catch (e) {
        // Not a git repo or can't read HEAD
    }

    console.log(`[${model}] 📁 ${currentDir}${gitBranch}`);
});
```

### [​](https://code.claude.com/docs/en/statusline\#helper-function-approach)  Helper Function Approach

For more complex bash scripts, you can create helper functions:

Copy

Ask AI

```
#!/bin/bash
# Read JSON input once
input=$(cat)

# Helper functions for common extractions
get_model_name() { echo "$input" | jq -r '.model.display_name'; }
get_current_dir() { echo "$input" | jq -r '.workspace.current_dir'; }
get_project_dir() { echo "$input" | jq -r '.workspace.project_dir'; }
get_version() { echo "$input" | jq -r '.version'; }
get_cost() { echo "$input" | jq -r '.cost.total_cost_usd'; }
get_duration() { echo "$input" | jq -r '.cost.total_duration_ms'; }
get_lines_added() { echo "$input" | jq -r '.cost.total_lines_added'; }
get_lines_removed() { echo "$input" | jq -r '.cost.total_lines_removed'; }
get_input_tokens() { echo "$input" | jq -r '.context_window.total_input_tokens'; }
get_output_tokens() { echo "$input" | jq -r '.context_window.total_output_tokens'; }
get_context_window_size() { echo "$input" | jq -r '.context_window.context_window_size'; }

# Use the helpers
MODEL=$(get_model_name)
DIR=$(get_current_dir)
echo "[$MODEL] 📁 ${DIR##*/}"
```

### [​](https://code.claude.com/docs/en/statusline\#context-window-usage)  Context Window Usage

Display the percentage of context window consumed. The `context_window` object contains:

- `total_input_tokens` / `total_output_tokens`: Cumulative totals across the entire session
- `used_percentage`: Pre-calculated percentage of context window used (0-100)
- `remaining_percentage`: Pre-calculated percentage of context window remaining (0-100)
- `current_usage`: Current context window usage from the last API call (may be `null` if no messages yet)

  - `input_tokens`: Input tokens in current context
  - `output_tokens`: Output tokens generated
  - `cache_creation_input_tokens`: Tokens written to cache
  - `cache_read_input_tokens`: Tokens read from cache

You can use the pre-calculated `used_percentage` and `remaining_percentage` fields directly, or calculate from `current_usage` for more control.**Simple approach using pre-calculated percentages:**

Copy

Ask AI

```
#!/bin/bash
input=$(cat)

MODEL=$(echo "$input" | jq -r '.model.display_name')
PERCENT_USED=$(echo "$input" | jq -r '.context_window.used_percentage // 0')

echo "[$MODEL] Context: ${PERCENT_USED}%"
```

**Advanced approach with manual calculation:**

Copy

Ask AI

```
#!/bin/bash
input=$(cat)

MODEL=$(echo "$input" | jq -r '.model.display_name')
CONTEXT_SIZE=$(echo "$input" | jq -r '.context_window.context_window_size')
USAGE=$(echo "$input" | jq '.context_window.current_usage')

if [ "$USAGE" != "null" ]; then
    # Calculate current context from current_usage fields
    CURRENT_TOKENS=$(echo "$USAGE" | jq '.input_tokens + .cache_creation_input_tokens + .cache_read_input_tokens')
    PERCENT_USED=$((CURRENT_TOKENS * 100 / CONTEXT_SIZE))
    echo "[$MODEL] Context: ${PERCENT_USED}%"
else
    echo "[$MODEL] Context: 0%"
fi
```

## [​](https://code.claude.com/docs/en/statusline\#tips)  Tips

- Keep your status line concise - it should fit on one line
- Use emojis (if your terminal supports them) and colors to make information scannable
- Use `jq` for JSON parsing in Bash (see examples above)
- Test your script by running it manually with mock JSON input: `echo '{"model":{"display_name":"Test"},"workspace":{"current_dir":"/test"}}' | ./statusline.sh`
- Consider caching expensive operations (like git status) if needed

## [​](https://code.claude.com/docs/en/statusline\#troubleshooting)  Troubleshooting

- If your status line doesn’t appear, check that your script is executable (`chmod +x`)
- Ensure your script outputs to stdout (not stderr)

Was this page helpful?

YesNo

[Memory management](https://code.claude.com/docs/en/memory) [Customize keyboard shortcuts](https://code.claude.com/docs/en/keybindings)

Ctrl+I

Assistant

Responses are generated using AI and may contain mistakes.