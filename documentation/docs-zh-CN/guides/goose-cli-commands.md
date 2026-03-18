---
title: "CLI 命令"
description: "完整说明 goose CLI 命令、交互特性、主题与快捷键。"
sidebar_position: 35
toc_max_heading_level: 4
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# CLI 命令

goose 提供一套完整的命令行接口（CLI），用于管理会话、配置、扩展、recipe 和定时任务。本文覆盖可用命令以及交互式会话里的功能。

## Flag 命名约定

goose CLI 在 flag 命名上尽量保持一致，便于理解和记忆：

- **`--session-id`**：用于 session 标识，例如 `20251108_1`
- **`--schedule-id`**：用于定时任务标识，例如 `daily-report`
- **`-n, --name`**：用于可读名称
- **`--path`**：用于文件路径（保留兼容）
- **`-o, --output`**：用于输出文件路径
- **`-r, --resume` 或 `-r, --regex`**：根据上下文表示恢复会话或正则匹配
- **`-v, --verbose`**：输出更多细节
- **`-l, --limit`**：限制结果数量
- **`-f, --format`**：指定输出格式
- **`-w, --working_dir`**：按工作目录过滤

### 核心命令

#### help

显示帮助菜单。

**用法**

```bash
goose --help
```

---

#### configure

配置 goose 的 provider、扩展等设置。

**用法**

```bash
goose configure
```

:::tip 输入即可过滤
在 `goose configure` 的菜单中，你可以直接输入关键字实时过滤选项。这对 providers、extensions 和 tools 列表都适用。
:::

---

#### info [options]

显示 goose 的版本、配置文件路径、会话存储位置、日志位置等信息。

**选项**

- **`-v, --verbose`**：显示更详细的配置内容，包括环境变量和启用的扩展

**用法**

```bash
goose info
```

---

#### version

查看当前已安装的 goose 版本。

**用法**

```bash
goose --version
```

---

#### update [options]

将 goose CLI 更新到更新的版本。

**选项**

- **`--canary, -c`**：升级到 canary（开发）版本，而不是稳定版
- **`--reconfigure, -r`**：升级时强制重新配置 goose

**用法**

```bash
# 升级到最新稳定版
goose update

# 升级到最新 canary 版
goose update --canary

# 升级并重新配置
goose update --reconfigure
```

---

#### completion

为指定 shell 生成补全脚本，用于自动补全 goose 的命令、子命令和参数。脚本会输出到 stdout，因此你需要将其重定向到 shell 对应的位置，或在 shell 配置中 `eval` 它。

安装后你可以：

- 按 `Tab` 查看可用命令和子命令
- 自动补全命令名和 flag
- 不查 `--help` 也能快速发现可用参数

**参数**

- **`<SHELL>`**：要生成补全脚本的 shell。支持：`bash`、`elvish`、`fish`、`powershell`、`zsh`

**用法**

```bash
# 为当前 shell 生成补全脚本（输出到 stdout）
goose completion bash
goose completion zsh
goose completion fish
```

**按 Shell 安装**

<Tabs groupId="shells">
<TabItem value="zsh" label="Zsh" default>

把下面这行加入 `~/.zshrc`：

```bash
eval "$(goose completion zsh)"
```

然后重新加载：

```bash
source ~/.zshrc
```

</TabItem>
<TabItem value="bash" label="Bash">

把下面这行加入 `~/.bashrc` 或 `~/.bash_profile`：

```bash
eval "$(goose completion bash)"
```

然后重新加载：

```bash
source ~/.bashrc
```

</TabItem>
<TabItem value="fish" label="Fish">

```bash
goose completion fish > ~/.config/fish/completions/goose.fish
```

然后重启终端，或执行 `exec fish`。

</TabItem>
<TabItem value="powershell" label="PowerShell">

把下面这行加入你的 PowerShell profile：

```powershell
goose completion powershell | Out-String | Invoke-Expression
```

然后重新加载：

```powershell
. $PROFILE
```

</TabItem>
</Tabs>

:::tip 测试补全
安装并重新加载后，可以输入 `goose ` 然后按 `Tab` 查看命令，或输入 `goose session --` 然后按 `Tab` 查看选项。
:::

---

### 会话管理

:::info Session 存储迁移
从 `1.10.0` 开始，goose 改用 SQLite 数据库（`sessions.db`）存储会话，而不是单独的 `.jsonl` 文件。
旧会话会自动导入数据库。原先的 `.jsonl` 文件仍会保留在磁盘上，但不再由 goose 管理。
:::

#### session [options]

启动或恢复交互式会话。

**基础选项**

- **`--session-id <session_id>`**：按 session ID 指定会话，例如 `20251108_1`
- **`-n, --name <name>`**：给会话命名
- **`--path <path>`**：按文件路径指定会话（旧参数）
- **`-r, --resume`**：恢复已有会话
- **`--fork`**：复制一份已有会话并从副本继续。必须与 `--resume` 一起使用。可配合 `--name` 或 `--session-id` 指定目标；不指定则 fork 最近一次会话
- **`--history`**：恢复会话时显示历史消息
- **`--container <container_id>`**：在 [Docker container](/zh-CN/docs/tutorials/goose-in-docker#running-extensions-in-docker-containers) 中运行扩展
- **`--debug`**：启用调试模式，输出完整 tool 响应、详细参数值和完整文件路径
- **`--max-tool-repetitions <NUMBER>`**：限制同一工具以相同参数连续调用的次数，避免死循环
- **`--max-turns <NUMBER>`**：设置无需用户输入即可连续运行的最大轮数（默认 `1000`）

**扩展选项**

- **`--with-extension <command>`**：附加 stdio 扩展
- **`--with-streamable-http-extension <url>`**：通过 Streamable HTTP 附加远程扩展
- **`--with-builtin <id>`**：启用内置扩展，例如 `developer`、`computercontroller`

**用法**

```bash
# 启动新会话
goose session -n my-project

# 恢复旧会话
goose session --resume -n my-project
goose session --resume --session-id 20251108_2
goose session --resume --path ./session.json
goose session --resume --path ./session.jsonl

# fork 指定会话
goose session --resume --fork --name my-project

# fork 最近会话并显示历史
goose session --resume --fork --history

# 启动时附加扩展
goose session --with-extension "npx -y @modelcontextprotocol/server-memory"
goose session --with-builtin developer
goose session --with-streamable-http-extension "http://localhost:8080/mcp"

# 混合使用多种扩展
goose session \
  --with-extension "echo hello" \
  --with-streamable-http-extension "http://localhost:8080/mcp" \
  --with-builtin "developer"

# 控制会话行为
goose session -n my-session --debug --max-turns 25
```

---

#### session list [options]

列出所有已保存会话。

**选项**

- **`-f, --format <format>`**：输出格式，支持 `text`、`json`。默认 `text`
- **`--ascending`**：按时间升序排列（最旧在前）
- **`-w, --working_dir <path>`**：按工作目录过滤
- **`-l, --limit <number>`**：限制返回数量

**用法**

```bash
# 默认文本格式
goose session list

# JSON 格式
goose session list --format json

# 升序排列
goose session list --ascending

# 按工作目录过滤
goose session list -w ~/projects/myapp

# 只看最近 10 个
goose session list --limit 10
```

---

#### session remove [options]

删除一个或多个已保存会话。

**选项**

- **`--session-id <session_id>`**：按 session ID 删除
- **`-n, --name <name>`**：按会话名删除
- **`-r, --regex <pattern>`**：按正则删除匹配的会话
- **`--path <path>`**：按文件路径删除（旧参数）

**用法**

```bash
# 交互式删除
goose session remove

# 按 ID 删除
goose session remove --session-id 20251108_3

# 按名称删除
goose session remove -n my-project

# 删除所有以 project- 开头的会话
goose session remove -r "project-.*"

# 删除名称里包含 migration 的会话
goose session remove -r ".*migration.*"
```

:::caution
删除会话是永久操作，无法撤销。goose 会先显示要删除的会话并要求确认。
:::

---

#### session export [options]

将会话导出为不同格式，用于备份、分享、迁移或文档整理。

**选项**

- **`--session-id <session_id>`**：按 ID 导出
- **`-n, --name <name>`**：按名称导出
- **`--path <path>`**：按文件路径导出（旧参数）
- **`-o, --output <file>`**：输出到文件，默认 stdout
- **`--format <format>`**：输出格式：`markdown`、`json`、`yaml`。默认 `markdown`

**导出格式**

- **`json`**：完整备份，保留消息历史、元数据和设置
- **`yaml`**：完整 YAML 备份
- **`markdown`**：默认格式，适合分享和整理文档

**用法**

```bash
# 交互式导出
goose session export

# 导出 JSON 备份
goose session export -n my-session --format json -o session-backup.json

# 导出可读 Markdown
goose session export -n my-session -o session.md

# 输出到 stdout
goose session export --session-id 20251108_4 --format json
goose session export -n my-session --format yaml

# 按旧路径导出
goose session export --path ./my-session.jsonl -o exported.md
```

---

#### session diagnostics [options] {#session-diagnostics-options}

为指定会话生成用于排障的 diagnostics bundle。

**选项**

- **`--session-id <session_id>`**：按 ID 生成
- **`-n, --name <name>`**：按名称生成
- **`--path <path>`**：按文件路径生成（旧参数）
- **`-o, --output <file>`**：输出 ZIP 路径，默认 `diagnostics_{session_id}.zip`

**包含内容**

- **系统信息**：应用版本、系统、架构、时间戳
- **会话数据**：该会话的完整消息历史
- **配置文件**：你的[配置文件](/zh-CN/docs/guides/config-files)（如果存在）
- **日志文件**：最近的日志

**用法**

```bash
# 按 ID 生成 diagnostics
goose session diagnostics --session-id 20251108_5

# 按名称生成
goose session diagnostics -n my-project-session

# 保存到自定义位置
goose session diagnostics --session-id 20251108_5 -o /path/to/my-diagnostics.zip

# 交互式选择
goose session diagnostics
```

:::warning 隐私提示
diagnostics bundle 会包含你的会话消息和系统信息。如果会话里包含 API key、个人信息或专有代码，请在公开分享前先检查内容。
:::

:::tip
在提 bug 前先生成 diagnostics，通常能显著加快定位速度。生成的 ZIP 可以直接附到 GitHub issue 或分享给支持人员。
:::

---

### 任务执行

#### run [options]

从指令文件或 stdin 执行任务。更详细说明见 [Running Tasks 指南](/zh-CN/docs/guides/running-tasks)。

**输入选项**

- **`-i, --instructions <FILE>`**：读取指令文件；传 `-` 表示 stdin
- **`-t, --text <TEXT>`**：直接传入文本
- **`--system <TEXT>`**：补充 system instructions
- **`--recipe <RECIPE_FILE_NAME> <OPTIONS>`**：在当前会话加载 recipe
- **`--params <KEY=VALUE>`**：传递给 recipe 的参数，可多次指定
- **`--sub-recipe <RECIPE>`**：附加子 recipe，可多次指定

**会话选项**

- **`-s, --interactive`**：执行完初始输入后继续进入交互模式
- **`-n, --name <name>`**：为这次 run 命名，例如 `daily-tasks`
- **`-r, --resume`**：恢复上一次 run
- **`--path <PATH>`**：会话路径（旧文件存储方式）
- **`--container <container_id>`**：在 [Docker container](/zh-CN/docs/tutorials/goose-in-docker#running-extensions-in-docker-containers) 中运行扩展
- **`--no-session`**：不创建也不保存 session

**扩展选项**

- **`--with-extension <COMMAND>`**：添加 stdio 扩展，可多次指定
- **`--with-streamable-http-extension <URL>`**：添加远程 Streamable HTTP 扩展，可多次指定
- **`--with-builtin <name>`**：添加内置扩展，例如 `developer` 或 `developer,github`

**控制选项**

- **`--debug`**：输出完整 tool 响应、参数值和路径
- **`--max-tool-repetitions <NUMBER>`**：限制相同工具+相同参数的连续调用次数，防止死循环
- **`--max-turns <NUMBER>`**：无需用户输入即可运行的最大轮数，默认 `1000`
- **`--explain`**：显示 recipe 标题、说明和参数
- **`--render-recipe`**：只打印渲染后的 recipe，不执行
- **`-q, --quiet`**：静默模式，仅将模型响应打印到 stdout
- **`--output-format <FORMAT>`**：输出格式：`text`、`json`、`stream-json`。默认 `text`
- **`--provider`**：本次会话使用的 provider，覆盖环境变量
- **`--model`**：本次会话使用的模型，覆盖环境变量

**用法**

```bash
# 从文件运行
goose run --instructions plan.md

# 加载 recipe，执行后退出
goose run --recipe recipe.yaml

# 加载 recipe，执行后继续交互
goose run --recipe recipe.yaml --interactive

# 以 debug 模式运行 recipe
goose run --recipe recipe.yaml --debug

# 查看 recipe 详情
goose run --recipe recipe.yaml --explain

# 为 recipe 传参
goose run --recipe recipe.yaml --params environment=production --params region=us-west-2

# 不保存 session
goose run --no-session -i instructions.txt

# 指定 provider 和模型
goose run --provider anthropic --model claude-4-sonnet -t "initial prompt"

# 限制自动轮数
goose run --recipe recipe.yaml --max-turns 10
```

---

#### bench

用于评估系统配置在一系列实际任务中的表现。详细说明见 [Benchmarking 教程](/zh-CN/docs/tutorials/benchmarking)。

**用法**

```bash
goose bench ...etc.
```

---

#### recipe

用于校验 recipe 文件、管理 recipe 分享、列出可用 recipe，以及在 goose Desktop 中打开 recipe。

**子命令**

- **`deeplink <RECIPE_NAME>`**：为 recipe 生成可分享链接
  - **`-p, --param <KEY=VALUE>`**：预填参数，可多次指定
- **`list [OPTIONS]`**：列出本地目录和 GitHub 仓库中的 recipe
  - **`--format <FORMAT>`**：`text` 或 `json`
  - **`-v, --verbose`**：显示标题和完整文件路径
- **`open <RECIPE_NAME>`**：在 goose Desktop 中直接打开 recipe
  - **`-p, --param <KEY=VALUE>`**：预填参数，可多次指定
- **`validate <RECIPE_NAME>`**：校验 recipe 文件

**用法**

```bash
# 生成分享链接
goose recipe deeplink my-recipe.yaml

# 生成 deeplink 并预填参数
goose recipe deeplink my-recipe.yaml -p environment=production -p region=us-west-2

# 列出所有 recipe
goose recipe list

# 显示详细信息
goose recipe list --verbose

# JSON 输出
goose recipe list --format json

# 在 Desktop 中打开
goose recipe open my-recipe.yaml
goose recipe open my-recipe

# 打开时预填参数
goose recipe open my-recipe --param name=myproject

# 校验 recipe
goose recipe validate my-recipe.yaml

# 查看帮助
goose recipe help
```

---

#### schedule

按[定时计划](/zh-CN/docs/guides/recipes/session-recipes#schedule-recipe)自动运行 recipe。

**子命令**

- `add <OPTIONS>`：创建新的定时任务，并把 recipe 当前版本复制到 `~/.local/share/goose/scheduled_recipes`
- `list`：查看所有定时任务
- `remove`：删除定时任务
- `sessions`：查看某个定时 recipe 产生的会话
- `run-now`：立即执行一次定时任务
- `cron-help`：显示 cron 表达式帮助

**选项**

- `--schedule-id <NAME>`：定时任务唯一 ID，例如 `daily-report`
- `--cron "* * * * * *"`：任务执行时间，使用 [cron expression](https://en.wikipedia.org/wiki/Cron#Cron_expression)
- `--recipe-source <PATH>`：recipe YAML 路径
- `-l, --limit <NUMBER>`：`sessions` 子命令返回的最大会话数量

**用法**

```bash
goose schedule <COMMAND>

# 新增一个每天上午 9 点运行的任务
goose schedule add --schedule-id daily-report --cron "0 0 9 * * *" --recipe-source ./recipes/daily-report.yaml

# 列出所有任务
goose schedule list

# 查看最近 10 个会话
goose schedule sessions --schedule-id daily-report -l 10

# 立即运行一次
goose schedule run-now --schedule-id daily-report

# 删除任务
goose schedule remove --schedule-id daily-report
```

---

#### mcp

运行一个已启用的 MCP server，参数为 `<name>`，例如 `'Google Drive'`。

**用法**

```bash
goose mcp <name>
```

---

#### acp

以 Agent Client Protocol (ACP) agent server 方式通过 stdio 运行 goose。这样 ACP-compatible client，例如 Zed，就可以把 goose 当作 agent 使用。

ACP 是一套仍在发展的协议，目标是标准化 AI agent 与客户端应用之间的通信方式。

**用法**

```bash
goose acp
```

:::info
这个命令通常由 ACP 客户端自动调用，普通用户很少直接手动运行。具体工作流见 [Using goose in ACP Clients](/zh-CN/docs/guides/acp-clients)。
:::

---

### 项目管理

#### project

继续处理最近一个项目，或创建新项目。详细工作流见 [Managing Projects Guide](/zh-CN/docs/guides/managing-projects)。

**别名**：`p`

**用法**

```bash
goose project
```

---

#### projects

从已有项目中选择一个开始工作。

**别名**：`ps`

**用法**

```bash
goose projects
```

---

### 终端集成

#### @goose / @g

直接在 shell prompt 中向 goose 提问，并自动把命令历史带入上下文。这两个 alias 会在你完成[终端集成](/zh-CN/docs/guides/terminal-integration)后创建。

**示例**

```bash
# 在带命令历史上下文的情况下提问
@goose create a python script to process these files
@goose create a PR description summarizing these changes
@g how do I fix these permission denied errors?
```

---

## 交互式会话特性

### Slash Commands {#slash-commands}

进入交互式会话后（通过 `goose session` 或 `goose run --interactive`），你可以使用这些 slash commands。所有命令都支持 Tab 补全；按 `/` 再按 `Tab` 可以轮换可用命令。

**可用命令**

- **`/?` 或 `/help`**：显示帮助
- **`/builtin <names>`**：按名称添加内置扩展，多个用逗号分隔
- **`/clear`**：清空当前会话历史
- **`/endplan`**：退出 plan mode，返回普通模式
- **`/exit` 或 `/quit`**：退出会话
- **`/extension <command>`**：添加 stdio 扩展，格式为 `ENV1=val1 command args...`
- **`/mode <name>`**：切换 goose 模式，支持 `auto`、`approve`、`chat`、`smart_approve`
- **`/plan <message_text>`**：进入 plan mode，并基于当前上下文生成计划
- **`/prompt <n> [--info] [key=value...]`**：查看 prompt 信息或执行 prompt
- **`/prompts [--extension <name>]`**：列出可用 prompts，可按扩展过滤
- **`/recipe [filepath]`**：根据当前会话生成 recipe 并保存到指定路径；不传则保存为 `./recipe.yaml`
- **`/compact`**：压缩并总结当前会话，减少上下文长度
- **`/r`**：切换完整 tool 输出显示
- **`/t`**：在 `light`、`dark`、`ansi` 之间切换主题
- **`/t <name>`**：直接指定主题

**示例**

```bash
# 为排查测试失败先做计划
/plan let's create a plan for triaging test failures

# 查看 developer 扩展的 prompts
/prompts --extension developer

# 切换到 chat 模式
/mode chat

# 在会话中启用 developer
/builtin developer

# 清空当前对话
/clear
```

你也可以在 goose Desktop 或 CLI 中创建[自定义 slash commands 来运行 recipes](/zh-CN/docs/guides/context-engineering/slash-commands)。

---

### 主题 {#themes}

`/t` 用于控制 goose CLI 响应中 Markdown 的语法高亮主题，包括标题、代码块、加粗和斜体等样式。

**命令**

- `/t`：按 `light → dark → ansi → light` 循环切换
- `/t light`：切换到 `light`
- `/t dark`：切换到 `dark`
- `/t ansi`：切换到 `ansi`

**配置**

- 默认主题是 `dark`
- 主题会保存到[配置文件](/zh-CN/docs/guides/config-files)里的 `GOOSE_CLI_THEME`
- 当前会话也可通过 `GOOSE_CLI_THEME` [环境变量](/zh-CN/docs/guides/environment-variables#session-management) 覆盖

**自定义语法高亮**

你还可以设置底层代码块高亮主题：

- `GOOSE_CLI_LIGHT_THEME`：light 模式使用，默认 `"GitHub"`
- `GOOSE_CLI_DARK_THEME`：dark 模式使用，默认 `"zenburn"`

这两个变量接受任意 [bat theme 名称](https://github.com/sharkdp/bat#adding-new-themes)。常见选择包括 `"Dracula"`、`"Nord"`、`"Solarized (light)"`、`"Solarized (dark)"`、`"OneHalfDark"`、`"Monokai Extended"`。可通过 `bat --list-themes` 查看本机可用主题。

:::info
语法高亮主题只影响字体和 Markdown 渲染，不会改变整个终端 UI。`light` 和 `dark` 的差异相对比较细。

goose CLI 的主题与 goose Desktop 主题彼此独立。
:::

**示例**

```bash
# 通过环境变量设置 ANSI 主题
export GOOSE_CLI_THEME=ansi
goose session --name use-custom-theme

# 在会话中切换主题
/t

# 直接切到 light
/t light
```

---

## 导航与控制

### 键盘快捷键 {#keyboard-shortcuts}

**会话控制**

- **`Ctrl+C`**：当前行有内容时清空输入；模型处理中时中断请求；空行时退出会话
- **`Ctrl+J`**：插入换行。可通过 `GOOSE_CLI_NEWLINE_KEY` 在[配置文件](/zh-CN/docs/guides/config-files)或[环境变量](/zh-CN/docs/guides/environment-variables#session-management)中自定义，例如 `GOOSE_CLI_NEWLINE_KEY: n`。避免使用 `"c"` 以及常见终端快捷键 `"r"`、`"w"`、`"z"`

**导航**

- **`Cmd+Up/Down arrows`**：浏览命令历史
- **`Ctrl+R`**：交互式命令历史搜索，见[下文](#command-history-search)

---

### 外部编辑器模式 {#external-editor-mode}

如果你需要写长 prompt 或复杂代码片段，可以让 goose 调用你喜欢的文本编辑器，而不是直接在 CLI 中输入。启用后，该会话的输入方式会整体切换为编辑器模式。

**工作方式**

1. goose 打开你配置好的编辑器，并加载模板文件
2. 在 `# Your prompt:` 标题后输入内容；下方会附带历史上下文
3. 保存并关闭编辑器，goose 就会把这段内容作为下一条消息发送
4. goose 响应完成后，会再次打开编辑器，并附上新的上下文
5. 重复第 2 到第 4 步

你可以使用任何接受文件路径参数的编辑器，例如 vim、nano、emacs、VS Code。

**配置**

<Tabs>
  <TabItem value="envvar" label="Environment Variable" default>

  仅对当前会话生效。

  ```bash
  # 终端编辑器
  export GOOSE_PROMPT_EDITOR=vim

  # GUI 编辑器（需要 --wait）
  export GOOSE_PROMPT_EDITOR="code --wait"
  ```

  </TabItem>
  <TabItem value="config" label="Config File">

  持久化到所有会话，除非被环境变量覆盖。

  1. 打开 goose 的[配置文件](/zh-CN/docs/guides/config-files)，例如 macOS 下的 `~/.config/goose/config.yaml`
  2. 添加 `GOOSE_PROMPT_EDITOR`：

  ```yaml
  # 终端编辑器
  GOOSE_PROMPT_EDITOR: vim

  # GUI 编辑器（需要 --wait）
  GOOSE_PROMPT_EDITOR: code --wait
  ```

  </TabItem>
</Tabs>

**GUI 编辑器注意事项**

GUI 编辑器通常都需要 `--wait` 或等效参数，否则编辑器虽然会打开，但 goose 会立即继续执行，认为你已经编辑完成。vim、nano 这类终端编辑器不需要这个参数。

---

### 命令历史搜索 {#command-history-search}

`Ctrl+R` 可以在已保存的 CLI [命令历史](/zh-CN/docs/guides/logs#command-history)中进行交互式搜索，方便复用长命令。

**工作方式**

1. 在 goose CLI 会话中按 `Ctrl+R`
2. 输入搜索词
3. 使用以下方式浏览结果：
   - `Ctrl+R`：向更早的匹配继续搜索
   - `Ctrl+S`：向更新的匹配前进
4. 按 `Return` / `Enter` 运行命中命令，或按 `Esc` 取消

例如，你不需要重新输入这条长命令：

```
analyze the performance issues in the sales database queries and suggest optimizations
```

你只要搜索 `"sales database"` 或 `"optimization"` 就可以快速找到并重跑它。

**搜索建议**

- **优先使用有区分度的词**：更容易缩小结果范围
- **支持部分匹配和多个单词**：例如 `"gith"`、`"run the unit test"`
