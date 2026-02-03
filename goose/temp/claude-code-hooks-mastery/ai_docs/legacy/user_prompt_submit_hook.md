# UserPromptSubmit Hook Documentation

The UserPromptSubmit hook is a powerful lifecycle event in Claude Code that fires immediately when a user submits a prompt, before Claude processes it. This hook provides the ability to log, validate, modify, or even block user prompts based on custom criteria.

## Overview

**Hook Name:** `UserPromptSubmit`  
**Fires:** Immediately when a user submits a prompt to Claude Code  
**Can Block:** Yes (exit code 2 with stderr message)  
**Primary Use Cases:** Prompt logging, validation, security filtering, context injection  

## JSON Payload Structure

The UserPromptSubmit hook receives the following JSON payload via stdin:

```json
{
  "hook_event_type": "UserPromptSubmit",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "prompt": "The user's submitted prompt text",
  "timestamp": "2024-01-20T15:30:45.123Z",
  "context": {
    "working_directory": "/path/to/project",
    "git_status": "clean",
    "platform": "darwin"
  }
}
```

### Payload Fields

- **hook_event_type**: Always "UserPromptSubmit" for this hook
- **session_id**: Unique identifier for the Claude Code session
- **prompt**: The exact text the user submitted
- **timestamp**: ISO 8601 timestamp of when the prompt was submitted
- **context**: Additional context about the environment

## Hook Capabilities

### 1. Logging and Auditing

The most basic use case is logging all user prompts for auditing purposes:

```python
# Log user prompt to session directory
log_dir = ensure_session_log_dir(session_id)
log_file = log_dir / 'user_prompt_submit.json'

# Append to existing log
log_data.append(input_data)
with open(log_file, 'w') as f:
    json.dump(log_data, f, indent=2)
```

### 2. Prompt Validation and Blocking

The hook can validate prompts and block them if they violate policies:

```python
def validate_prompt(prompt):
    """Validate the user prompt for security or policy violations."""
    blocked_patterns = [
        ('sudo rm -rf /', 'Dangerous command detected'),
        ('delete all', 'Overly broad deletion request'),
        ('api_key', 'Potential secret exposure risk')
    ]
    
    prompt_lower = prompt.lower()
    
    for pattern, reason in blocked_patterns:
        if pattern.lower() in prompt_lower:
            return False, reason
    
    return True, None

# In main():
is_valid, reason = validate_prompt(prompt)
if not is_valid:
    print(f"Prompt blocked: {reason}", file=sys.stderr)
    sys.exit(2)  # Exit code 2 blocks the prompt
```

### 3. Context Injection

The hook can print additional context that gets prepended to the user's prompt:

```python
# Add context information that will be included in the prompt
print(f"Project: {project_name}")
print(f"Current branch: {git_branch}")
print(f"Time: {datetime.now()}")
```

### 4. Prompt Modification

While the hook cannot directly modify the prompt text, it can provide additional context that effectively changes what Claude sees:

```python
# Example: Add coding standards reminder
if "write code" in prompt.lower():
    print("Remember: Follow PEP 8 style guide and include type hints")
```

## Exit Codes and Flow Control

The UserPromptSubmit hook follows standard Claude Code hook exit code conventions:

| Exit Code | Behavior | Use Case |
|-----------|----------|----------|
| 0 | Success | Prompt is processed normally, stdout content is added as context |
| 2 | Block | Prompt is blocked, stderr message shown to user |
| Other | Non-blocking Error | Error shown to user, prompt still processed |

## Advanced JSON Output Control

Beyond simple exit codes, UserPromptSubmit can return structured JSON:

```json
{
  "decision": "block" | "approve" | undefined,
  "reason": "Explanation for the decision",
  "context": "Additional context to prepend to prompt",
  "suppressOutput": true | false
}
```

- **decision: "block"** - Prevents prompt processing, shows reason to user
- **decision: "approve"** - Explicitly allows prompt (useful for allowlisting)
- **context** - Text to prepend to the user's prompt
- **suppressOutput** - Hide stdout from being added to prompt

## Implementation Examples

### Example 1: Basic Logging

```python
#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///

import json
import sys
from pathlib import Path

# Read input
input_data = json.loads(sys.stdin.read())
session_id = input_data.get('session_id', 'unknown')
prompt = input_data.get('prompt', '')

# Log to file
log_dir = Path(f"logs/{session_id}")
log_dir.mkdir(parents=True, exist_ok=True)
log_file = log_dir / 'user_prompts.json'

# Append prompt
if log_file.exists():
    with open(log_file, 'r') as f:
        prompts = json.load(f)
else:
    prompts = []

prompts.append({
    'timestamp': input_data.get('timestamp'),
    'prompt': prompt
})

with open(log_file, 'w') as f:
    json.dump(prompts, f, indent=2)

sys.exit(0)
```

### Example 2: Security Validation

```python
#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///

import json
import sys
import re

# Security patterns to block
DANGEROUS_PATTERNS = [
    (r'rm\s+-rf\s+/', 'Dangerous system deletion command'),
    (r'curl.*\|\s*sh', 'Unsafe remote script execution'),
    (r'eval\s*\(', 'Unsafe code evaluation'),
    (r'export\s+.*KEY', 'Potential credential exposure'),
]

input_data = json.loads(sys.stdin.read())
prompt = input_data.get('prompt', '')

# Check for dangerous patterns
for pattern, reason in DANGEROUS_PATTERNS:
    if re.search(pattern, prompt, re.IGNORECASE):
        print(f"Security Policy Violation: {reason}", file=sys.stderr)
        sys.exit(2)  # Block the prompt

sys.exit(0)
```

### Example 3: Context Enhancement

```python
#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "python-dotenv",
# ]
# ///

import json
import sys
import os
from datetime import datetime
from dotenv import load_dotenv

load_dotenv()

input_data = json.loads(sys.stdin.read())
prompt = input_data.get('prompt', '')

# Add project context for coding requests
if any(keyword in prompt.lower() for keyword in ['code', 'implement', 'function', 'class']):
    project_name = os.getenv('PROJECT_NAME', 'Unknown Project')
    coding_standards = os.getenv('CODING_STANDARDS', 'Follow best practices')
    
    print(f"Project: {project_name}")
    print(f"Standards: {coding_standards}")
    print(f"Generated at: {datetime.now().isoformat()}")
    print("---")  # Separator

sys.exit(0)
```

### Example 4: Intelligent Prompt Analysis

```python
#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "anthropic",
#     "python-dotenv",
# ]
# ///

import json
import sys
import os
from dotenv import load_dotenv

load_dotenv()

def analyze_prompt_intent(prompt):
    """Use LLM to analyze prompt intent and risks."""
    import anthropic
    
    client = anthropic.Anthropic(api_key=os.getenv("ANTHROPIC_API_KEY"))
    
    analysis_prompt = f"""Analyze this user prompt for potential risks or policy violations:
    
    Prompt: "{prompt}"
    
    Respond with JSON containing:
    - risk_level: "low", "medium", or "high"
    - concerns: list of specific concerns
    - recommendation: "allow", "block", or "warn"
    """
    
    response = client.messages.create(
        model="claude-3-5-haiku-20241022",
        max_tokens=200,
        messages=[{"role": "user", "content": analysis_prompt}]
    )
    
    return json.loads(response.content[0].text)

input_data = json.loads(sys.stdin.read())
prompt = input_data.get('prompt', '')

# Analyze prompt
analysis = analyze_prompt_intent(prompt)

if analysis['recommendation'] == 'block':
    print(f"Blocked: {', '.join(analysis['concerns'])}", file=sys.stderr)
    sys.exit(2)
elif analysis['recommendation'] == 'warn':
    # Add warning as context
    print(f"⚠️  Caution: {', '.join(analysis['concerns'])}")
    print("Please ensure you understand the implications.")
    print("---")

sys.exit(0)
```

## Configuration Options

The UserPromptSubmit hook in this codebase supports several command-line flags:

- **--validate**: Enable prompt validation against security patterns
- **--log-only**: Only log prompts without any validation or blocking
- **--summarize**: Generate AI summaries of prompts (when integrated with observability)

Example configuration in `.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "uv run .claude/hooks/user_prompt_submit.py --log-only"
          },
          {
            "type": "command",
            "command": "uv run .claude/hooks/send_event.py --source-app my-app --event-type UserPromptSubmit --summarize"
          }
        ]
      }
    ]
  }
}
```

## Best Practices

1. **Fast Execution**: Keep processing minimal as this runs on every prompt
2. **Clear Messages**: When blocking, provide clear reasons to the user
3. **Fail Open**: On errors, exit with code 0 to avoid blocking legitimate work
4. **Privacy**: Be mindful of logging sensitive information
5. **Context Relevance**: Only add context that's relevant to the prompt

## Integration with Other Hooks

UserPromptSubmit often works in conjunction with other hooks:

- **PreToolUse**: UserPromptSubmit can set context that influences tool blocking
- **Stop**: Can verify that requested tasks in the prompt were completed
- **Notification**: Can trigger custom notifications based on prompt content

## Security Considerations

1. **Input Sanitization**: Always validate and sanitize prompt content
2. **Log Rotation**: Implement log rotation to prevent unbounded growth
3. **Sensitive Data**: Consider redacting sensitive information in logs
4. **Rate Limiting**: Consider implementing rate limiting for repeated prompts

## Troubleshooting

Common issues and solutions:

1. **Hook Not Firing**: Ensure the hook is properly configured in settings.json
2. **Blocking Not Working**: Verify you're using exit code 2 with stderr
3. **Context Not Added**: Ensure you're printing to stdout, not stderr
4. **JSON Errors**: Always handle JSON parsing errors gracefully

## Summary

The UserPromptSubmit hook provides a powerful interception point for:
- Logging all user interactions
- Enforcing security policies
- Adding contextual information
- Preventing dangerous operations
- Analyzing prompt patterns

When combined with other Claude Code hooks, it forms a comprehensive system for controlling and monitoring AI assistant behavior.