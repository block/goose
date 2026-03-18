---
title: "使用 Skills"
description: "介绍如何创建和使用可复用的 instruction sets、脚本和资源，让 goose 按需加载。"
sidebar_position: 2
sidebar_label: "使用 Skills"
---

Skills 是一组可复用的指令与资源，用来教会 goose 如何执行特定任务。一个 skill 可以只是简单的检查清单，也可以是带有领域知识的完整工作流，还能包含脚本、模板等配套文件。常见用途包括部署流程、代码评审清单、API 集成指南等。

:::info
这项功能依赖内置的 [Summon 扩展](/zh-CN/docs/mcp/summon-mcp)，需要 `v1.25.0+`。
:::

当会话启动时，goose 会把它发现的 skills 加入到自己的指令中。在会话过程中，只要满足以下任一条件，goose 就会自动加载某个 skill：
- 你的请求和该 skill 的用途明显匹配
- 你明确要求使用某个 skill，例如：
  - “Use the code-review skill to review this PR”
  - “Follow the new-service skill to set up the auth service”
  - “Apply the deployment skill”

你也可以直接让 goose 告诉你当前有哪些 skills 可用。

:::info Claude 兼容性
goose skills 与 Claude Desktop 以及其他[支持 Agent Skills 的 agents](https://agentskills.io/home#adoption)兼容。
:::

## Skill 存放位置

Skills 可以存放在全局目录和 / 或项目目录中。goose 会按以下顺序检查这些目录，并把找到的 skills 合并起来；如果同名 skill 同时存在于多个目录，后出现的目录优先级更高：

1. `~/.claude/skills/`：全局目录，与 Claude Desktop 共享
2. `~/.config/agents/skills/`：全局目录，可跨多种 AI coding agent 复用
3. `~/.config/goose/skills/`：全局目录，goose 专用
4. `./.claude/skills/`：项目级目录，与 Claude Desktop 共享
5. `./.goose/skills/`：项目级目录，goose 专用
6. `./.agents/skills/`：项目级目录，可跨 agent 复用

跨项目通用的流程建议放在全局 skills 中；某个代码库专用的流程则适合放在项目级 skills 中。

## 创建 Skill

如果你有一套会反复执行的流程，而且它包含多个步骤、专门知识或配套文件，就适合创建成一个 skill。

### Skill 文件结构

每个 skill 都放在自己的目录里，并至少包含一个 `SKILL.md` 文件：

```
~/.config/agents/skills/
└── code-review/
    └── SKILL.md
```

`SKILL.md` 文件需要包含带有 `name` 和 `description` 的 YAML frontmatter，后面接 skill 正文：

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

Skill 可以包含脚本、模板、配置文件等辅助文件。把这些文件放在 skill 目录中即可：

```
~/.config/agents/skills/
└── api-setup/
    ├── SKILL.md
    ├── setup.sh
    └── templates/
        └── config.template.json
```

当 goose 加载该 skill 时，它也能看到这些配套文件，并通过 [Developer 扩展](/zh-CN/docs/mcp/developer-mcp) 的文件工具访问它们。

<details>
<summary>带辅助文件的 Skill 示例</summary>

**SKILL.md:**
```markdown
---
name: api-setup
description: Set up API integration with configuration and helper scripts
---

# API Setup

This skill helps you set up a new API integration with our standard configuration.

## Steps

1. Run `setup.sh <api-name>` to create the integration directory
2. Copy `templates/config.template.json` to your integration directory
3. Update the config with your API credentials
4. Test the connection

## Configuration

The config template includes:
- `api_key`: Your API key (get from the provider's dashboard)
- `endpoint`: API endpoint URL
- `timeout`: Request timeout in seconds (default: 30)

## Verification

After setup, verify:
- [ ] Config file is valid JSON
- [ ] API key is set and not a placeholder
- [ ] Test connection succeeds
```

**setup.sh:**
```bash
#!/bin/bash
API_NAME=$1
mkdir -p "integrations/$API_NAME"
cp templates/config.template.json "integrations/$API_NAME/config.json"
echo "Created integration directory for $API_NAME"
echo "Edit integrations/$API_NAME/config.json with your credentials"
```

**templates/config.template.json:**
```json
{
  "api_key": "YOUR_API_KEY_HERE",
  "endpoint": "https://api.example.com/v1",
  "timeout": 30,
  "retry_attempts": 3
}
```

</details>

## 常见使用场景示例

<details>
<summary>部署流程</summary>

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
<summary>测试策略</summary>

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
<summary>API 集成指南</summary>

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

:::tip 其他同样适合复用的 goose 功能
- [.goosehints](/zh-CN/docs/guides/context-engineering/using-goosehints)：适合写通用偏好、项目背景和反复出现的指令，例如 “Always use TypeScript”
- [recipes](/zh-CN/docs/guides/recipes/session-recipes)：适合把指令、prompt 和设置打包成可分享的配置
:::

## 最佳实践

- **让 skill 聚焦单一任务**：一个 workflow 或一个领域一个 skill。内容太长时，考虑拆分。
- **为可执行性而写**：Skill 本质上是给 goose 的操作说明，尽量使用清晰、直接的语言和编号步骤。
- **写上验证步骤**：帮助 goose 在完成后自行确认结果是否正确。

## 更多资源

import ContentCardCarousel from '@site/src/components/ContentCardCarousel';
import skillsvsmcp from '@site/blog/2025-12-22-agent-skills-vs-mcp/skills-vs-mcp.png';

<ContentCardCarousel
  items={[
    {
      type: 'blog',
      title: 'Did Skills Kill MCP?',
      description: '快速了解 Agent Skills 与 MCP 的区别。',
      thumbnailUrl: skillsvsmcp,
      linkUrl: '/goose/blog/2025/12/22/agent-skills-vs-mcp',
      date: '2025-12-22',
      duration: '4 min read'
    }
  ]}
/>
