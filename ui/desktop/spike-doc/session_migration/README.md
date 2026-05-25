# ACP Session Migration Docs

Use this folder as a compact working set for the remaining desktop ACP session
migration work.

## Start Here

1. `progress.md` - current status and remaining checklist.
2. `acp-session-migration-plan.md` - overall migration shape and sequencing.
3. `16-acp-message-parity-audit.md` - message/content gaps that can still affect
   user-visible parity.

## Active Implementation Areas

- Recipe and recipe-parameter parity: see `05-conversation-load.md` and
  `15-acp-new-session-plan.md`.
- ACP reattach / active prompt recovery: see `14-acp-reply-spike-plan.md`.
- Elicitation confidence and test gaps: see `17-acp-elicitation-plan.md`.
- Remaining message gaps: see `16-acp-message-parity-audit.md`.

## Reference Docs

- `10-on-load-session-rewrite.md` - backend inline-load design and rollback
  context.
- `mcp-servers-ownership.md` - ownership model for `mcpServers` on ACP session
  creation/load.

## Removed Docs

Completed historical slice docs were deleted after their status was folded into
`progress.md` and `acp-session-migration-plan.md`.
