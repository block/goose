@AGENTS.md
# Rules for automated bug fixes
- Read ticket.json for the bug description.
- Reproduce the bug with a failing test first, then fix it, then run the full test suite.
- Keep the change minimal (under ~100 lines). No refactoring, no dependency upgrades.
- Never modify: .github/, infra/, auth/, payments/, migrations/, CI config, or test thresholds.
- Do not commit ticket.json. Do not run git commit or git push; the workflow does that.
- If the bug is unclear, can't be reproduced, or needs a forbidden area, make no changes and explain why.
