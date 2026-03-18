---
title: "运行任务"
description: "介绍如何在 goose 中运行和管理任务。"
sidebar_position: 75
---

# 运行任务

在 goose CLI 中，你可以通过 `goose run` 命令把文件和指令传给 goose，让它执行单次任务或一整套工作流。任务既可以是简短的一行命令，也可以是一份保存在文件中的复杂说明。

## 基本用法

`goose run` 会启动一个新会话，执行你传入的参数，并在任务完成后自动退出会话。

运行任务有很多方式，完整选项请查看 [CLI 命令指南](/zh-CN/docs/guides/goose-cli-commands#run-options)。

### 在命令里直接写文本

```bash
goose run -t "your instructions here"
```

通过 `-t` 参数，你可以直接把文本指令作为命令参数传入。这非常适合一次性的小任务，不需要进入交互式会话。指令执行完后，会话就会结束。典型场景包括 CI/CD 流水线，或者和其他脚本一起运行。

### 使用指令文件

如果你有一套更复杂的说明或自动化流程，可以把它写进文件，再交给 `goose run`：

```bash
goose run -i instructions.md
```

例如，下面这份文件会要求 goose 对项目依赖做一次安全审计：

```md
# Dependency Security Audit

1. Analyze project dependencies:
   - Check package.json and requirements.txt files
   - List all dependencies with versions
   - Identify outdated packages

2. Security check:
   - Run npm audit (for JavaScript packages)
   - Check for known vulnerabilities in Python packages
   - Identify dependencies with critical security issues

3. Create an upgrade plan:
   - List packages requiring immediate updates
   - Note breaking changes in latest versions
   - Estimate impact of required updates

Save findings in 'security_audit.md' with severity levels highlighted.
```

### 通过标准输入传入

你也可以使用 `-i -` 从标准输入把指令传给 goose。这在你想把其它工具或脚本的输出直接 pipe 给 goose 时很有用。

#### 简单 echo 管道

```bash
echo "What is 2+2?" | goose run -i -
```

#### 多行指令

```bash
cat << EOF | goose run -i -
Please help me with these tasks:
1. Calculate 15% of 85
2. Convert 32°C to Fahrenheit
EOF
```

## 关键能力

### Interactive Mode

如果你不希望 goose 在任务结束后自动退出，可以传入 `-s` 或 `--interactive`，让它在处理完初始任务后继续进入交互式会话：

```bash
goose run -i instructions.txt -s
```

这适合你先跑一段初始化任务，再继续和 goose 互动的场景。

### Session Management

你可以为会话命名并进行后续管理：

```bash
# 启动一个命名会话
goose run -n my-project -t "initial instructions"

# 恢复之前的命名会话
goose run -n my-project -r
```

如果你不希望创建或保存会话文件，可以使用 `--no-session`。这适合自动化脚本或一次性任务，因为它不会保留对话历史和状态。该选项会把会话输出重定向到临时空路径（Unix 上是 `/dev/null`，Windows 上是 `NUL`），任务结束后直接丢弃。

```bash
goose run --no-session -t "your command here"
```

### 指定 Provider 和 Model

你可以在运行时临时指定 provider 和 model，覆盖默认环境变量中的配置：

```bash
goose run --provider anthropic --model claude-4-sonnet -t "initial prompt"
```

### 搭配扩展使用

如果你希望某些扩展在任务运行时一定可用，可以用参数显式指定，例如 `--with-extension`、`--with-remote-extension`、`--with-streamable-http-extension` 或 `--with-builtin`：

- 使用内置扩展，例如 developer 和 computercontroller：

```bash
goose run --with-builtin "developer,computercontroller" -t "your instructions"
```

- 使用自定义扩展：

```bash
goose run --with-extension "ENV1=value1 custom-extension-args" -t "your instructions"
```

- 使用 streamable HTTP 扩展：

```bash
goose run --with-streamable-http-extension "https://example.com/streamable" -t "your instructions"
```

### Debug Mode

在排障或开发复杂工作流时，可以打开 `--debug` 查看更详细的工具执行信息，包括：

- 完整工具响应
- 更详细的参数值
- 完整文件路径

Debug mode 适合：

- 开发新的自动化脚本
- 排查扩展行为
- 验证工具参数和返回结果

```bash
goose run --debug -t "your instructions"
goose run --debug --recipe recipe.yaml
```

### JSON 输出格式

为了方便自动化、脚本和 CI/CD 集成，`goose run` 支持 `--output-format` 输出结构化结果：

- `json`：任务结束后输出完整 JSON，适合 CI 流水线和日志采集
- `stream-json`：执行过程中实时流式输出 JSON 事件，适合进度监控和长任务

```bash
goose run --output-format json -t "your instructions"
goose run --output-format stream-json -t "your instructions"
goose run --output-format json --recipe recipe.yaml
goose run --output-format json --no-session -t "automated task"
```

JSON 输出中通常包含：

- 会话元数据和执行结果
- 工具输出和错误信息
- 方便脚本或 CI 解析的结构化字段

## 常见用法

### 运行脚本文件

先准备一份指令文件，例如 `build-script.txt`：

```text
Check the current branch
Run the test suite
Build the documentation
```

然后运行：

```bash
goose run -i build-script.txt
```

### 快速命令

对于一次性的短任务，最适合直接使用文本参数：

```bash
goose run -t "Create a CHANGELOG.md entry comparing current git branch with main"
```

### 开发工作流

你也可以带着特定扩展启动一个命名会话：

```bash
goose run --with-builtin "developer,git" -n dev-session -s
```

### 组合多个选项

`goose run` 的强项之一，就是可以把多个参数叠起来构成一个完整工作流：

```bash
goose run \
  --with-builtin "developer,git" \
  --with-extension "API_KEY=xyz123 custom-tool" \
  -n project-setup \
  -t "Initialize project"
```

这条命令会：

1. 加载 `developer` 和 `git` 两个内置扩展
2. 额外接入一个带 API Key 的自定义扩展
3. 把会话命名为 `project-setup`
4. 用 `Initialize project` 作为初始任务
5. 在命令处理结束后自动退出
