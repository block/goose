---
sidebar_position: 2
title: Recipe 参考指南
sidebar_label: Recipe Reference
description: 创建与定制 goose recipe 的完整技术参考
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Recipes 是可复用的 goose 配置，用来把 instructions、settings 和扩展等内容打包起来，方便分享给他人或重复执行。

## Recipe 文件格式

Recipes 可以定义为以下格式：
- `.yaml`（推荐）和 `.yml`
- `.json`

:::info
goose CLI 不支持 `.yml` 文件。
:::

如需了解如何创建、使用和管理 recipes，请先阅读[可分享 Recipes](/zh-CN/docs/guides/recipes/session-recipes)。

## Recipe 存放位置

Recipe 可以从以下来源加载：

1. 本地文件系统：
   - 当前目录
   - [`GOOSE_RECIPE_PATH`](/zh-CN/docs/guides/environment-variables) 环境变量指定的目录

2. GitHub 仓库：
   - 通过 [`GOOSE_RECIPE_GITHUB_REPO`](/zh-CN/docs/guides/environment-variables) 配置项指定
   - 需要本机已安装并完成认证的 GitHub CLI（`gh`）

## 核心 Recipe Schema {#core-recipe-schema}

Recipes 遵循如下 schema 结构：

| 字段 | 类型 | 必填 | 说明 |
|-------|------|------|------|
| `description` | String | ✅ | 详细描述 recipe 做什么 |
| `instructions` | String | ✅* | 模板 instructions，可包含参数替换 |
| `prompt` | String | ✅* | 模板 prompt，也可包含参数替换。在 [headless](/zh-CN/docs/tutorials/headless-goose)（非交互）模式下必填 |
| `title` | String | ✅ | 简短标题，用来描述 recipe |
| [`activities`](#activities) | Array | - | 示例 prompt 列表，可包含参数替换。会在 goose Desktop 中显示为可点击气泡 |
| [`extensions`](#extensions) | Array | - | 扩展配置列表 |
| [`parameters`](#parameters) | Array | - | 动态 recipe 所需的参数定义列表 |
| [`response`](#response) | Object | - | 自动化工作流使用的结构化输出 schema |
| [`retry`](#retry) | Object | - | 自动重试与成功校验配置 |
| [`settings`](#settings) | Object | - | 模型 provider、模型名及其他设置 |
| [`sub_recipes`](#subrecipes) | Array | - | subrecipes 列表 |
| `version` | String | - | recipe 格式版本；省略时默认为 `"1.0.0"` |

\* `instructions` 和 `prompt` 至少要提供一个。

## 字段说明

### Activities {#activities}

`activities` 字段用于定义一个可选提示消息，以及一组会显示在 goose Desktop 中的可点击活动气泡（按钮）。

:::info 仅限 Desktop
Activities 是 Desktop 专属功能。通过 CLI 或定时任务运行包含 activities 的 recipe 时，`activities` 字段会被忽略，不影响 recipe 执行。
:::

#### Activity 类型

Activities 有两种定义方式：

1. **消息型 Activity**：在活动气泡上方的信息框中显示 Markdown 格式的提示文字。例如：

   ```
   activities:
     - "message: **Welcome!** Here's what I can help with:\n\n• 📊 Data analysis\n• 🔍 Code review\n• 📝 Documentation\n\nSelect an option below to begin."
   ```

   只应包含一个带 `message:` 前缀的 activity。额外的 `message:` activity 会退化为普通按钮，并直接显示字面上的 `message:` 文本。

2. **按钮型 Activity**：显示在活动气泡中的文本；点击后会把该文本作为 prompt 发送出去。

#### 参数替换

Activities 支持[参数替换](#parameters)。用户在 **Recipe Parameters** 对话框中填好参数值后，这些值会先替换到 activity 文本里，再显示到气泡上。

#### 配置示例

<Tabs groupId="format">
  <TabItem value="yaml" label="YAML" default>
    ```yaml
    version: "1.0.0"
    title: "Code Review Assistant"
    description: "Review code with customizable focus areas"
    parameters:
      - key: language
        input_type: string
        requirement: required
        description: "Programming language to review"
      - key: focus
        input_type: string
        requirement: optional
        default: "best practices"
        description: "Review focus area"

    activities:
      - "message: Click an option below to start reviewing {{ language }} code with a focus on {{ focus }}."
      - "Review the current file for {{ focus }}"
      - "Suggest improvements for {{ language }} code quality"
      - "Check for security vulnerabilities"
      - "Generate unit tests"
    ```
  </TabItem>
  <TabItem value="json" label="JSON">
    ```json
    {
      "version": "1.0.0",
      "title": "Code Review Assistant",
      "description": "Review code with customizable focus areas",
      "parameters": [
        {
          "key": "language",
          "input_type": "string",
          "requirement": "required",
          "description": "Programming language to review"
        },
        {
          "key": "focus",
          "input_type": "string",
          "requirement": "optional",
          "default": "best practices",
          "description": "Review focus area"
        }
      ],
      "activities": [
        "message: Click an option below to start reviewing {{ language }} code with a focus on {{ focus }}.",
        "Review the current file for {{ focus }}",
        "Suggest improvements for {{ language }} code quality",
        "Check for security vulnerabilities",
        "Generate unit tests"
      ]
    }
    ```
  </TabItem>
</Tabs>

在这个示例中：
- 消息型 activity 会显示替换后的提示语，例如：“Click an option below to start reviewing rust code with a focus on best practices.”
- 前两个活动气泡会进行参数替换，例如：“Review the current file for best practices”
- 最后两个活动气泡是静态 prompt，不依赖参数

### Extensions {#extensions}

`extensions` 字段用于声明 recipe 运行所需的 Model Context Protocol (MCP) servers 或其他扩展。`extensions` 数组中的每一项都遵循以下 schema：

#### Extension Schema

| 字段 | 类型 | 说明 |
|-------|------|------|
| `type` | String | 扩展类型，例如 `"stdio"` |
| `name` | String | 扩展的唯一名称 |
| `cmd` | String | 启动扩展的命令 |
| `args` | Array | 命令参数列表 |
| `env_keys` | Array | （可选）扩展所需环境变量名称 |
| `timeout` | Number | 超时时间，单位秒 |
| `bundled` | Boolean | （可选）是否由 goose 内置提供 |
| `description` | String | 扩展用途说明 |
| `available_tools` | Array | 限定对外暴露的工具名；未指定时表示全部可用 |

#### Extension 类型

- **`stdio`**：基于标准输入输出的客户端，需要提供命令与参数
- **`builtin`**：打包在 goose MCP server 中的内置扩展
- **`platform`**：在 agent 进程内运行的平台扩展
- **`streamable_http`**：使用 URI 端点的可流式 HTTP 客户端
- **`frontend`**：由前端提供、通过前端调用的工具
- **`inline_python`**：使用 uvx 执行的内联 Python 代码。需要 `code` 字段，可选 `dependencies`

#### Extensions 配置示例

<Tabs groupId="format">
  <TabItem value="yaml" label="YAML" default>

```yaml
extensions:
  - type: stdio
    name: codesearch
    cmd: uvx
    args:
      - mcp_codesearch@latest
    timeout: 300
    bundled: true
    description: "Query https://codesearch.sqprod.co/ directly from goose"
  
  - type: stdio
    name: presidio
    timeout: 300
    cmd: uvx
    args:
      - 'mcp_presidio@latest'
    available_tools:
      - query_logs

  - type: stdio
    name: github-mcp
    cmd: github-mcp-server
    args: []
    env_keys:
      - GITHUB_PERSONAL_ACCESS_TOKEN
    timeout: 60
    description: "GitHub MCP extension for repository operations"
    
  - type: inline_python
    name: data_processor
    code: |
      import pandas as pd
      print("Processing data...")
    dependencies:
      - pandas
      - numpy
    timeout: 120
    description: "Process data using pandas"
```

  </TabItem>
  <TabItem value="json" label="JSON">

```json
{
  "extensions": [
    {
      "type": "stdio",
      "name": "codesearch",
      "cmd": "uvx",
      "args": ["mcp_codesearch@latest"],
      "timeout": 300,
      "bundled": true,
      "description": "Query https://codesearch.sqprod.co/ directly from goose"
    },
    {
      "type": "stdio",
      "name": "presidio",
      "timeout": 300,
      "cmd": "uvx",
      "args": ["mcp_presidio@latest"],
      "available_tools": ["query_logs"]
    },
    {
      "type": "stdio",
      "name": "github-mcp",
      "cmd": "github-mcp-server",
      "args": [],
      "env_keys": ["GITHUB_PERSONAL_ACCESS_TOKEN"],
      "timeout": 60,
      "description": "GitHub MCP extension for repository operations"
    },
    {
      "type": "inline_python",
      "name": "data_processor",
      "code": "import pandas as pd\nprint(\"Processing data...\")",
      "dependencies": ["pandas", "numpy"],
      "timeout": 120,
      "description": "Process data using pandas"
    }
  ]
}
```

  </TabItem>
</Tabs>

#### Extension Secrets {#extension-secrets}

这个功能目前只在 CLI 中可用。

如果 recipe 使用的 extension 依赖 secret，goose 在运行 recipe 时可以提示用户输入对应 secret：

1. recipe 加载后，goose 会扫描所有 extensions（包括 subrecipes 中的 extensions）是否声明了 `env_keys`
2. 如果系统安全 keyring 里缺少所需环境变量，goose 会提示用户输入
3. 输入的值会安全地保存在系统 keyring 中，之后运行可直接复用

如果你要更新已保存的 secret，请先从系统 keyring 中删除该项，然后再次运行 recipe，以便重新触发输入。

:::info
这个能力主要用于提示并安全保存 secrets（例如 API keys），但 `env_keys` 也可以声明任意 extension 所需的环境变量，例如 API endpoint、普通配置值等。

如果某个变量对 extension 来说是可选的，用户可以按 `ESC` 跳过输入。
:::

### Parameters {#parameters}

`parameters` 字段用于创建可动态定制、可复用的 recipes。它允许 recipe 在运行时由用户填入具体值，从而适应不同上下文。

参数替换采用 Jinja 风格模板语法，通过 `{{ parameter_name }}` 占位。`parameters` 数组中的每个参数都遵循以下 schema：

#### Parameter Schema

| 字段 | 类型 | 必填 | 说明 |
|-------|------|------|------|
| `key` | String | ✅ | 参数唯一标识 |
| `input_type` | String | ✅ | 输入类型：`"string"`（默认）、`"number"`、`"boolean"`、`"date"`、`"file"` 或 `"select"` |
| `requirement` | String | ✅ | 取值只能是 `"required"`、`"optional"` 或 `"user_prompt"` |
| `description` | String | ✅ | 面向用户的说明文字 |
| `default` | String | - | 可选参数的默认值 |
| `options` | Array | - | 备选项列表，`select` 类型时必填 |

#### 参数要求

- `required`：使用 recipe 时必须提供该参数
- `optional`：如果给出了默认值，可以省略
- `user_prompt`：若运行时未提供，系统会以交互方式提示输入

`required` 和 `optional` 适合在 goose Desktop 中使用。如果 `user_prompt` 参数没有值，它不会被替换，最终可能在输出里以字面文本 `{{ parameter_name }}` 形式出现。

#### 输入类型

- `string`：默认类型，按原样替换
- `number`：数字值；Desktop 会提供数字校验
- `boolean`：布尔值；Desktop 会显示 `"True"` / `"False"` 下拉框
- `date`：日期值；当前渲染为普通文本输入
- `file`：参数值应为文件路径；goose 会读取文件内容并把**内容**而不是路径替换进模板
- `select`：预定义选项下拉框，必须提供 `options`

**示例：**
```yaml
parameters:
  - key: max_files
    input_type: number
    requirement: optional
    default: "10"
    description: "Maximum files to process"
  
  - key: output_format
    input_type: select
    requirement: required
    description: "Choose output format"
    options:
      - json
      - markdown
      - csv
  
  - key: enable_debug
    input_type: boolean
    requirement: optional
    default: "false"
    description: "Enable debug mode"
  
  - key: source_code
    input_type: file
    requirement: required
    description: "Path to the source code file to analyze"

prompt: "Process {{ max_files }} files in {{ output_format }} format. Debug: {{ enable_debug }}. Code:\n\n{{ source_code }}"
```

:::important
- `optional` 参数必须提供默认值
- `required` 参数不能提供默认值
- `file` 参数无论 requirement 类型是什么，都不能有默认值，以避免意外读取敏感文件
- `select` 参数必须提供 `options`
- 参数 `key` 必须与 instructions、prompt 或 activities 中使用的模板变量一一对应
:::

#### Desktop 中的参数替换

当带参数的 recipe 在 goose Desktop 中打开时，用户会看到 **Recipe Parameters** 对话框，可以：
- 为必填参数提供值
- 接受或修改可选参数的默认值
- 为 `user_prompt` 参数输入内容

提交后，这些值会在 recipe 启动前被替换进 `instructions`、`prompt` 和 `activities`。

### Response {#response}

`response` 字段用于强制 recipe 最终输出结构化 JSON。当你声明 `json_schema` 后，goose 会：

1. **校验输出**：用基本 JSON schema 规则校验最终 JSON
2. **确保结构化输出**：要求 agent 的最终输出符合你定义的 JSON 结构

这个能力主要面向**非交互式自动化**，用于保证输出稳定、可解析。无论 recipe 通过 goose CLI 还是 goose Desktop 运行，都可以生成结构化输出。更多使用场景见[可分享 Recipes](/zh-CN/docs/guides/recipes/session-recipes#structured-output-for-automation)。

#### Response Schema

| 字段 | 类型 | 必填 | 说明 |
|-------|------|------|------|
| `json_schema` | Object | ✅ | 用于输出校验的 [JSON schema](https://json-schema.org/) |

#### 基础结构

```yaml
response:
  json_schema:
    type: object
    properties:
      # 在这里定义字段、类型和说明
    required:
      # 列出必填字段名
```

#### 简单示例

```yaml
version: "1.0.0"
title: "Task Summary"
description: "Summarize completed tasks"
prompt: "Summarize the tasks you completed"
response:
  json_schema:
    type: object
    properties:
      summary:
        type: string
        description: "Brief summary of work done"
      tasks_completed:
        type: number
        description: "Number of tasks finished"
      next_steps:
        type: array
        items:
          type: string
        description: "Recommended next actions"
    required:
      - summary
      - tasks_completed
```

### Retry {#retry}

`retry` 字段允许 recipe 在未满足成功条件时自动重试。它非常适合以下场景：
- 任务可能需要多次尝试才能成功
- 需要自动校验与恢复的工作流
- 希望把“运行 -> 检查 -> 清理 -> 重试”做成统一流程

#### Retry Schema

| 字段 | 类型 | 必填 | 说明 |
|-------|------|------|------|
| `max_retries` | Number | ✅ | 最大重试次数 |
| `checks` | Array | ✅ | 成功校验配置列表 |
| `timeout_seconds` | Number | - | 成功检查命令的超时时间（默认 300 秒） |
| `on_failure_timeout_seconds` | Number | - | `on_failure` 命令超时时间（默认 600 秒） |
| `on_failure` | String | - | 某次尝试失败后执行的 shell 命令 |

#### 成功检查配置

`checks` 数组中的每一项遵循以下 schema：

| 字段 | 类型 | 必填 | 说明 |
|-------|------|------|------|
| `type` | String | ✅ | 检查类型；当前只支持 `"shell"` |
| `command` | String | ✅ | 作为校验执行的 shell 命令，退出码必须为 0 才算成功 |

#### Retry 逻辑如何工作

1. **执行 recipe**：recipe 按正常方式运行
2. **成功校验**：任务完成后，按顺序执行所有成功检查
3. **决定是否重试**：如果任一检查失败且还有剩余次数：
   - 先执行 `on_failure`（如果有）
   - 重置 agent 的消息历史到初始状态
   - 重试计数加一，并重新开始执行
4. **结束条件**：以下两种情况之一出现时停止：
   - 所有成功检查通过
   - 达到最大重试次数

#### 基础重试示例

```yaml
version: "1.0.0"
title: "Counter Increment Task"
description: "Increment a counter until it reaches target value"
prompt: "Increment the counter value in /tmp/counter.txt by 1."

retry:
  max_retries: 5
  timeout_seconds: 10
  checks:
    - type: shell
      command: "test $(cat /tmp/counter.txt 2>/dev/null || echo 0) -ge 3"
  on_failure: "echo 'Counter is at:' $(cat /tmp/counter.txt 2>/dev/null || echo 0) '(need 3 to succeed)'"
```

#### 进阶重试示例

```yaml
version: "1.0.0"
title: "Service Health Check"
description: "Start service and verify it's running properly"
prompt: "Start the web service and verify it responds to health checks"

retry:
  max_retries: 3
  timeout_seconds: 30
  on_failure_timeout_seconds: 60
  checks:
    - type: shell
      command: "curl -f http://localhost:8080/health"
    - type: shell  
      command: "pgrep -f 'web-service' > /dev/null"
  on_failure: "systemctl stop web-service || killall web-service"
```

#### 环境变量

你也可以通过环境变量全局配置 retry 行为：

- `GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS`：成功检查命令的全局超时时间
- `GOOSE_RECIPE_ON_FAILURE_TIMEOUT_SECONDS`：`on_failure` 命令的全局超时时间

一旦 recipe 内显式配置了超时值，会覆盖这些全局环境变量。

### Settings {#settings}

`settings` 字段用于给 recipe 指定 AI 模型和 provider 设置。recipe 运行时，这些设置会覆盖用户默认配置。

#### Settings Schema

| 字段 | 类型 | 必填 | 说明 |
|-------|------|------|------|
| `goose_provider` | String | - | 使用的 AI provider，例如 `"anthropic"`、`"openai"` |
| `goose_model` | String | - | 要使用的具体模型名 |
| `temperature` | Number | - | 模型 temperature，通常在 0.0 到 1.0 之间 |
| `max_turns` | Number | - | 由该 recipe 创建的 subagent 任务允许的最大轮数 |

#### 理解 max_turns

`max_turns` 用来控制 agent 在停止前最多能迭代多少次。它配置在 recipe 的 `settings` 中后，会作用于该 recipe 的执行，以及它创建的 subagents 或 subrecipes（除非这些子项自己显式声明了更具体的值）。

**配置优先级（从高到低）：**
1. Subagent 工具调用时的显式覆盖
2. Recipe 的 `settings.max_turns`
3. `GOOSE_SUBAGENT_MAX_TURNS` 环境变量
4. 默认值（主 recipe 为 1000，subagents 为 25）

**常见场景：** 限制自动化工作流执行时间、防止 subagent 失控、控制定时任务资源消耗。

#### Settings 配置示例

```yaml
settings:
  goose_provider: "anthropic"
  goose_model: "claude-sonnet-4-20250514"
  temperature: 0.7
  max_turns: 50
```

```yaml
settings:
  goose_provider: "openai"
  goose_model: "gpt-4o"
  temperature: 0.3
```

:::note
Recipe 中声明的 settings 会在运行时覆盖你本地的默认 goose 配置。如果没有配置 settings，goose 会继续使用你的默认设置。
:::

### Subrecipes {#subrecipes}

`sub_recipes` 字段用于声明主 recipe 会调用哪些[subrecipes](/zh-CN/docs/guides/recipes/subrecipes)来完成特定任务。`sub_recipes` 数组中的每一项遵循以下 schema：

#### Subrecipe Schema

| 字段 | 类型 | 必填 | 说明 |
|-------|------|------|------|
| `name` | String | ✅ | subrecipe 的唯一标识 |
| `path` | String | ✅ | subrecipe 文件的相对或绝对路径 |
| `values` | Object | - | 预设参数值，会传递给 subrecipe |
| `sequential_when_repeated` | Boolean | - | 强制同一 subrecipe 的多次调用改为串行执行。详见[并行运行 Subrecipes](/zh-CN/docs/tutorials/subrecipes-in-parallel) |
| `description` | String | - | 可选说明文字 |

#### Subrecipe 配置示例

```yaml
sub_recipes:
  - name: "security_scan"
    path: "./subrecipes/security-analysis.yaml"
    values:  # key-value 形式：{parameter_name}: {parameter_value}
      scan_level: "comprehensive"
      include_dependencies: "true"
  
  - name: "quality_check"
    path: "./subrecipes/quality-analysis.yaml"
    description: "Performs code quality analysis"
```

## Desktop 元数据字段

通过 goose Desktop 保存的 recipe 会包含额外的元数据字段。这些字段主要供 Desktop 端组织和管理使用，CLI 操作会忽略它们。

| 字段 | 类型 | 必填 | 说明 |
|-------|------|------|------|
| `recipe` | Object | ✅ | 包含真正的 recipe 字段（`title`、`description`、`instructions` 等） |
| `name` | String | ✅ | 在 Recipe Library 中显示的名称 |
| `isGlobal` | Boolean | ✅ | 是否作为全局 recipe 提供 |
| `lastModified` | String | ✅ | 最近修改时间，使用 ISO 时间戳 |
| `isArchived` | Boolean | ✅ | 是否已在 Desktop 界面中归档 |

<details>
<summary>CLI 与 Desktop 格式示例</summary>

**CLI 格式**

<Tabs groupId="format">
  <TabItem value="yaml" label="YAML" default>

```yaml
version: "1.0.0"
title: "Code Review Assistant"
description: "Automated code review with best practices"
instructions: "You are a code reviewer..."
prompt: "Review the code in this repository"
extensions: []
```

  </TabItem>
  <TabItem value="json" label="JSON">

```json
{
  "version": "1.0.0",
  "title": "Code Review Assistant",
  "description": "Automated code review with best practices",
  "instructions": "You are a code reviewer...",
  "prompt": "Review the code in this repository",
  "extensions": []
}
```

  </TabItem>
</Tabs>

**Desktop 格式**

<Tabs groupId="format">
  <TabItem value="yaml" label="YAML" default>

```yaml
name: "Code Review Assistant"
recipe:
  version: "1.0.0"
  title: "Code Review Assistant"
  description: "Automated code review with best practices"
  instructions: "You are a code reviewer..."
  prompt: "Review the code in this repository"
  extensions: []
isGlobal: true
lastModified: 2025-07-02T03:46:46.778Z
isArchived: false
```

  </TabItem>
  <TabItem value="json" label="JSON">

```json
{
  "name": "Code Review Assistant",
  "recipe": {
    "version": "1.0.0",
    "title": "Code Review Assistant",
    "description": "Automated code review with best practices",
    "instructions": "You are a code reviewer...",
    "prompt": "Review the code in this repository",
    "extensions": []
  },
  "isGlobal": true,
  "lastModified": "2025-07-02T03:46:46.778Z",
  "isArchived": false
}
```

  </TabItem>
</Tabs>

</details>

## 模板支持 {#template-support}

Recipes 在 `instructions`、`prompt` 和 `activities` 中支持 Jinja 风格模板语法，用于参数替换：

```yaml
instructions: "Follow these steps with {{ parameter_name }}"
prompt: "Your task is to {{ action }}"
activities:
  - "Process {{ parameter_name }} with {{ action }}"
```

进阶模板能力包括：
- [转义模板变量](#escaping-template-variables)，用于输出字面量
- [模板继承](#template-inheritance)，通过 `{% extends "parent.yaml" %}` 复用父模板
- 可定义并重写的 block：
  ```yaml
  {% block content %}
  Default content
  {% endblock %}
  ```
- [`indent()` 模板过滤器](#indent-filter-for-multi-line-values)，用于处理多行值

### 转义模板变量 {#escaping-template-variables}

如果你想在 recipe 中输出字面量模板语法（例如 `{{ variable }}`），而不是让它被替换，可以把它包在单引号里：

```yaml
prompt: |
  This will be substituted: {{ actual_parameter }}
  This will appear literally: {{'{{example_variable}}'}}
```

**示例：生成配置模板**

```yaml
version: "1.0.0"
title: "Generate Config Template"
description: "Generate a template with placeholder values"
parameters:
  - key: app_name
    input_type: string
    requirement: required
    description: "Application name"

prompt: |
  Create a config.yaml file for {{ app_name }} with these placeholder variables:
  - {{'{{API_KEY}}'}} for the API key
  - {{'{{DATABASE_URL}}'}} for the database connection
  - {{'{{PORT}}'}} for the server port
```

### 模板继承 {#template-inheritance}

使用 `{% extends "parent.yaml" %}` 可以继承父模板：

**父 Recipe（`parent.yaml`）：**
```yaml
version: "1.0.0"
title: "Parent Recipe"
description: "Base recipe template"
prompt: |
  {% block prompt %}
  Default prompt text
  {% endblock %}
```

**子 Recipe：**
```yaml
{% extends "parent.yaml" %}
{% block prompt %}
Modified prompt text
{% endblock %}
```

### 用于多行值的 indent() 过滤器 {#indent-filter-for-multi-line-values}

如果要把多行参数值传入 subrecipe，并希望最终仍是合法 JSON 或 YAML，请使用 `indent()` 过滤器。下面的示例中，通过 `{{ raw_data | indent(2) }}` 指定用两个空格缩进：

```yaml
sub_recipes:
  - name: "analyze"
    path: "./analyze.yaml"
    values:
      content: |
        {{ raw_data | indent(2) }}
```

### 内置参数

内置模板参数会自动提供，无需手动在 `parameters` 中声明。

| 参数 | 说明 |
|-----------|------|
| `recipe_dir` | 自动设置为 recipe 文件所在目录。可用于引用同目录资源，例如：`{{ recipe_dir }}/style-guide.md` |

## 校验规则

加载 recipe 时，以及运行 [`goose recipe validate`](/zh-CN/docs/guides/goose-cli-commands#recipe) 子命令时，都会应用 [`validate_recipe.rs`](https://github.com/block/goose/blob/main/crates/goose/src/recipe/validate_recipe.rs) 中定义的校验规则：

### Recipe 级校验

- `validate_prompt_or_instructions`：`instructions` 或 `prompt` 至少存在一个
- `validate_json_schema`：如果配置了 `response.json_schema`，其 JSON schema 必须合法

### 参数校验

- `validate_parameters_in_template`：所有模板变量都必须有对应参数定义，且所有已定义参数都必须实际被使用
- `validate_optional_parameters`：`optional` 参数必须提供默认值
- `validate_optional_parameters`：`file` 参数不能提供默认值，以避免误导入敏感文件

:::info
更基础的字段要求（必填字段、类型、长度限制等）已经在[核心 Recipe Schema](#core-recipe-schema)表格中说明。
:::

## 完整 Recipe 示例

<Tabs groupId="format">
  <TabItem value="yaml" label="YAML" default>

```yaml
version: "1.0.0"
title: "Example Recipe"
description: "A sample recipe demonstrating the format"
instructions: "Process {{ file_count }} files using {{ required_param }} and output in {{ output_format }} format. Configuration: {{ config_file }}"
prompt: "Start processing with the provided parameters"
parameters:
  - key: required_param
    input_type: string
    requirement: required
    description: "A required text parameter"
  
  - key: file_count
    input_type: number
    requirement: optional
    default: 10
    description: "Maximum number of files to process"
  
  - key: output_format
    input_type: select
    requirement: required
    description: "Choose the output format"
    options:
      - json
      - markdown
      - csv
  
  - key: config_file
    input_type: file
    requirement: required
    description: "Path to configuration file"

extensions:
  - type: stdio
    name: codesearch
    cmd: uvx
    args:
      - mcp_codesearch@latest
    timeout: 300
    bundled: true
    description: "Query codesearch directly from goose"

settings:
  goose_provider: "anthropic"
  goose_model: "claude-sonnet-4-20250514"
  temperature: 0.7
  max_turns: 100

retry:
  max_retries: 3
  timeout_seconds: 30
  checks:
    - type: shell
      command: "echo 'Task validation check passed'"
  on_failure: "echo 'Retry attempt failed, cleaning up...'"

response:
  json_schema:
    type: object
    properties:
      result:
        type: string
        description: "The main result of the task"
      details:
        type: array
        items:
          type: string
        description: "Additional details of steps taken"
    required:
      - result
      - details
```

  </TabItem>
  <TabItem value="json" label="JSON">

```json
{
  "version": "1.0.0",
  "title": "Example Recipe",
  "description": "A sample recipe demonstrating the format",
  "instructions": "Process {{ file_count }} files using {{ required_param }} and output in {{ output_format }} format. Configuration: {{ config_file }}",
  "prompt": "Start processing with the provided parameters",
  "parameters": [
    {
      "key": "required_param",
      "input_type": "string",
      "requirement": "required",
      "description": "A required text parameter"
    },
    {
      "key": "file_count",
      "input_type": "number",
      "requirement": "optional",
      "default": "10",
      "description": "Maximum number of files to process"
    },
    {
      "key": "output_format",
      "input_type": "select",
      "requirement": "required",
      "description": "Choose the output format",
      "options": ["json", "markdown", "csv"]
    },
    {
      "key": "config_file",
      "input_type": "file",
      "requirement": "required",
      "description": "Path to configuration file"
    }
  ],
  "extensions": [
    {
      "type": "stdio",
      "name": "codesearch",
      "cmd": "uvx",
      "args": ["mcp_codesearch@latest"],
      "timeout": 300,
      "bundled": true,
      "description": "Query codesearch directly from goose"
    }
  ],
  "settings": {
    "goose_provider": "anthropic",
    "goose_model": "claude-sonnet-4-20250514",
    "temperature": 0.7,
    "max_turns": 100
  },
  "retry": {
    "max_retries": 3,
    "timeout_seconds": 30,
    "checks": [
      {
        "type": "shell",
        "command": "echo 'Task validation check passed'"
      }
    ],
    "on_failure": "echo 'Retry attempt failed, cleaning up...'"
  },
  "response": {
    "json_schema": {
      "type": "object",
      "properties": {
        "result": {
          "type": "string",
          "description": "The main result of the task"
        },
        "details": {
          "type": "array",
          "items": {
            "type": "string"
          },
          "description": "Additional details of steps taken"
        }
      },
      "required": ["result", "details"]
    }
  }
}
```

  </TabItem>
</Tabs>

## 错误处理

常见错误包括：

- 缺少必填参数
- 可选参数没有默认值
- 模板变量没有对应的参数定义
- YAML / JSON 语法不合法
- 缺少必填字段
- 扩展配置不合法
- retry 配置不合法（缺字段、shell 命令错误等）

发生这些问题时，goose 会给出可帮助定位问题的错误信息。

### Retry 相关错误

- **无效的成功检查**：shell 命令无法执行或包含语法错误
- **超时错误**：成功检查或 `on_failure` 命令执行时间超过限制
- **超出最大重试次数**：所有尝试都失败后仍未满足成功条件
- **缺少 retry 必填字段**：例如没有声明 `max_retries` 或 `checks`

## 了解更多
继续阅读[Recipes 指南](/zh-CN/docs/guides/recipes)，查看更多文档、工具和资源，进一步掌握 goose recipes。
