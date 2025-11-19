# Claude Desktop Session File Specification

## Overview

Claude Desktop stores conversation sessions in `~/.claude/projects/` as JSONL (JSON Lines) files. This document specifies the format and how to extract session information.

## Directory Structure

```
~/.claude/projects/
├── -Users-micn-Documents/              # Project directory (working_dir encoded)
│   ├── b0c8fba6-xxxx.jsonl           # Main session file (UUID)
│   ├── agent-2c1e5286.jsonl          # Agent sidechain file
│   └── agent-d4d379b8.jsonl          # Agent sidechain file
├── -Users-micn/                       # Another project directory
└── -Users-micn-Documents-code-opencode/
```

### Working Directory Encoding

Project directories are encoded from filesystem paths:
- Replace `/` with `-`
- Remove leading `/`
- Examples:
  - `/Users/micn/Documents` → `-Users-micn-Documents`
  - `/Users/micn` → `-Users-micn`
  - `/Users/micn/Documents/code/opencode` → `-Users-micn-Documents-code-opencode`

### File Naming

- **Main sessions**: `{uuid}.jsonl` (e.g., `b0c8fba6-0600-4013-bdcf-2d6d41bb48d6.jsonl`)
- **Agent sidechains**: `agent-{short-id}.jsonl` (e.g., `agent-2c1e5286.jsonl`)

## JSONL Format

Each line is a separate JSON object representing an event in the conversation. Lines are **appended** as the session progresses.

### Event Types

#### 1. File History Snapshot
```json
{
  "type": "file-history-snapshot",
  "messageId": "f4ae33a8-d2fa-4271-a10f-cd1ab70393b8",
  "snapshot": {
    "messageId": "f4ae33a8-d2fa-4271-a10f-cd1ab70393b8",
    "trackedFileBackups": {},
    "timestamp": "2025-11-19T04:55:17.472Z"
  },
  "isSnapshotUpdate": false
}
```

#### 2. User Message
```json
{
  "parentUuid": null,
  "isSidechain": false,
  "userType": "external",
  "cwd": "/Users/micn/Documents",
  "sessionId": "b0c8fba6-0600-4013-bdcf-2d6d41bb48d6",
  "version": "2.0.33",
  "gitBranch": "",
  "type": "user",
  "message": {
    "role": "user",
    "content": "tell me a joke"
  },
  "uuid": "f4ae33a8-d2fa-4271-a10f-cd1ab70393b8",
  "timestamp": "2025-11-19T04:55:17.465Z",
  "thinkingMetadata": {
    "level": "high",
    "disabled": false,
    "triggers": []
  }
}
```

#### 3. Assistant Message (with thinking)
```json
{
  "parentUuid": "f4ae33a8-d2fa-4271-a10f-cd1ab70393b8",
  "isSidechain": false,
  "userType": "external",
  "cwd": "/Users/micn/Documents",
  "sessionId": "b0c8fba6-0600-4013-bdcf-2d6d41bb8d6",
  "version": "2.0.33",
  "gitBranch": "",
  "message": {
    "model": "claude-sonnet-4-5-20250929",
    "id": "msg_011UriXkmnza4iWurCKPaoKN",
    "type": "message",
    "role": "assistant",
    "content": [
      {
        "type": "thinking",
        "thinking": "The user is asking me to tell them a joke...",
        "signature": "EtgCCkYICRgCKkBn..."
      }
    ],
    "stop_reason": null,
    "stop_sequence": null,
    "usage": {
      "input_tokens": 9,
      "cache_creation_input_tokens": 3819,
      "cache_read_input_tokens": 12180,
      "output_tokens": 5
    }
  },
  "requestId": "req_011CVGenpnQWECsLDd6oTbWi",
  "type": "assistant",
  "uuid": "c9ae1dd9-e3f2-467b-845c-0721dfb68927",
  "timestamp": "2025-11-19T04:55:20.331Z"
}
```

#### 4. Assistant Message (with text response)
```json
{
  "parentUuid": "c9ae1dd9-e3f2-467b-845c-0721dfb68927",
  "isSidechain": false,
  "userType": "external",
  "cwd": "/Users/micn/Documents",
  "sessionId": "b0c8fba6-0600-4013-bdcf-2d6d41bb48d6",
  "version": "2.0.33",
  "gitBranch": "",
  "message": {
    "model": "claude-sonnet-4-5-20250929",
    "id": "msg_011UriXkmnza4iWurCKPaoKN",
    "type": "message",
    "role": "assistant",
    "content": [
      {
        "type": "text",
        "text": "Why do programmers prefer dark mode?\n\nBecause light attracts bugs."
      }
    ],
    "stop_reason": null,
    "stop_sequence": null,
    "usage": { ... }
  },
  "requestId": "req_011CVGenpnQWECsLDd6oTbWi",
  "type": "assistant",
  "uuid": "b95005b3-9585-4eb2-ad4d-d0d710080dbf",
  "timestamp": "2025-11-19T04:55:20.661Z"
}
```

#### 5. Assistant Message (with tool use)
```json
{
  "parentUuid": "...",
  "isSidechain": false,
  "userType": "external",
  "cwd": "/Users/micn/Documents",
  "sessionId": "b0c8fba6-0600-4013-bdcf-2d6d41bb48d6",
  "version": "2.0.33",
  "gitBranch": "",
  "message": {
    "model": "claude-sonnet-4-5-20250929",
    "id": "msg_01WAbrwQwxsoDMRwjK5KaoVo",
    "type": "message",
    "role": "assistant",
    "content": [
      {
        "type": "tool_use",
        "id": "toolu_012t21FRoavGupuXYcxifgaq",
        "name": "Bash",
        "input": {
          "command": "ls -la",
          "description": "List files in current directory"
        }
      }
    ],
    "stop_reason": null,
    "stop_sequence": null,
    "usage": { ... }
  },
  "requestId": "req_011CVGf7HHrYQiA2WrQk2fMq",
  "type": "assistant",
  "uuid": "692fb9db-e44d-4605-9d65-2901e5e36512",
  "timestamp": "2025-11-19T04:59:27.653Z"
}
```

#### 6. User Message (with tool result)
```json
{
  "parentUuid": "692fb9db-e44d-4605-9d65-2901e5e36512",
  "isSidechain": false,
  "userType": "external",
  "cwd": "/Users/micn/Documents",
  "sessionId": "b0c8fba6-0600-4013-bdcf-2d6d41bb48d6",
  "version": "2.0.33",
  "gitBranch": "",
  "type": "user",
  "message": {
    "role": "user",
    "content": [
      {
        "tool_use_id": "toolu_012t21FRoavGupuXYcxifgaq",
        "type": "tool_result",
        "content": "total 772984\ndrwx------+ 76 micn  staff  ...",
        "is_error": false
      }
    ]
  },
  "uuid": "e71d3e54-c7b9-47f7-b9dd-77cb41fa4420",
  "timestamp": "2025-11-19T04:59:27.764Z",
  "toolUseResult": {
    "stdout": "total 772984\ndrwx------+ 76 micn  staff  ...",
    "stderr": "",
    "interrupted": false,
    "isImage": false
  }
}
```

## Key Fields

### Session Identification

- **`sessionId`**: UUID identifying the main conversation session
- **`isSidechain`**: `false` for main session, `true` for agent sidechains
- **`agentId`**: Only present in sidechain files (e.g., `"2c1e5286"`)
- **`cwd`**: Current working directory for the session

### Message Chain

- **`uuid`**: Unique identifier for this message/event
- **`parentUuid`**: Links to the previous message in the chain (null for first message)
- **`timestamp`**: ISO 8601 timestamp

### Message Content

User messages can have:
- Simple string content: `"content": "tell me a joke"`
- Tool result array: `"content": [{"tool_use_id": "...", "type": "tool_result", ...}]`

Assistant messages can have multiple content blocks:
- `{"type": "thinking", "thinking": "...", "signature": "..."}` - Extended thinking
- `{"type": "text", "text": "..."}` - Text response
- `{"type": "tool_use", "id": "...", "name": "...", "input": {...}}` - Tool invocation

## Identifying Sessions and Files

### Finding Latest Sessions

```bash
# Find all main session files (not agents) sorted by modification time
find ~/.claude/projects/ -type f -name "*.jsonl" ! -name "agent-*" -exec ls -lt {} + | head -n 10
```

### Extracting Working Directory

```python
import os

def decode_working_dir(project_dir_name: str) -> str:
    """Convert project directory name to working directory path.
    
    Example: '-Users-micn-Documents' -> '/Users/micn/Documents'
    """
    # Remove leading dash and replace remaining dashes with slashes
    if project_dir_name.startswith('-'):
        project_dir_name = project_dir_name[1:]
    return '/' + project_dir_name.replace('-', '/')

# Or simpler:
working_dir = '/' + project_dir_name.lstrip('-').replace('-', '/')
```

### Identifying Session Type

```python
def is_main_session(data: dict) -> bool:
    """Check if this is a main session (not a sidechain)."""
    return data.get('isSidechain') == False and data.get('agentId') is None

def is_agent_session(data: dict) -> bool:
    """Check if this is an agent sidechain."""
    return data.get('isSidechain') == True and data.get('agentId') is not None
```

### Extracting Messages

```python
import json

def parse_session_file(filepath: str):
    """Parse a session JSONL file and extract messages."""
    messages = []
    
    with open(filepath, 'r') as f:
        for line in f:
            data = json.loads(line)
            
            # Skip file history snapshots
            if data.get('type') == 'file-history-snapshot':
                continue
            
            # Extract user messages
            if data.get('type') == 'user':
                msg = {
                    'role': 'user',
                    'timestamp': data['timestamp'],
                    'uuid': data['uuid'],
                    'content': data['message']['content']
                }
                messages.append(msg)
            
            # Extract assistant messages
            elif data.get('type') == 'assistant':
                for content_block in data['message'].get('content', []):
                    msg = {
                        'role': 'assistant',
                        'timestamp': data['timestamp'],
                        'uuid': data['uuid'],
                        'content_type': content_block.get('type')
                    }
                    
                    if content_block['type'] == 'text':
                        msg['text'] = content_block['text']
                    elif content_block['type'] == 'thinking':
                        msg['thinking'] = content_block['thinking']
                    elif content_block['type'] == 'tool_use':
                        msg['tool_name'] = content_block['name']
                        msg['tool_id'] = content_block['id']
                        msg['tool_input'] = content_block['input']
                    
                    messages.append(msg)
    
    return messages
```

### Extracting Tool Usage

```python
def extract_tool_interactions(filepath: str):
    """Extract all tool use and tool result pairs."""
    tool_interactions = []
    
    with open(filepath, 'r') as f:
        for line in f:
            data = json.loads(line)
            
            if data.get('type') == 'assistant':
                for content in data['message'].get('content', []):
                    if content.get('type') == 'tool_use':
                        interaction = {
                            'tool_id': content['id'],
                            'tool_name': content['name'],
                            'tool_input': content['input'],
                            'timestamp': data['timestamp'],
                            'result': None
                        }
                        tool_interactions.append(interaction)
            
            elif data.get('type') == 'user':
                content = data['message'].get('content', [])
                if isinstance(content, list):
                    for item in content:
                        if isinstance(item, dict) and item.get('type') == 'tool_result':
                            # Find matching tool use
                            tool_id = item['tool_use_id']
                            for interaction in tool_interactions:
                                if interaction['tool_id'] == tool_id:
                                    interaction['result'] = {
                                        'content': item['content'],
                                        'is_error': item.get('is_error', False),
                                        'timestamp': data['timestamp']
                                    }
                                    if 'toolUseResult' in data:
                                        interaction['result'].update(data['toolUseResult'])
    
    return tool_interactions
```

## Complete Workflow: Finding Latest N Sessions

```python
import os
import json
from pathlib import Path
from datetime import datetime

def get_latest_sessions(n: int = 10):
    """Get the latest N Claude Desktop sessions with their details."""
    
    projects_dir = Path.home() / ".claude" / "projects"
    sessions = []
    
    # Find all main session files
    for project_dir in projects_dir.iterdir():
        if not project_dir.is_dir():
            continue
        
        # Decode working directory
        working_dir = '/' + project_dir.name.lstrip('-').replace('-', '/')
        
        # Find session files (not agent files)
        for session_file in project_dir.glob("*.jsonl"):
            if session_file.name.startswith("agent-"):
                continue
            
            # Get file modification time
            mtime = session_file.stat().st_mtime
            
            # Parse first line to get session info
            with open(session_file, 'r') as f:
                for line in f:
                    data = json.loads(line)
                    if data.get('sessionId'):
                        sessions.append({
                            'session_id': data['sessionId'],
                            'working_dir': working_dir,
                            'file_path': str(session_file),
                            'last_modified': datetime.fromtimestamp(mtime),
                            'cwd': data.get('cwd')
                        })
                        break
    
    # Sort by modification time (most recent first)
    sessions.sort(key=lambda x: x['last_modified'], reverse=True)
    
    # Return top N
    return sessions[:n]

def get_session_messages(session_file: str):
    """Get all messages from a session file."""
    messages = []
    
    with open(session_file, 'r') as f:
        for line in f:
            data = json.loads(line)
            
            if data.get('type') in ['user', 'assistant']:
                msg = {
                    'type': data['type'],
                    'timestamp': data['timestamp'],
                    'uuid': data['uuid'],
                    'session_id': data.get('sessionId'),
                    'cwd': data.get('cwd')
                }
                
                if data['type'] == 'user':
                    msg['content'] = data['message']['content']
                else:  # assistant
                    msg['content_blocks'] = data['message']['content']
                    msg['model'] = data['message'].get('model')
                
                messages.append(msg)
    
    return messages

# Usage
if __name__ == "__main__":
    sessions = get_latest_sessions(5)
    
    for session in sessions:
        print(f"Session: {session['session_id']}")
        print(f"  Working dir: {session['working_dir']}")
        print(f"  Last modified: {session['last_modified']}")
        print(f"  File: {session['file_path']}")
        
        messages = get_session_messages(session['file_path'])
        print(f"  Messages: {len(messages)}")
        print()
```

## Notes

1. **Duplicate Entries**: Files may contain duplicate entries (possibly due to write buffering or flushes). Handle by deduplication on `uuid` + `timestamp`.

2. **Agent Sidechains**: Agent files share the same `sessionId` as their parent main session but have `isSidechain: true` and an `agentId` field.

3. **File Growth**: Session files grow by appending new lines. No in-place modifications occur.

4. **Working Directory vs CWD**: 
   - The project directory name encodes where Claude Desktop was opened
   - The `cwd` field in each message shows the actual working directory at that moment

5. **Message Chaining**: Use `parentUuid` to reconstruct the conversation tree structure.

6. **Tool Results**: Tool results appear as user messages with `type: "tool_result"` in the content array, and include an additional `toolUseResult` field with structured output.
