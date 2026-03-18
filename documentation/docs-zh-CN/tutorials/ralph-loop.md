---
title: Ralph Loop
description: 使用每轮全新上下文和跨模型评审，让 goose 迭代推进任务直到真正完成
---

Ralph Loop 基于 [Geoffrey Huntley 的 Ralph Wiggum technique](https://ghuntley.com/ralph/)，是一种迭代式开发模式：让 goose 不断循环处理同一个任务，直到任务真的达到可交付状态。

标准 agent loop 的问题在于上下文不断累积。每一次失败尝试都会留在对话历史里，几轮之后，模型就不得不先处理一长串噪音，再回到真正的任务本身。Ralph Loop 的核心做法是：**每一轮都从全新上下文开始**。这是 Geoffrey 方案最重要的洞察。

这个实现又进一步加入了**跨模型评审**：一个模型负责完成工作，另一个模型负责审查结果。只有在评审通过后才结束循环，否则继续下一轮。

每轮结束后，worker 模型和 reviewer 模型都会把摘要与反馈写入文件。这些文件会在轮次之间保留下来，但聊天历史不会。因此下一轮虽然是全新会话，却仍能通过这些状态文件准确接续前一轮工作。

在这个教程中，你会用 Ralph Loop 来构建一个简单的 Electron 浏览器，并观察迭代流程如何在发版前捕获遗漏功能。

### 前置条件

- 先[安装 goose CLI](/zh-CN/docs/getting-started/installation)，因为 Ralph Loop 通过终端运行
- 先[配置两个模型](/zh-CN/docs/getting-started/providers)，分别作为 worker 和 reviewer。虽然 worker / reviewer 使用同一个模型也能工作，但使用不同模型通常能带来更高质量的评审结果

<details>
<summary>下载 Ralph Loop Recipes</summary>

在终端中复制并执行以下命令，下载 Ralph Loop 所需 recipes：

```bash
mkdir -p ~/.config/goose/recipes

curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/ralph-loop.sh -o ~/.config/goose/recipes/ralph-loop.sh
curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/ralph-work.yaml -o ~/.config/goose/recipes/ralph-work.yaml
curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/ralph-review.yaml -o ~/.config/goose/recipes/ralph-review.yaml

chmod +x ~/.config/goose/recipes/ralph-loop.sh
```

</details>

:::warning 成本提示
Ralph Loop 会在循环中多次运行 agent（默认最多 10 轮）。请注意使用量，并在必要时调小 `RALPH_MAX_ITERATIONS`。
:::

### 第 1 步：启动 Loop

在终端里运行脚本，并把你的任务描述放在引号中。这个命令会启动 worker / reviewer 的第一次迭代：

```bash
~/.config/goose/recipes/ralph-loop.sh "Create a simple browser using Electron and React"
```

:::tip 复杂任务的用法
你也可以把任务写在文件里，再把文件路径传给脚本。这对于 PRD、详细规格说明或任何适合迭代推进的多步骤任务都很有帮助：

```bash
~/.config/goose/recipes/ralph-loop.sh ./prd.md
```
:::

### 第 2 步：配置模型

脚本会在当前会话中提示你设置环境变量：

```
Worker model [gpt-4o]: 
Worker provider [openai]: 
Reviewer model (should be different from worker): claude-sonnet-4-20250514
Reviewer provider: anthropic
Max iterations [10]: 

⚠️  Cost Warning: This will run up to 10 iterations, each using both models.
    Estimated token usage could be significant depending on your task.

Continue? [y/N]: y
```

| 变量 | 说明 |
|--------|------|
| Worker model | 真正执行编码工作的模型。如果已设置 `GOOSE_MODEL`，默认沿用它 |
| Worker provider | worker 模型对应的 provider，例如 `openai`、`anthropic`。如果已设置 `GOOSE_PROVIDER`，默认沿用它 |
| Reviewer model | 负责评审结果的模型。建议与 worker 不同，以获得更独立的审查视角 |
| Reviewer provider | reviewer 模型对应的 provider |
| Max iterations | 最大工作 / 评审循环次数，默认 10 |

:::tip 直接设置环境变量
如果你不想走交互式输入，也可以直接设置环境变量，跳过这些提示：

```bash
RALPH_WORKER_MODEL="gpt-4o" \
RALPH_WORKER_PROVIDER="openai" \
RALPH_REVIEWER_MODEL="claude-sonnet-4-20250514" \
RALPH_REVIEWER_PROVIDER="anthropic" \
~/.config/goose/recipes/ralph-loop.sh "Create a simple browser using Electron and React"
```
:::

### 第 3 步：观察它运行

终端会展示 goose 在 worker 阶段和 reviewer 阶段之间来回切换。每一轮都以全新会话开始，以保持上下文干净。下面是一次成功运行的样子：

```
═══════════════════════════════════════════════════════════════
  Ralph Loop - Multi-Model Edition
═══════════════════════════════════════════════════════════════

  Task: Create a simple browser using Electron and React
  Worker: gpt-4o (openai)
  Reviewer: claude-sonnet-4-20250514 (anthropic)
  Max Iterations: 10

───────────────────────────────────────────────────────────────
  Iteration 1 / 10
───────────────────────────────────────────────────────────────

▶ WORK PHASE
... (goose creates initial implementation) ...

▶ REVIEW PHASE
... (goose reviews the work) ...

↻ REVISE - Feedback for next iteration:
Missing error handling for invalid URLs. Also needs back/forward navigation buttons.

───────────────────────────────────────────────────────────────
  Iteration 2 / 10
───────────────────────────────────────────────────────────────

▶ WORK PHASE
... (goose addresses feedback) ...

▶ REVIEW PHASE
... (goose reviews again) ...

═══════════════════════════════════════════════════════════════
  ✓ SHIPPED after 2 iteration(s)
═══════════════════════════════════════════════════════════════
```

## 它是如何工作的

```
Iteration 1:
  WORK PHASE  → Model A does work, writes to files
  REVIEW PHASE → Model B reviews the work
    → SHIP? Exit successfully ✓
    → REVISE? Write feedback, continue to iteration 2

Iteration 2:
  WORK PHASE  → Model A reads feedback, fixes things (fresh context!)
  REVIEW PHASE → Model B reviews again
    → SHIP? Exit successfully ✓
    → REVISE? Continue...

... repeats until SHIP or max iterations
```

### 状态文件

Ralph Loop 通过 `.goose/ralph/` 下的状态文件在轮次之间持久化信息。即使每一轮都是全新上下文，worker 仍然能通过这些文件知道该做什么，reviewer 也能知道前一轮完成了什么。

| 文件 | 用途 |
|------|------|
| `task.md` | 原始任务描述 |
| `iteration.txt` | 当前迭代轮次 |
| `work-summary.txt` | 本轮 worker 完成了什么 |
| `work-complete.txt` | 如果 worker 认为任务完成，就会创建这个文件 |
| `review-result.txt` | `SHIP` 或 `REVISE` |
| `review-feedback.txt` | 留给下一轮的反馈 |
| `.ralph-complete` | 成功完成后生成 |
| `RALPH-BLOCKED.md` | 如果 worker 卡住则生成 |

### Recipe 文件

Ralph Loop 由三个文件组成：
- 一个 bash 脚本，负责串起 work / review 循环
- 一个 work recipe，负责指导 worker 模型推进工作
- 一个 review recipe，负责指导 reviewer 模型评估结果

下面是这三个文件的内容。你可以[下载](#前置条件)它们，也可以直接复制。

<details>
<summary>Bash Wrapper（`ralph-loop.sh`）</summary>

```bash
#!/bin/bash
#
# Ralph Loop - Multi-Model Edition
#
# Fresh context per iteration + cross-model review
# Based on Geoffrey Huntley's technique
#
# Usage: ./ralph-loop.sh "your task description here"
#    or: ./ralph-loop.sh /path/to/task.md
#
# Environment variables:
#   RALPH_WORKER_MODEL    - Model for work phase (prompts if not set)
#   RALPH_WORKER_PROVIDER - Provider for work phase (prompts if not set)
#   RALPH_REVIEWER_MODEL  - Model for review phase (prompts if not set)
#   RALPH_REVIEWER_PROVIDER - Provider for review phase (prompts if not set)
#   RALPH_MAX_ITERATIONS  - Max iterations (default: 10)
#   RALPH_RECIPE_DIR      - Recipe directory (default: ~/.config/goose/recipes)
#

set -e

INPUT="$1"
RECIPE_DIR="${RALPH_RECIPE_DIR:-$HOME/.config/goose/recipes}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

if [ -z "$INPUT" ]; then
    echo -e "${RED}Error: No task provided${NC}"
    echo "Usage: $0 \"your task description\""
    echo "   or: $0 /path/to/task.md"
    exit 1
fi

# Function to prompt for settings
prompt_for_settings() {
    local default_model="${GOOSE_MODEL:-}"
    local default_provider="${GOOSE_PROVIDER:-}"
    
    # Worker model
    if [ -n "$default_model" ]; then
        echo -ne "${BLUE}Worker model${NC} [${default_model}]: "
        read -r user_input
        WORKER_MODEL="${user_input:-$default_model}"
    else
        echo -ne "${BLUE}Worker model${NC}: "
        read -r WORKER_MODEL
        if [ -z "$WORKER_MODEL" ]; then
            echo -e "${RED}Error: Worker model is required${NC}"
            exit 1
        fi
    fi
    
    # Worker provider
    if [ -n "$default_provider" ]; then
        echo -ne "${BLUE}Worker provider${NC} [${default_provider}]: "
        read -r user_input
        WORKER_PROVIDER="${user_input:-$default_provider}"
    else
        echo -ne "${BLUE}Worker provider${NC}: "
        read -r WORKER_PROVIDER
        if [ -z "$WORKER_PROVIDER" ]; then
            echo -e "${RED}Error: Worker provider is required${NC}"
            exit 1
        fi
    fi
    
    # Reviewer model
    echo -ne "${BLUE}Reviewer model${NC} (should be different from worker): "
    read -r REVIEWER_MODEL
    if [ -z "$REVIEWER_MODEL" ]; then
        echo -e "${RED}Error: Reviewer model is required${NC}"
        echo "The reviewer should be a different model to provide fresh perspective."
        exit 1
    fi
    
    # Reviewer provider
    echo -ne "${BLUE}Reviewer provider${NC}: "
    read -r REVIEWER_PROVIDER
    if [ -z "$REVIEWER_PROVIDER" ]; then
        echo -e "${RED}Error: Reviewer provider is required${NC}"
        exit 1
    fi
    
    # Same model warning
    if [ "$WORKER_MODEL" = "$REVIEWER_MODEL" ] && [ "$WORKER_PROVIDER" = "$REVIEWER_PROVIDER" ]; then
        echo -e "${YELLOW}Warning: Worker and reviewer are the same model.${NC}"
        echo "For best results, use different models for cross-model review."
        echo -ne "Continue anyway? [y/N]: "
        read -r confirm
        if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
            exit 1
        fi
    fi
    
    # Max iterations
    echo -ne "${BLUE}Max iterations${NC} [10]: "
    read -r user_input
    MAX_ITERATIONS="${user_input:-10}"
}

# Initialize from environment variables
WORKER_MODEL="${RALPH_WORKER_MODEL:-}"
WORKER_PROVIDER="${RALPH_WORKER_PROVIDER:-}"
REVIEWER_MODEL="${RALPH_REVIEWER_MODEL:-}"
REVIEWER_PROVIDER="${RALPH_REVIEWER_PROVIDER:-}"
MAX_ITERATIONS="${RALPH_MAX_ITERATIONS:-10}"

# If any required setting is missing, prompt for all settings
if [ -z "$WORKER_MODEL" ] || [ -z "$WORKER_PROVIDER" ] || [ -z "$REVIEWER_MODEL" ] || [ -z "$REVIEWER_PROVIDER" ]; then
    prompt_for_settings
fi

# Cost warning and confirmation loop
while true; do
    echo ""
    echo -e "${YELLOW}⚠️  Cost Warning:${NC} This will run up to ${MAX_ITERATIONS} iterations, each using both models."
    echo "    Estimated token usage could be significant depending on your task."
    echo ""
    echo -ne "Continue? [y/N]: "
    read -r confirm
    
    if [ "$confirm" = "y" ] || [ "$confirm" = "Y" ]; then
        break
    else
        echo ""
        prompt_for_settings
    fi
done

STATE_DIR=".goose/ralph"
mkdir -p "$STATE_DIR"

if [ -f "$INPUT" ]; then
    cp "$INPUT" "$STATE_DIR/task.md"
    echo -e "${BLUE}Reading task from file: $INPUT${NC}"
else
    echo "$INPUT" > "$STATE_DIR/task.md"
fi

TASK=$(cat "$STATE_DIR/task.md")

rm -f "$STATE_DIR/review-result.txt"
rm -f "$STATE_DIR/review-feedback.txt"
rm -f "$STATE_DIR/work-complete.txt"
rm -f "$STATE_DIR/work-summary.txt"

echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Ralph Loop - Multi-Model Edition${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  Task: ${YELLOW}$TASK${NC}"
echo -e "  Worker: ${WORKER_MODEL} (${WORKER_PROVIDER})"
echo -e "  Reviewer: ${REVIEWER_MODEL} (${REVIEWER_PROVIDER})"
echo -e "  Max Iterations: $MAX_ITERATIONS"
echo ""

for i in $(seq 1 "$MAX_ITERATIONS"); do
    echo -e "${BLUE}───────────────────────────────────────────────────────────────${NC}"
    echo -e "${BLUE}  Iteration $i / $MAX_ITERATIONS${NC}"
    echo -e "${BLUE}───────────────────────────────────────────────────────────────${NC}"
    
    echo "$i" > "$STATE_DIR/iteration.txt"
    
    echo ""
    echo -e "${YELLOW}▶ WORK PHASE${NC}"
    
    GOOSE_PROVIDER="$WORKER_PROVIDER" GOOSE_MODEL="$WORKER_MODEL" goose run --recipe "$RECIPE_DIR/ralph-work.yaml" || {
        echo -e "${RED}✗ WORK PHASE FAILED${NC}"
        exit 1
    }
    
    if [ -f "$STATE_DIR/RALPH-BLOCKED.md" ]; then
        echo ""
        echo -e "${RED}✗ BLOCKED${NC}"
        cat "$STATE_DIR/RALPH-BLOCKED.md"
        exit 1
    fi
    
    echo ""
    echo -e "${YELLOW}▶ REVIEW PHASE${NC}"
    
    GOOSE_PROVIDER="$REVIEWER_PROVIDER" GOOSE_MODEL="$REVIEWER_MODEL" goose run --recipe "$RECIPE_DIR/ralph-review.yaml" || {
        echo -e "${RED}✗ REVIEW PHASE FAILED${NC}"
        exit 1
    }
    
    if [ -f "$STATE_DIR/review-result.txt" ]; then
        RESULT=$(cat "$STATE_DIR/review-result.txt" | tr -d '[:space:]')
        
        if [ "$RESULT" = "SHIP" ]; then
            echo ""
            echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
            echo -e "${GREEN}  ✓ SHIPPED after $i iteration(s)${NC}"
            echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
            echo "COMPLETE: $(date)" > "$STATE_DIR/.ralph-complete"
            exit 0
        else
            echo ""
            echo -e "${YELLOW}↻ REVISE - Feedback for next iteration:${NC}"
            if [ -f "$STATE_DIR/review-feedback.txt" ]; then
                cat "$STATE_DIR/review-feedback.txt"
            fi
        fi
    else
        echo -e "${RED}✗ No review result found${NC}"
        exit 1
    fi
    
    rm -f "$STATE_DIR/work-complete.txt"
    rm -f "$STATE_DIR/review-result.txt"
    echo ""
done

echo -e "${RED}✗ Max iterations ($MAX_ITERATIONS) reached${NC}"
exit 1
```

</details>

<details>
<summary>Work Phase Recipe（`ralph-work.yaml`）</summary>

```yaml
version: 1.0.0
title: Ralph Work Phase
description: Single iteration of work - fresh context each time

instructions: |
  You are in a RALPH LOOP - one iteration of work.
  
  Your work persists through FILES ONLY. You will NOT remember previous iterations.
  
  STATE FILES (in .goose/ralph/):
  - task.md = The task you need to accomplish (READ THIS FIRST)
  - iteration.txt = Current iteration number
  - review-feedback.txt = Feedback from last review (if any)
  - work-complete.txt = Create when task is DONE (reviewer will verify)
  
  FIRST: Check your state
  1. cat .goose/ralph/task.md (YOUR TASK)
  2. cat .goose/ralph/iteration.txt 2>/dev/null || echo "1"
  3. cat .goose/ralph/review-feedback.txt 2>/dev/null
  4. ls -la to see existing work
  
  THEN: Make progress
  - If review-feedback.txt exists, ADDRESS THAT FEEDBACK FIRST
  - Read existing code/files before modifying
  - Make meaningful incremental progress
  - Run tests/verification if applicable
  
  FINALLY: Signal status
  - If task is complete: echo "done" > .goose/ralph/work-complete.txt
  - Always write a summary: echo "what I did" > .goose/ralph/work-summary.txt

prompt: |
  ## Ralph Work Phase
  
  Read your task from: .goose/ralph/task.md
  
  1. Read the task: `cat .goose/ralph/task.md`
  2. Check iteration: `cat .goose/ralph/iteration.txt 2>/dev/null || echo "1"`
  3. Check for review feedback: `cat .goose/ralph/review-feedback.txt 2>/dev/null`
  4. List existing files: `ls -la`
  5. Do the work (address feedback if any, otherwise make progress)
  6. Write summary: `echo "summary" > .goose/ralph/work-summary.txt`
  7. If complete: `echo "done" > .goose/ralph/work-complete.txt`

extensions:
  - type: builtin
    name: developer
    timeout: 600
```

</details>

<details>
<summary>Review Phase Recipe（`ralph-review.yaml`）</summary>

```yaml
version: 1.0.0
title: Ralph Review Phase
description: Cross-model review of work - returns SHIP or REVISE

instructions: |
  You are a CODE REVIEWER in a Ralph Loop.
  
  Your job: Review the work done and decide SHIP or REVISE.
  
  You are a DIFFERENT MODEL than the worker. Your fresh perspective catches mistakes.
  
  STATE FILES (in .goose/ralph/):
  - task.md = The original task (READ THIS FIRST)
  - work-summary.txt = What the worker claims to have done
  - work-complete.txt = Exists if worker claims task is complete
  
  REVIEW CRITERIA:
  1. Does the code/work actually accomplish the task?
  2. Does it run without errors?
  3. Is it reasonably complete, not half-done?
  4. Are there obvious bugs or issues?
  
  BE STRICT but FAIR:
  - Don't nitpick style if functionality is correct
  - DO reject incomplete work
  - DO reject code that doesn't run
  - DO reject if tests fail
  
  OUTPUT:
  If approved: echo "SHIP" > .goose/ralph/review-result.txt
  If needs work: 
    echo "REVISE" > .goose/ralph/review-result.txt
    echo "specific feedback" > .goose/ralph/review-feedback.txt

prompt: |
  ## Ralph Review Phase
  
  1. Read the task: `cat .goose/ralph/task.md`
  2. Read work summary: `cat .goose/ralph/work-summary.txt`
  3. Check if complete: `cat .goose/ralph/work-complete.txt 2>/dev/null`
  4. Examine the actual files created/modified
  5. Run verification (tests, build, etc.)
  6. Decide: SHIP or REVISE
  
  If SHIP: `echo "SHIP" > .goose/ralph/review-result.txt`
  If REVISE: 
    `echo "REVISE" > .goose/ralph/review-result.txt`
    `echo "specific feedback" > .goose/ralph/review-feedback.txt`

extensions:
  - type: builtin
    name: developer
    timeout: 300
```

</details>

## 使用建议

### 什么时候适合 Ralph Loop

Ralph Loop 最适合这些场景：

- **复杂、分多步的任务**：适合通过多轮推进逐步逼近完成态
- **完成标准清晰的任务**：例如测试通过、构建成功、功能全部就绪
- **希望发版前加一道质量门禁的场景**

以下情况则通常不值得动用 Ralph Loop：

- 简单的一次性任务
- 强交互、强探索性质的工作
- 没有明确“完成标准”的任务

### 重置

如果你要开始一个全新任务，或者之前的一轮执行卡住了，需要从头开始，可以清空状态目录：

```bash
rm -rf .goose/ralph
```
