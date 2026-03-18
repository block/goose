---
title: "在 Headless 模式中使用 goose"
description: "介绍 goose 的 Headless Mode 以及自动化场景下的使用方式。"
---

# 在 Headless 模式中使用 goose

*在 CI/CD、服务器和批处理任务中运行 AI 驱动的工程自动化流程。*

无需人工干预就能自动完成复杂工程任务，本身已经很有价值；而借助 AI，这件事可以再往前走一步。goose 的 Headless 模式让开发者可以在服务器环境、CI/CD 流水线和批处理场景里，直接使用 goose 的自动化能力，而这些场景通常并不适合交互式会话。

## 什么是 Headless 模式？

Headless 模式是 goose 的非交互执行环境，面向那些没有人工输入也不需要人工输入的自动化场景。

它和 Desktop 或 CLI 的交互式会话不同：Headless 模式会接收指令、自动执行，然后自行退出，因此非常适合嵌入已有开发工作流。

你可以把它理解成：交互式模式像是在和 AI 助手对话；而 Headless 模式更像是给它一份清晰任务书，然后让它独立去完成。

## 交互式模式 vs Headless 模式

| 功能 | 交互式模式 | Headless 模式 |
|---|---|---|
| **用户输入** | 会在需要时询问你选择或澄清 | 使用默认行为和预配置设置 |
| **上下文管理** | 超限时会询问你使用哪种策略 | 自动摘要会话 |
| **会话持久化** | 保留持续对话状态 | 执行任务后干净退出 |
| **错误处理** | 用户可以介入并提供指导 | 按配置执行自动错误响应 |
| **工具权限** | 风险操作可以询问批准 | 使用预配置默认值或安全失败 |
| **执行方式** | 来回对话式 | 单次执行并输出完整结果 |

## 真实使用场景与命令示例

### 1. 服务器环境与云部署

非常适合无图形界面的服务器、容器环境和云部署场景。

```bash
# 自动化服务器维护
goose run --with-builtin developer -t "Check system logs for errors in the last 24 hours, identify performance bottlenecks, and generate a maintenance report"

# 容器优化
goose run --no-session -t "Analyze the Dockerfile, optimize for smaller image size, and update the build process documentation"

# 云资源审计
goose run -t "Review our AWS infrastructure configuration, identify cost optimization opportunities, and create a migration plan for underutilized resources"
```

### 2. CI/CD 集成

把 AI 驱动的分析与修复无缝接入你的持续集成流程。

```bash
# 写在 .github/workflows/ci.yml 中
- name: AI-Powered Code Review
  run: |
    goose run --with-builtin developer \
      -t "Analyze the code changes in this PR, check for security vulnerabilities, performance issues, and suggest improvements. Generate a detailed review report."

# 测试失败分析
goose run --debug -t "Examine the failing test suite, identify the root cause of failures, implement fixes, and ensure all tests pass"

# 自动更新文档
goose run -t "Review code changes and update the README.md and API documentation to reflect new features and modifications"
```

### 3. 批处理与大规模操作

适合跨多个文件、仓库或系统执行批量任务。

```bash
# 批量代码现代化
goose run --with-builtin developer \
  -t "Upgrade all Python files in the src/ directory from Python 3.8 to 3.11 syntax, update dependencies, and ensure compatibility"

# 多仓库维护
for repo in repo1 repo2 repo3; do
  cd $repo
  goose run --no-session -t "Update all dependencies to latest stable versions, run tests, and create a PR if changes are needed"
  cd ..
done

# 数据库迁移自动化
goose run -t "Analyze the current database schema, generate migration scripts for the new requirements, and create rollback procedures"
```

### 4. 定时任务

你可以把它和 cron 或任务调度器结合起来，定期执行维护工作。

```bash
# 每日安全扫描（加入 crontab）
0 2 * * * /usr/local/bin/goose run --no-session -t "Run comprehensive security audit, check for vulnerabilities, and email report to security team"

# 每周依赖更新
0 9 * * 1 /usr/local/bin/goose run -t "Check for outdated dependencies, create update PRs for non-breaking changes, and schedule review for major updates"
```

## Headless 模式的最佳实践

### 1. 写出足够清晰的指令

prompt 是自动化成功与否的蓝图。你的指令需要具体、详细、没有歧义。如果一时拿不准，可以先参考一些 [vibe prompting 的示例](https://www.youtube.com/watch?v=IjXmT0W4f2Q) 来打开思路。

```bash
# 一般：过于模糊
goose run -t "Fix the issues"

# 更好：明确且可执行
goose run -t "Analyze the test failures in the latest CI run, identify the root cause, and create a fix with appropriate unit tests"

# 更进一步：给出更具体的范围和约束
goose run -t "Review the failed tests in tests/integration/, identify why the authentication middleware is failing, implement a fix that maintains backward compatibility, and add regression tests"
```

### 2. 预先配置环境

提前设置好环境变量，可以减少运行时反复做决定的成本。

```bash
export GOOSE_CONTEXT_STRATEGY=summarize
export GOOSE_MAX_TURNS=50
export GOOSE_MODE=auto
export GOOSE_DISABLE_SESSION_NAMING=true
```

其中 `GOOSE_CONTEXT_STRATEGY` 和 `GOOSE_MAX_TURNS` 用来控制上下文管理；`GOOSE_MODE=auto` 允许非交互执行；`GOOSE_DISABLE_SESSION_NAMING=true` 则可以省掉为了生成会话名而触发的一次额外后台模型调用，让默认名称保持为 `CLI Session`。

### 3. 做好错误处理

在自动化脚本里一定要检查退出码，并为失败场景准备清晰处理：

```bash
#!/bin/bash
if ! goose run --no-session -t "Run security audit and fix critical issues"; then
    echo "goose automation failed - manual intervention required"
    exit 1
fi
```

### 4. 选择合适的 Session 策略

一次性任务建议使用 `--no-session`，避免把会话历史堆得太杂；但如果任务复杂、步骤很多，或者你第一次尝试这类自动化，保留 session 会更方便事后调试。

## 在 Headless 模式中执行 Recipes

[Recipes](/zh-CN/docs/guides/recipes/) 是 goose 用来定义可复用、参数化工作流的核心能力。在 Headless 模式里，recipes 会更有价值，因为它非常适合承载自动化流程。

### Headless 模式下的 Recipe 要求

如果一个 recipe 要在 Headless 模式中运行，它**必须**包含 `prompt` 字段。这个 `prompt` 会作为自动执行的起始指令：

```yaml
# automation-recipe.yaml
title: "Automated Code Quality Check"
name: "Automated Code Quality Check"
description: "Comprehensive code quality analysis and improvement"
author:
  name: "DevOps Team"
  email: "devops@company.com"

# Headless 模式必须提供
prompt: "Perform a comprehensive code quality analysis including linting, security scanning, test coverage analysis, and generate an improvement plan"

instructions: |
  You are an expert code quality engineer. Your task is to:
  1. Run static analysis tools (eslint, pylint, etc.)
  2. Perform security vulnerability scanning
  3. Analyze test coverage and identify gaps
  4. Check for code duplication and complexity issues
  5. Generate a prioritized improvement plan
  6. Create actionable tickets for the development team

parameters:
  - key: target_directory
    input_type: string
    requirement: required
    description: "Directory to analyze"
    default: "./src"
  - key: output_format
    input_type: string
    requirement: required
    description: "Report format (markdown, json, html)"
    default: "markdown"

extensions:
  - type: builtin
    name: developer
    display_name: Developer
    timeout: 300
    bundled: true
```

### 在 Headless 模式中执行 Recipes

```bash
# 基本执行
goose run --recipe automation-recipe.yaml

# 传入自定义参数
goose run --recipe automation-recipe.yaml \
  --params target_directory=./backend \
  --params output_format=json

# 多 recipe 组合工作流
goose run --recipe main-workflow.yaml \
  --sub-recipe security-audit.yaml \
  --sub-recipe performance-analysis.yaml \
  --params environment=production
```

## 理解它的限制

Headless 模式很强，但你也需要理解它的边界，才能给自动化设计合理预期。

### 1. 无法与用户交互

**这意味着什么**：执行期间 goose 不能中途请求澄清、审批或额外输入。如果它不确定该怎么做，通常会在输出结果里留下类似 “How should I proceed?” 的问题。

**影响**：一旦指令模糊，或者执行中出现意外情况，goose 只能基于现有上下文自行判断，而结果不一定完全符合你的原始意图。

**缓解方式**：把指令写得足够详细，尤其是要说明“遇到问题时应该怎么处理”，并且先在**非生产环境里充分测试**你的自动化流程。

```bash
# 问题示例：太模糊
goose run -t "Fix the issues"

# 更好的写法：具体且可执行
goose run -t "Fix the TypeScript compilation errors in src/components/, ensure all imports are correct, and update any deprecated API calls to use the latest syntax"
```

### 2. Recipe 的 prompt 是硬要求

**这意味着什么**：任何在 Headless 模式下运行的 recipe 都必须包含 `prompt` 字段，否则会直接报错。

**影响**：原本只适合交互式使用的 recipe，可能需要先改造后才能用于自动化。

**缓解方式**：即使一个 recipe 主要是为交互场景设计，也建议始终提供一个清晰的 `prompt`。

### 3. 工具权限依赖预配置

**这意味着什么**：goose 无法在执行时询问是否允许使用高风险工具或执行某类操作。

**影响**：需要审批的操作可能会直接使用默认权限，也可能因为权限不足而失败，从而阻塞自动化流程。

**缓解方式**：通过环境变量预先配置工具权限：

```bash
export GOOSE_MODE=auto
# 或在配置文件中显式配置工具权限
```

### 4. 上下文决策会自动化

**这意味着什么**：当会话上下文达到上限时，goose 会直接使用预设策略处理，而不是征求用户意见。

**影响**：如果摘要不够准确，可能丢失重要上下文；如果清理策略过于激进，也可能打断执行。

**缓解方式**：预先设定合理的上下文策略，并控制 token 消耗：

```bash
export GOOSE_CONTEXT_STRATEGY=summarize
export GOOSE_MAX_TURNS=100
```

### 5. 复杂错误恢复能力有限

**这意味着什么**：某些本来适合通过人类判断来快速修复的问题，在 Headless 模式里无法通过对话来补救。

**影响**：自动化流程在边缘场景下可能失败，而这些失败如果有人在场，本来很容易通过几句澄清解决。

**缓解方式**：在调用脚本里提供清晰的降级和回退逻辑：

```bash
#!/bin/bash
if ! goose run --recipe complex-deployment.yaml; then
    echo "Complex deployment failed, initiating rollback procedure"
    goose run --recipe rollback.yaml
fi
```

## 配置与环境准备

### 常用环境变量

```bash
# 上下文管理
export GOOSE_CONTEXT_STRATEGY=summarize
export GOOSE_MAX_TURNS=50

# 工具行为
export GOOSE_MODE=auto

# 模型配置
export GOOSE_PROVIDER=openai
export GOOSE_MODEL=gpt-4o

# 输出控制
export GOOSE_CLI_MIN_PRIORITY=0.2
```

### 高级配置

```bash
# 为复杂工作流分配不同模型
export GOOSE_LEAD_MODEL=gpt-4o
export GOOSE_WORKER_MODEL=gpt-4o-mini

# 安全与权限
export GOOSE_ALLOWLIST=https://company.com/allowed-extensions.json
```

## 自动化开发的下一步

goose 的 Headless 模式不只是一个功能点，它更像是 AI 驱动自动化开发工作流的一块基础设施。你可以把日常、重复、耗时的工程任务交给 AI，把人的精力腾出来做更高价值的判断和设计工作。

无论你的目标是把 AI 接进 CI/CD、自动化服务器维护，还是跨多个仓库做批量操作，Headless 模式都为构建更可靠、更复杂的自动化流程提供了基础。
