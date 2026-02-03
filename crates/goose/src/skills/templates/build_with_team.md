## Build Phase
Execute the implementation with builder/validator pairing.

### Builder: {{BUILDER_NAME}}
**Capabilities:**
- Full tool access (Write, Edit, Bash)
- Auto-validates with configured validators (Ruff, Clippy, etc.)
- Creates implementation artifacts

**Tasks:**
{{BUILDER_TASKS}}

### Validator: {{VALIDATOR_NAME}}
**Capabilities:**
- Read-only access (Read, Grep, Glob)
- Can execute tests
- Cannot modify files

**Acceptance Criteria:**
{{ACCEPTANCE_CRITERIA}}

### Workflow
1. Builder implements the feature
2. Auto-validators run on each file change
3. Builder signals completion
4. Validator verifies all acceptance criteria
5. If validation fails, builder receives feedback and iterates
6. On success, task is marked complete

### Validation Commands
{{VALIDATION_COMMANDS}}
