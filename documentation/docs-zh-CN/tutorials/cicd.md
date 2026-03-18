---
title: CI/CD 环境
description: 把 goose 集成进 CI/CD 流水线，以自动化完成各类任务
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

goose 不只适合跑在你的本地机器上，它也可以帮助你简化 CI/CD 环境中的工作。把 goose 集成进流水线后，你可以自动化处理这些任务：

- 代码评审
- 文档检查
- 构建和部署流程
- 基础设施与环境管理
- 回滚与恢复流程
- 智能化测试执行

本指南会带你完成 goose 在 CI/CD 流水线中的基本配置，并重点演示如何在 GitHub Actions 中做 PR 代码评审。


## 在 GitHub Actions 中使用 goose
你可以直接在 GitHub Actions 中运行 goose。按照下面的步骤搭建你的 workflow。

:::info TL;DR
<details>
   <summary>复制这份 GitHub Workflow</summary>
   
   ```yaml title="goose.yml"


name: goose

on:
   pull_request:
      types: [opened, synchronize, reopened, labeled]

permissions:
   contents: write
   pull-requests: write
   issues: write

env:
   PROVIDER_API_KEY: ${{ secrets.REPLACE_WITH_PROVIDER_API_KEY }}
   PR_NUMBER: ${{ github.event.pull_request.number }}
   GH_TOKEN: ${{ github.token }}

jobs:
   goose-comment:
      name: goose Comment
      runs-on: ubuntu-latest
      steps:
         - name: Check out repository
           uses: actions/checkout@v4
           with:
              fetch-depth: 0

         - name: Gather PR information
           run: |
              {
              echo "# Files Changed"
              gh pr view $PR_NUMBER --json files \
                 -q '.files[] | "* " + .path + " (" + (.additions|tostring) + " additions, " + (.deletions|tostring) + " deletions)"'
              echo ""
              echo "# Changes Summary"
              gh pr diff $PR_NUMBER
              } > changes.txt

         - name: Install goose CLI
           run: |
              mkdir -p /home/runner/.local/bin
              curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh \
                | GOOSE_VERSION=REPLACE_WITH_VERSION CONFIGURE=false GOOSE_BIN_DIR=/home/runner/.local/bin bash
              echo "/home/runner/.local/bin" >> $GITHUB_PATH

         - name: Configure goose
           run: |
              mkdir -p ~/.config/goose
              cat <<EOF > ~/.config/goose/config.yaml
              GOOSE_PROVIDER: REPLACE_WITH_PROVIDER
              GOOSE_MODEL: REPLACE_WITH_MODEL
              keyring: false
              EOF

         - name: Create instructions for goose
           run: |
              cat <<EOF > instructions.txt
              Create a summary of the changes provided. Don't provide any session or logging details.
              The summary for each file should be brief and structured as:
              <filename/path (wrapped in backticks)>
                 - dot points of changes
              You don't need any extensions, don't mention extensions at all.
              The changes to summarise are:
              $(cat changes.txt)
              EOF

         - name: Test
           run: cat instructions.txt

         - name: Run goose and filter output
           run: |
              goose run --instructions instructions.txt | \
              # Remove ANSI color codes
              sed -E 's/\x1B\[[0-9;]*[mK]//g' | \
              # Remove session/logging lines
              grep -v "logging to /home/runner/.config/goose/sessions/" | \
              grep -v "^starting session" | \
              grep -v "^Closing session" | \
              # Trim trailing whitespace
              sed 's/[[:space:]]*$//' \
              > pr_comment.txt

         - name: Post comment to PR
           run: |
              cat -A pr_comment.txt
              gh pr comment $PR_NUMBER --body-file pr_comment.txt

   ```
</details>

:::

### 1. 创建 Workflow 文件

在仓库中创建一个新文件：`.github/workflows/goose.yml`。这个文件就是你的 GitHub Actions workflow。

### 2. 定义触发条件与权限

配置 action，使其：

- 在 PR 被打开、更新、重新打开或打标签时触发
- 授予 goose 与仓库交互所需的权限
- 为你选用的 LLM provider 配置环境变量

```yaml
name: goose

on:
    pull_request:
        types: [opened, synchronize, reopened, labeled]

permissions:
    contents: write
    pull-requests: write
    issues: write

env:
   PROVIDER_API_KEY: ${{ secrets.REPLACE_WITH_PROVIDER_API_KEY }}
   PR_NUMBER: ${{ github.event.pull_request.number }}
```


### 3. 安装并配置 goose

把下面步骤加入 workflow，以便在 runner 中安装和配置 goose：

```yaml
steps:
    - name: Install goose CLI
      run: |
          mkdir -p /home/runner/.local/bin
          curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh \
            | GOOSE_VERSION=REPLACE_WITH_VERSION CONFIGURE=false GOOSE_BIN_DIR=/home/runner/.local/bin bash
          echo "/home/runner/.local/bin" >> $GITHUB_PATH

    - name: Configure goose
      run: |
          mkdir -p ~/.config/goose
          cat <<EOF > ~/.config/goose/config.yaml
          GOOSE_PROVIDER: REPLACE_WITH_PROVIDER
          GOOSE_MODEL: REPLACE_WITH_MODEL
          keyring: false
          EOF
```

#### 在 CI/CD 中固定 goose 版本

在 CI/CD 场景下，建议使用 `GOOSE_VERSION` 固定一个具体版本，保证每次运行都可复现。同时，这也能避免当 `stable` 标签下没有对应二进制资产时，下载 goose CLI 失败并返回 404。

CI 中相关安装参数：
- `GOOSE_VERSION`：指定要安装的版本（支持 `1.21.1` 与 `v1.21.1` 两种格式）
- `GOOSE_BIN_DIR`：安装目录（记得把它加入 `PATH`）
- `CONFIGURE=false`：跳过交互式 `goose configure`

:::info 需要替换的占位符
把 `REPLACE_WITH_VERSION`、`REPLACE_WITH_PROVIDER` 和 `REPLACE_WITH_MODEL` 替换成你想固定的 goose 版本，以及实际使用的 provider / model。其他必要配置也可以按需补充。
:::

### 4. 收集 PR 变更并生成 instructions

这一步会提取 pull request 详情，并把它整理成结构化 instructions 交给 goose：

```yaml
    - name: Create instructions for goose
      run: |
          cat <<EOF > instructions.txt
          Create a summary of the changes provided. Don't provide any session or logging details.
          The summary for each file should be brief and structured as:
            <filename/path (wrapped in backticks)>
              - dot points of changes
          You don't need any extensions, don't mention extensions at all.
          The changes to summarise are:
          $(cat changes.txt)
          EOF
```

### 5. 运行 goose 并清洗输出

接下来运行 goose，并移除 ANSI 颜色码和无关日志：

```yaml
    - name: Run goose and filter output
      run: |
          goose run --instructions instructions.txt | \
            # Remove ANSI color codes
            sed -E 's/\x1B\[[0-9;]*[mK]//g' | \
            # Remove session/logging lines
            grep -v "logging to /home/runner/.config/goose/sessions/" | \
            grep -v "^starting session" | \
            grep -v "^Closing session" | \
            # Trim trailing whitespace
            sed 's/[[:space:]]*$//' \
            > pr_comment.txt
```

### 6. 把结果评论到 PR

最后，把 goose 的输出作为评论发回 pull request：

```yaml
    - name: Post comment to PR
      run: |
          cat -A pr_comment.txt
          gh pr comment $PR_NUMBER --body-file pr_comment.txt
```

通过这份 workflow，goose 会在 pull request 上自动运行、分析变更，并把总结作为评论发回 PR。

这只是一个示例。你完全可以根据自己的需求，把 GitHub Action 改造成更适合你的版本。

---

## 在 CI/CD 中并行运行多个 goose 实例

goose 支持多个并发 session，且各自状态隔离，因此可以安全地在 CI/CD 流水线中并行运行多个 goose 实例。每个实例都会维护自己的会话历史、agent 上下文和扩展配置，互不干扰。

这非常适合 matrix builds、多环境任务，或多个组件并行处理这类场景。

---

## 安全注意事项

在 CI/CD 环境里运行 goose 时，建议特别关注这些安全实践：

1. **Secret 管理**
      - 把 API keys 这类敏感凭据保存在 GitHub Secrets 中
      - 不要在日志或 PR 评论里暴露这些凭据

2. **最小权限原则**
      - 只授予 workflow 真正需要的权限，并定期审查

3. **输入校验**
      - 传给 goose 的任何输入都应该经过清理和校验，避免引入不可预期行为
