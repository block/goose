---
title: "智能上下文管理"
description: "介绍 goose 如何管理上下文长度、记忆和长对话。"
sidebar_position: 3
sidebar_label: "智能上下文管理"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { ScrollText } from 'lucide-react';
import { PanelLeft } from 'lucide-react';

在使用 [Large Language Models (LLMs)](../../getting-started/providers.md) 时，模型一次能处理的对话历史是有限的。goose 提供了一套智能上下文管理能力，帮助你在接近上下文上限时继续维持高效会话。先明确几个基本概念：

- **Context Length**：模型当前可以纳入考虑的对话历史长度，也常被称为 context window
- **Context Limit**：模型最多能处理的 token 数
- **Context Management**：goose 处理“接近或超过上下文上限”问题的方式
- **Turn**：一次完整的 prompt / response 交互

## goose 如何管理上下文
goose 采用两层策略来管理上下文：

1. **Auto-Compaction**：在接近 token 上限时，主动总结并压缩对话
2. **Context Strategies**：如果自动压缩后仍超限，或自动压缩被禁用，再使用备用策略处理

这套分层机制可以让 goose 更平滑地应对 token 和上下文限制。

## 自动压缩 {#automatic-compaction}
当会话接近 token 上限时，goose 会自动压缩（总结）较早的对话内容，让你能在不手工干预的情况下维持长会话。

默认情况下，在 goose Desktop 和 goose CLI 中，当 token 使用量达到上限的 80% 时，就会触发自动压缩。

你可以通过 `GOOSE_AUTO_COMPACT_THRESHOLD` [环境变量](/zh-CN/docs/guides/environment-variables#session-management) 来控制自动压缩阈值。把它设置成 `0.0` 即可关闭该功能。

```
# 当已用 token 达到 60% 时自动压缩
export GOOSE_AUTO_COMPACT_THRESHOLD=0.6
```

当达到自动压缩阈值时：
  1. goose 会自动开始压缩对话，为后续内容腾出空间。
  2. 完成后，你会看到一条确认消息，说明会话已经被总结并压缩。
  3. 你可以继续当前会话。旧对话仍然可见，但 goose 的活跃上下文中只会携带压缩后的版本。

:::tip 自定义压缩方式
如果你想调整 goose 在压缩时如何总结会话，可以编辑 `compaction.md` [prompt template](../prompt-templates.md)。
:::

:::tip 工具输出总结
为了更高效地使用上下文，goose 会在后台对较早的工具调用输出做总结，同时保留最近的调用详情。默认情况下，当一个会话里有超过 10 次工具调用时就会触发。若要做更细粒度调整，请参考 [`GOOSE_TOOL_CALL_CUTOFF`](/zh-CN/docs/guides/environment-variables#session-management)。
:::

### 手动压缩
你也可以在达到上限前手动触发压缩：

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

  1. 把鼠标移到应用底部模型名称旁边的 token 使用指示点上
  2. 在弹出的上下文窗口中点击 <ScrollText className="inline" size={16} /> `Compact now`
  3. 完成后，你会看到“会话已压缩并总结”的确认消息
  4. 然后继续当前会话。旧对话仍然可见，但 goose 的活跃上下文里只会保留压缩后的版本

  :::info 
  只有在当前聊天中至少发送过一条消息之后，`Compact now` 按钮才会可用。
  :::

</TabItem>
<TabItem value="cli" label="goose CLI" default>

如果你想在达到上下文上限前主动总结会话，可以使用 `/summarize`：

```sh
( O)> /summarize
◇  Are you sure you want to summarize this conversation? This will condense the message history.
│  Yes 
│
Summarizing conversation...
Conversation has been summarized.
Key information has been preserved while reducing context length.
```

</TabItem>
</Tabs>

## 上下文超限后的处理策略

如果自动压缩被关闭，或者压缩之后会话仍然超过上下文上限，goose 会提供不同的处理方式：

| 功能 | 说明 | 适用场景 | 可用范围 | 影响 |
|---------|-------------|-----------|-----------|---------|
| **Summarization** | 总结会话并保留关键点 | 长而复杂的对话 | Desktop 和 CLI | 尽量保留上下文 |
| **Truncation** | 删除最早的消息，为新内容腾空间 | 线性且简单的对话 | 仅 CLI | 会丢失较早上下文 |
| **Clear** | 清空上下文但保留会话本身 | 需要彻底转向新任务 | 仅 CLI | 丢失全部旧上下文 |
| **Prompt** | 让用户自己选择上述方案 | 需要人工控制每次决策 | 仅 CLI | 取决于你的选择 |

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

goose Desktop 只使用 summarization，也就是通过压缩会话来管理上下文：尽量保留关键内容，同时减少上下文体积。

  </TabItem>
  <TabItem value="cli" label="goose CLI">

CLI 支持全部上下文处理策略：`summarize`、`truncate`、`clear` 和 `prompt`。

默认行为取决于运行模式：
- **交互模式**：提示用户选择（等价于 `prompt`）
- **Headless 模式**（`goose run`）：自动总结（等价于 `summarize`）

你可以通过设置 `GOOSE_CONTEXT_STRATEGY` 环境变量来指定 goose 应该如何处理上下文上限：

```bash
# 设置自动策略（四选一）
export GOOSE_CONTEXT_STRATEGY=summarize  # 自动总结（推荐）
export GOOSE_CONTEXT_STRATEGY=truncate   # 自动删除最老消息
export GOOSE_CONTEXT_STRATEGY=clear      # 自动清空会话上下文

# 或者每次都提示用户
export GOOSE_CONTEXT_STRATEGY=prompt
```

达到上下文上限后，实际行为取决于你的配置：

**默认设置（未配置 `GOOSE_CONTEXT_STRATEGY`）** 下，会看到如下选择：

```sh
◇  The model's context length is maxed out. You will need to reduce the # msgs. Do you want to?
│  ○ Clear Session   
│  ○ Truncate Message
// highlight-start
│  ● Summarize Session
// highlight-end

final_summary: [A summary of your conversation will appear here]

Context maxed out
--------------------------------------------------
goose summarized messages for you.
```

**如果已配置 `GOOSE_CONTEXT_STRATEGY`**，goose 会自动应用对应策略：

```sh
# 例如：GOOSE_CONTEXT_STRATEGY=summarize
Context maxed out - automatically summarized messages.
--------------------------------------------------
goose automatically summarized messages for you.

# 例如：GOOSE_CONTEXT_STRATEGY=truncate
Context maxed out - automatically truncated messages.
--------------------------------------------------
goose tried its best to truncate messages for you.

# 例如：GOOSE_CONTEXT_STRATEGY=clear
Context maxed out - automatically cleared session.
--------------------------------------------------
```
  </TabItem>
</Tabs>

## 最大轮数 {#maximum-turns}
`Max Turns` 表示在没有用户输入的情况下，goose 最多可以连续执行多少轮（默认值：1000）。达到上限时，goose 会提示：

“I've reached the maximum number of actions I can do without user input. Would you like me to continue?”

如果用户选择继续，goose 会再运行到上限，然后再次提示。

这个特性可以帮助你控制 agent 的自治程度，防止无限循环和失控行为，避免在生产环境里带来额外成本或破坏性结果。常见用途包括：

- 防止自动任务中出现无限循环、过量 API 调用或资源消耗
- 在自治执行过程中加入人工确认节点
- 在调试 agent 行为时更容易控制迭代节奏

这个设置会存储为你 [config.yaml](../config-files.md) 中的 `GOOSE_MAX_TURNS` 环境变量。你既可以通过 Desktop 设置，也可以通过 CLI 配置。

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>

      1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
      2. 点击侧边栏中的 `Settings`
      3. 点击 `Chat` 标签
      4. 滚动到 `Conversation Limits`，给 `Max Turns` 设置一个值
        
    </TabItem>
    <TabItem value="cli" label="goose CLI">

      1. 运行 `configuration` 命令：
      ```sh
      goose configure
      ```

      2. 选择 `goose settings`：
      ```sh
      ┌   goose-configure
      │
      ◆  What would you like to configure?
      │  ○ Configure Providers
      │  ○ Add Extension
      │  ○ Toggle Extensions
      │  ○ Remove Extension
      // highlight-start
      │  ● goose settings (Set the goose mode, Tool Output, Tool Permissions, Experiment, goose recipe github repo and more)
      // highlight-end
      └ 
      ```

      3. 选择 `Max Turns`：
      ```sh
      ┌   goose-configure
      │
      ◇  What would you like to configure?
      │  goose settings
      │
      ◆  What setting would you like to configure?
      │  ○ goose mode 
      │  ○ Router Tool Selection Strategy 
      │  ○ Tool Permission 
      │  ○ Tool Output 
      // highlight-start
      │  ● Max Turns (Set maximum number of turns without user input)
      // highlight-end
      │  ○ Toggle Experiment 
      │  ○ goose recipe github repo 
      │  ○ Scheduler Type 
      └ 
      ```

      4. 输入最大轮数：
      ```sh
      ┌   goose-configure 
      │
      ◇  What would you like to configure?
      │  goose settings 
      │
      ◇  What setting would you like to configure?
      │  Max Turns 
      │
        // highlight-start
      ◆  Set maximum number of agent turns without user input:
      │  10
        // highlight-end
      │
      └  Set maximum turns to 10 - goose will ask for input after 10 consecutive actions
      ```

      :::tip
      除了持久化的 `Max Turns` 设置，你也可以通过 `goose session --max-turns` 和 `goose run --max-turns` 在单次任务或单个会话上临时覆盖这个值。
      :::

    </TabItem>
    
</Tabs>

**如何选择合适的值**

这个值该设多大，取决于你的任务类型，以及你对自动化程度的接受度：

- **5-10 轮**：适合探索型任务、调试阶段，或者你想频繁确认过程的场景。例如“分析这个代码库并给出改进建议”
- **25-50 轮**：适合中等复杂度、边界明确的任务，例如“把这个模块重构到新 API”或“搭一个基础 CI/CD 流水线”
- **100+ 轮**：适合复杂、多步骤、你又愿意让 goose 更独立推进的任务，例如“把整个项目从 React 16 迁到 React 18”或“为这个服务补齐全面测试覆盖”

要注意的是，看起来简单的任务往往也需要多轮。例如“修好这些 failing tests”，可能就会经历：分析测试输出（1 轮）、找根因（1 轮）、修改代码（1 轮）、验证修复（1 轮）。

## Token 使用量 {#token-usage}
发送第一条消息之后，goose Desktop 和 goose CLI 都会显示 token 使用情况。

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
    Desktop 会在会话窗口底部、模型名称旁边显示一个彩色圆点。颜色可以快速反映本次会话的 token 使用状态：
      - **绿色**：正常状态，可用上下文还很多
      - **橙色**：预警状态，接近上限（达到 80%）
      - **红色**：错误状态，已经触达上下文上限
    
    把鼠标悬停到这个圆点上，会看到：
      - 当前已使用的 token 数
      - 占总可用 token 的百分比
      - 总可用 token 数
      - 一个展示当前使用比例的进度条
        
    </TabItem>
    <TabItem value="cli" label="goose CLI">
    CLI 会在每条命令提示符上方显示 context 标签，内容包括：
      - 用圆点（●○）和颜色表示 token 使用状态：
        - **绿色**：低于 50%
        - **黄色**：在 50%-85% 之间
        - **红色**：高于 85%
      - 当前使用百分比
      - 当前 token 数和上下文上限

    </TabItem>
</Tabs>

## 模型上下文上限覆盖 {#model-context-limit-overrides}

goose 会根据模型名称自动判断上下文上限，但你也可以手动覆盖默认值：

| Model | 说明 | 适用场景 | 设置项 |
|-------|-------------|----------|---------|
| **Main** | 为主模型设置上下文上限（也会作为其他模型的 fallback） | LiteLLM 代理、自定义模型名 | `GOOSE_CONTEXT_LIMIT` |
| **Lead** | 为 [lead/worker mode](/zh-CN/docs/tutorials/lead-worker) 中的规划模型设置更大上下文 | 需要大上下文的复杂规划任务 | `GOOSE_LEAD_CONTEXT_LIMIT` |
| **Worker** | 为 lead/worker mode 中的执行模型设置较小上下文 | 执行阶段节省成本 | `GOOSE_WORKER_CONTEXT_LIMIT` |
| **Planner** | 为[planner model](../creating-plans.md)设置上下文上限 | 大规模规划任务 | `GOOSE_PLANNER_CONTEXT_LIMIT` |

:::info
这个设置只影响界面中显示的 token 使用量和进度条。真正的上下文管理仍由底层 LLM 决定，所以无论显示如何，你实际可能比设定上限用得更多或更少。
:::

这个特性在以下场景尤其有用：

- **LiteLLM 代理模型**：模型名不符合 goose 的内置命名模式
- **企业内部部署**：模型命名不标准的内部部署
- **微调模型**：上下文上限和基础模型不同
- **开发 / 测试**：临时调整上下文上限用于测试

goose 解析上下文上限时遵循以下优先级（从高到低）：

1. 模型配置中的显式 `context_limit`（如果是程序化设置）
2. 对应模型的专用环境变量（如 `GOOSE_LEAD_CONTEXT_LIMIT`）
3. 全局环境变量（`GOOSE_CONTEXT_LIMIT`）
4. 根据模型名模式匹配出的默认值
5. 全局默认值（128,000 tokens）

**配置方式**

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

     goose Desktop 目前还不支持在图形界面中设置模型上下文上限覆盖。

  </TabItem>
  <TabItem value="cli" label="goose CLI">

    上下文上限覆盖只能通过[环境变量](/zh-CN/docs/guides/environment-variables#model-context-limit-overrides)设置，不能写在 config file 里。

    ```bash
    export GOOSE_CONTEXT_LIMIT=1000
    goose session
    ```

  </TabItem>
    
</Tabs>

**场景示例**

1. 使用自定义模型名的 LiteLLM 代理

```bash
# LiteLLM proxy with custom model name
export GOOSE_PROVIDER="openai"
export GOOSE_MODEL="my-custom-gpt4-proxy"
export GOOSE_CONTEXT_LIMIT=200000  # Override the 32k default
```

2. Lead / worker 使用不同上下文上限

```bash
# Different context limits for planning vs execution
export GOOSE_LEAD_MODEL="claude-opus-custom"
export GOOSE_LEAD_CONTEXT_LIMIT=500000    # Large context for planning
export GOOSE_WORKER_CONTEXT_LIMIT=128000  # Smaller context for execution
```

3. Planner 使用超大上下文

```bash
# Large context for complex planning
export GOOSE_PLANNER_MODEL="gpt-4-custom"
export GOOSE_PLANNER_CONTEXT_LIMIT=1000000
```

## 成本跟踪 {#cost-tracking}
显示当前会话的实时成本估算。

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
要管理实时成本跟踪：
  1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
  2. 点击侧边栏中的 `Settings`
  3. 点击 `App` 标签
  4. 打开或关闭 `Cost Tracking`

会话成本会显示在 goose 窗口底部，并随着 token 消耗实时更新。把鼠标移到成本数字上，可以看到更细的 token 使用明细。如果会话里用了多个模型，还会按模型拆分显示。对于 Ollama 和本地部署，成本始终显示为 `$0.00`。

价格数据会定期从 OpenRouter API 拉取并缓存在本地。你可以在 `Advanced settings` 标签中查看上次更新时间，也可以手动刷新。

这些成本只是估算值，并不会和你的 provider 实际账单直接对齐。显示结果是基于 token 数和公开价格数据计算出的近似值。
</TabItem>
    <TabItem value="cli" label="goose CLI">
    如果要在 goose CLI 中显示估算成本，可以设置 `GOOSE_CLI_SHOW_COST` [环境变量](/zh-CN/docs/guides/environment-variables#session-management)，或者把它写进 [configuration file](../config-files.md)。

  ```yaml
  # Set environment variable
  export GOOSE_CLI_SHOW_COST=true

  # config.yaml
  GOOSE_CLI_SHOW_COST: true
  ```
  </TabItem>
</Tabs>
