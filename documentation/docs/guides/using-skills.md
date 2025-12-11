---
title: Using Skills
sidebar_position: 41
sidebar_label: Using Skills
---

Skills are reusable sets of instructions and resources that teach goose how to perform specific tasks. A skill can range from simple steps to detailed workflows, and can include domain expertise and supporting files like scripts or templates. Example use cases include deployment procedures, code review checklists, and API integration guides.

:::info
This functionality requires the built-in [Skills extension](/docs/mcp/skills-mcp) to be enabled (it's enabled by default).
:::

When a session starts, goose adds any skills that it discovers to its instructions. During the session, goose automatically loads a skill when:
- Your request clearly matches a skill's purpose
- You explicitly ask to use a skill, for example:
  - "Use the code-review skill to review this PR"
  - "Follow the new-service skill to set up the auth service"
  - "Apply the deployment skill"

You can also ask goose what skills are available.

:::tip Other goose features that support reuse
- [.goosehints](/docs/guides/using-goosehints): Best for general preferences, project context, and repeated instructions like "Always use TypeScript"
- [recipes](/docs/guides/recipes/session-recipes): Shareable configurations that package instructions, prompts, and settings together
:::

## Creating a Skill

Create a skill when you have a repeatable workflow that involves multiple steps, specialized knowledge, or supporting files.

### Skill Locations

Skills can be stored globally and/or per-project. goose checks all of these directories in order and combines what it finds. If the same skill name exists in multiple directories, the latest directory takes priority:

1. `~/.claude/skills/` — Global, shared with Claude Desktop
2. `~/.config/goose/skills/` — Global, goose-specific
3. `./.claude/skills/` — Current directory, shared with Claude Desktop
4. `./.goose/skills/` — Current directory, goose-specific (highest priority)

Use global skills for workflows you use across projects. Use project-specific skills for procedures unique to a codebase.

### Skill File Structure

Each skill lives in its own directory with a `SKILL.md` file:

```
~/.config/goose/skills/
└── code-review/
    └── SKILL.md
```

A `SKILL.md` file requires YAML frontmatter with `name` and `description`, followed by the skill content:

```markdown
---
name: code-review
description: Comprehensive code review checklist for pull requests
---

# Code Review Checklist

When reviewing code, check each of these areas:

## Functionality
- [ ] Code does what the PR description claims
- [ ] Edge cases are handled
- [ ] Error handling is appropriate

## Code Quality
- [ ] Follows project style guide
- [ ] No hardcoded values that should be configurable
- [ ] Functions are focused and well-named

## Testing
- [ ] New functionality has tests
- [ ] Tests are meaningful, not just for coverage
- [ ] Existing tests still pass

## Security
- [ ] No credentials or secrets in code
- [ ] User input is validated
- [ ] SQL queries are parameterized
```

### Supporting Files

Skills can include supporting files like scripts, templates, or configuration files. Place them in the skill directory:

```
~/.config/goose/skills/
└── api-setup/
    ├── SKILL.md
    ├── setup.sh
    └── templates/
        └── config.template.json
```

When goose loads the skill, it sees the supporting files and can access them using the [Developer extension's](/docs/mcp/developer-mcp) file tools.

<details>
<summary>Example Skill with Supporting Files</summary>

**Directory structure:**
```
~/.config/goose/skills/
└── new-service/
    ├── SKILL.md
    ├── init-service.sh
    └── templates/
        ├── Dockerfile
        ├── docker-compose.yml
        └── .env.template
```

**SKILL.md:**
```markdown
---
name: new-service
description: Bootstrap a new microservice with standard configuration
---

# New Service Setup

This skill helps you create a new microservice with our standard stack.

## Steps

1. Run `init-service.sh <service-name>` to create the directory structure
2. Copy templates from `templates/` directory
3. Update `.env.template` with service-specific values
4. Build and test with `docker-compose up`

## Configuration

The templates use these placeholder values that need to be replaced:
- `{{SERVICE_NAME}}` - Name of the service
- `{{PORT}}` - Port the service runs on
- `{{DATABASE_URL}}` - Connection string for the database

## Verification

After setup, verify:
- [ ] Service starts without errors
- [ ] Health endpoint responds at `/health`
- [ ] Logs are properly formatted
```

**init-service.sh:**
```bash
#!/bin/bash
SERVICE_NAME=$1
mkdir -p "$SERVICE_NAME"/{src,tests,config}
echo "Created service structure for $SERVICE_NAME"
```

</details>

## Common Use Case Examples

<details>
<summary>Deployment Workflow</summary>

```markdown
---
name: production-deploy
description: Safe deployment procedure for production environment
---

# Production Deployment

## Pre-deployment
1. Ensure all tests pass
2. Get approval from at least 2 reviewers
3. Notify #deployments channel

## Deploy
1. Create release branch from main
2. Run `npm run build:prod`
3. Deploy to staging, verify, then production
4. Monitor error rates for 30 minutes

## Rollback
If error rate exceeds 1%:
1. Revert to previous deployment
2. Notify #incidents channel
3. Create incident report
```

</details>

<details>
<summary>Testing Strategy</summary>

```markdown
---
name: testing-strategy
description: Guidelines for writing effective tests in this project
---

# Testing Guidelines

## Unit Tests
- Test one thing per test
- Use descriptive test names: `test_user_creation_fails_with_invalid_email`
- Mock external dependencies

## Integration Tests
- Test API endpoints with realistic data
- Verify database state changes
- Clean up test data after each test

## Running Tests
- `npm test` — Run all tests
- `npm test:unit` — Unit tests only
- `npm test:integration` — Integration tests (requires database)
```

</details>

<details>
<summary>API Integration Guide</summary>

````markdown
---
name: square-integration
description: How to integrate with our Square account
---

# Square Integration

## Authentication
- Test key: Use `SQUARE_TEST_KEY` from `.env.test`
- Production key: In 1Password under "Square Production"

## Common Operations

### Create a customer
```javascript
const customer = await squareup.customers.create({
  email: user.email,
  metadata: { userId: user.id }
});
```

### Handle webhooks
Always verify webhook signatures. See `src/webhooks/square.js` for our handler pattern.

## Error Handling
- `card_declined`: Show user-friendly message, suggest different payment method
- `rate_limit`: Implement exponential backoff
- `invalid_request`: Log full error, likely a bug in our code
````

</details>

## Best Practices

- **Keep skills focused** — One skill per workflow or domain. If a skill is getting long, consider splitting it.
- **Write for clarity** — Skills are instructions for goose. Use clear, direct language and numbered steps.
- **Include verification steps** — Help goose confirm the workflow completed successfully.

## Claude Compatibility

goose skills are compatible with Claude Desktop and Claude skills are compatible with goose.

You can also create goose-specific skills in `~/.config/goose/skills/` if you want different behavior between the tools. Skills in the current directory's `.goose/skills/` take precedence over `.claude/skills/` when both exist.
