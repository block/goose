# Responsible AI-Assisted Coding Guide  
_Guidelines for contributing responsibly to goose during Hacktoberfest_

goose benefits from thoughtful AI assisted development, but contributors must maintain high standards for code quality, security, and collaboration. Whether you use goose, Copilot, Claude, or other AI tools, these principles will help you avoid common pitfalls.

---

## Core Principles

- **Human Oversight**: You are accountable for all code you submit. Never commit code you don’t understand or can’t maintain.  
- **Quality Standards**: AI code must meet the same standards as human written code—tests, docs, and patterns included.  
- **Transparency**: Be open about significant AI usage in PRs and explain how you validated it.  

---

## Best Practices

**✅ Recommended Uses**  

- Generating boilerplate code and common patterns  
- Creating comprehensive test suites  
- Writing documentation and comments  
- Refactoring existing code for clarity  
- Generating utility functions and helpers  
- Explaining existing code patterns  

**❌ Avoid AI For**  

- Complex business logic without thorough review  
- Security critical authentication/authorization code  
- Code you don’t fully understand  
- Large architectural changes  
- Database migrations or schema changes  

**Workflow Tips**  

- Start small and validate often—build, lint, and test incrementally  
- Study existing patterns before generating new code  
- Always ask: “Is this secure? Does it follow project patterns? What edge cases need testing?”  

**Security Considerations**  

- Extra review required for MCP servers, network code, file system ops, user input, and credential handling  
- Never expose secrets in prompts  
- Sanitize inputs/outputs and follow goose’s security patterns  

---

## Testing & Review

Before submitting AI assisted code, confirm that:  
- You understand every line  
- All tests pass locally (happy path + error cases)  
- Docs are updated and accurate  
- Code follows existing patterns  

**Always get human review** for: 

- Security sensitive code  
- Core architecture changes  
- Async/concurrency logic  
- MCP protocol implementations  
- Large refactors or anything you’re unsure about  

---

## Using goose for goose Development

- Protect sensitive files with `.gooseignore` (e.g., `.env*`, `*.key`, `target/`, `.git/`)  
- Guide Goose with `.goosehints` (patterns, error handling, formatting, tests, docs)  
- Use `/plan` to structure work, and choose modes wisely:  
  - **Chat** for understanding  
  - **Smart Approval** for most dev work  
  - **Approval** for critical areas  
  - **Autonomous** only with safety nets  

---

## Community & Collaboration

- In PRs, note significant AI use and how you validated results  
- Share prompting tips, patterns, and pitfalls  
- Be responsive to feedback and help improve this guide  

---

## Remember

AI is a powerful assistant, not a replacement for your judgment. Use it to speed up development; while keeping your brain engaged, your standards high, and goose secure.  

Questions? Join our [Discord](https://discord.gg/goose-oss) or [GitHub Discussions](https://github.com/block/goose/discussions) to talk more about responsible AI development.  
