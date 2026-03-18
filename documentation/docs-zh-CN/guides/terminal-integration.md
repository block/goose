# 终端集成

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

你可以直接在 shell prompt 里和 goose 对话，而不用切到单独的 REPL 会话。保持在终端里工作，需要时直接调用 goose 即可。

## 设置

<Tabs groupId="shells">
<TabItem value="zsh" label="zsh" default>

把下面这行加入 `~/.zshrc`：
```bash
eval "$(goose term init zsh)"
```

</TabItem>
<TabItem value="bash" label="bash">

把下面这行加入 `~/.bashrc`：
```bash
eval "$(goose term init bash)"
```

</TabItem>
<TabItem value="fish" label="fish">

把下面这行加入 `~/.config/fish/config.fish`：
```fish
goose term init fish | source
```

</TabItem>
<TabItem value="powershell" label="PowerShell">

把下面这行加入 `$PROFILE`：
```powershell
Invoke-Expression (goose term init powershell)
```

</TabItem>
</Tabs>

重启终端，或者重新 source 配置文件，就完成了。

## 使用方式

直接输入 `@goose`（也可以简写成 `@g`），后面跟上你的问题：

```bash
npm install express
    npm ERR! code EACCES
    npm ERR! permission denied

@goose "how do I fix this error?"
```

goose 会自动看到你从上次提问以来执行过的命令，所以你不需要重新解释刚才做了什么。如果 prompt 里包含 `?`、`*`、`'` 这类特殊字符，请用引号包起来：

```bash
@goose "what's in this directory?"
@g "analyze the error: 'permission denied'"
```

## 命名会话
默认情况下，每个终端都会有一个自己的 goose 会话，直到你关闭终端为止。命名会话允许你在终端重启后继续同一段对话，也能在不同窗口之间共享上下文。

<Tabs groupId="shells">
<TabItem value="zsh" label="zsh" default>

```bash
eval "$(goose term init zsh --name my-project)"
```

</TabItem>
<TabItem value="bash" label="bash">

```bash
eval "$(goose term init bash --name my-project)"
```

</TabItem>
<TabItem value="fish" label="fish">

```fish
goose term init fish --name my-project | source
```

</TabItem>
<TabItem value="powershell" label="PowerShell">

```powershell
Invoke-Expression (goose term init powershell --name my-project)
```

</TabItem>
</Tabs>

命名会话会持久化保存在 goose 的数据库里，因此即使你重启电脑，之后也能继续使用。重新打开终端，再执行同样的命令即可续上：

```bash
# Start debugging
eval "$(goose term init zsh --name auth-bug)"
@goose help me debug this login timeout

# Close terminal, come back later
eval "$(goose term init zsh --name auth-bug)"
@goose "what was the solution we discussed?"
# Continues the same conversation with context
```

## 在 Prompt 中显示上下文状态

把 `goose term info` 加到你的 prompt 里，就可以在终端 goose 会话中看到当前上下文使用量和激活模型。

<Tabs groupId="shells">
<TabItem value="zsh" label="zsh" default>

```bash
PROMPT='$(goose term info) %~ $ '
```

</TabItem>
<TabItem value="bash" label="bash">

```bash
PS1='$(goose term info) \w $ '
```

</TabItem>
<TabItem value="fish" label="fish">

```fish
function fish_prompt
    goose term info
    echo -n ' '(prompt_pwd)' $ '
end
```

</TabItem>
<TabItem value="powershell" label="PowerShell">

```powershell
function prompt {
    $gooseInfo = & goose term info
    "$gooseInfo $(Get-Location) PS> "
}
```

</TabItem>
</Tabs>

配置后，你的终端 prompt 会显示当前会话的上下文使用量，以及当前模型名（为便于阅读会做缩写）。例如：

```bash
●●○○○ sonnet ~/projects $
```
## goose 命令的 Shell Completion

`@goose` 会根据你的命令历史提供上下文感知辅助。如果你还想给 goose CLI 命令（例如 `goose session`、`goose run` 等）启用 tab completion，请参考 [shell completion 文档](/zh-CN/docs/guides/goose-cli-commands#completion)。

## 故障排查

**goose 看不到最近执行的命令：**  
如果你已经执行了一些命令，但 goose 说它看不到近期活动，请先检查终端集成是否已经按[设置方式](#设置)正确写入 shell 配置。
你也可以检查当前终端里的 goose session ID：
```bash
# Check if session ID exists
echo $AGENT_SESSION_ID
# Should show something like: 20251209_151730
```
如果你需要在多个终端窗口之间共享上下文，请改用[命名会话](#命名会话)。

**会话太满了**（prompt 显示 `●●●●●`）：  
如果 goose 的响应变慢，或者频繁触发上下文限制，建议在当前终端里启动一个新的 goose 会话。新的会话能看到你的命令历史，但不会继承上一段对话历史。
```bash
# Start a new goose session in the same shell
eval "$(goose term init zsh)"
```
