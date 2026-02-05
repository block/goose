The messages above are a conversation to summarize. Create a structured context checkpoint summary that another LLM will use to continue the work.

**Conversation History:**
{{ messages }}

Use this EXACT format:

## Goal
[What is the user trying to accomplish? Can be multiple items if the session covers different tasks.]

## Constraints & Preferences
- [Any constraints, preferences, or requirements mentioned by user]
- [Or "(none)" if none were mentioned]

## Progress
### Done
- [x] [Completed tasks/changes with file paths]

### In Progress
- [ ] [Current work]

### Blocked
- [Issues preventing progress, if any]

## Key Decisions
- **[Decision]**: [Brief rationale]

## Next Steps
1. [Ordered list of what should happen next]

## Critical Context
- [Any data, examples, error messages, or references needed to continue]
- [Or "(none)" if not applicable]

RULES:
- Keep each section concise but complete
- Preserve exact file paths, function names, and error messages
- Include code snippets only if they are critical for continuation
- This summary will be read by you (the LLM) to continue the session
- Do not include information that can be easily re-derived from files
