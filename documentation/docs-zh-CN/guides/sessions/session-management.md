---
title: "会话管理"
description: "介绍如何开始、恢复、搜索和管理 goose 会话。"
sidebar_position: 1
sidebar_label: "会话管理"
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { AppWindow, PanelLeft, FolderDot, Copy, Edit2, Trash2, Download, Upload, ChefHat } from 'lucide-react';

会话（session）是你与 goose 之间一次连续的交互，用来提问、下达指令并推进任务。本指南会说明整个会话生命周期该如何管理。

## 启动会话 {#start-session}

:::info 首次设置
在你的第一个会话中，goose 会提示你先[配置 LLM（Large Language Model）provider](../../getting-started/installation.md#set-llm-provider)。
:::

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        打开 goose 后，你会直接看到可用的会话界面。只要把问题、请求或指令直接输入到输入框里，或者[通过语音输入](./in-session-actions.md#voice-dictation)，goose 就会立刻开始处理。

        要开始一个新的聊天会话：

        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
        2. 点击侧边栏中的 `Home` 或 `Chat`
        3. 在聊天框中发送你的第一条 prompt

        goose Desktop 支持在同一个窗口里同时保留多个活跃会话。新会话会出现在侧边栏 `Chat` 区域中，你可以快速切换最近的 10 个会话。只要点击某个会话，就能在该会话里[继续工作](#resume-session)。
        
        如果要切换工作目录，点击应用底部的 <FolderDot className="inline" size={16} /> 目录切换器即可。

        :::info 在新窗口中启动会话
        如果想在新窗口中开始会话，点击左上角的 <AppWindow className="inline" size={16} /> 按钮。发送第一条 prompt 后，新会话就会出现在侧边栏的 `Chat` 区域里。

        在 macOS 上，你也可以通过 Dock 图标快速开启会话：
            - **拖拽目录** 到 goose 图标上，在该目录中打开一个新会话
            - **右键** 点击 goose 图标，选择 `New Window`，在你最近使用的目录中打开新会话
        :::
        
        #### 键盘快捷键
        
        你也可以使用快捷键来新建会话或管理 goose 窗口。
        
        | 操作 | macOS | Windows/Linux |
        |--------|-------|---------------|
        | 使用 Quick Launcher 新建会话 | `Cmd+Option+Shift+G` | `Ctrl+Alt+Shift+G` |
        | 在当前目录中新建会话 | `Cmd+N` | `Ctrl+N` |
        | 在当前目录中新建会话（同一窗口） | `Cmd+T` | `Ctrl+T` |
        | 在其他目录中新建会话 | `Cmd+O` | `Ctrl+O` |
        | 切换侧边栏显示 | `Cmd+B` | `Ctrl+B` |
        | 打开设置 | `Cmd+,` | `Ctrl+,` |
        | 聚焦 goose 窗口 | `Cmd+Option+G` | `Ctrl+Alt+G` |
        | 让 goose 窗口始终置顶 | `Cmd+Shift+T` | `Ctrl+Shift+T` |
        
        #### 自定义快捷键
        
        你可以在 **Settings** 菜单中自定义这些快捷键：
        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
        2. 点击 `Settings`
        3. 点击 `Keyboard` 标签
        
        对全局快捷键（例如 Focus Window、Quick Launcher）的修改会立即生效。对应用内快捷键（如 New Chat、Settings）的修改，则需要重启 goose 才会生效。
        
        #### Quick Launcher
        你可以在弹窗中直接输入 prompt 来开始一个新会话：
        1. 按下 `Cmd+Option+Shift+G`（macOS）或 `Ctrl+Alt+Shift+G`（Windows/Linux）打开弹窗
        2. 输入你的 prompt，然后按 `Enter`

        这个会话会在一个新的 goose 窗口中打开，并使用你最近一次打开的目录。

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        在终端中先切换到你希望开始工作的目录，然后运行 [session](/zh-CN/docs/guides/goose-cli-commands#session-options) 命令：
        ```sh
        goose session
        ```

    </TabItem>
</Tabs>

## 命名会话 {#name-session}
<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        会话的显示名称会根据你的第一条 prompt 上下文自动生成。合理的会话名可以帮助你在多个活跃会话之间切换，或者后续[恢复会话](#resume-session)。

        会话创建之后，你仍然可以修改名称：

        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
        2. 点击 `Chat` 区域底部的 `View All`
        3. 将鼠标悬停到你想重命名的会话上
        4. 点击会话卡片上出现的 <Edit2 className="inline" size={16} /> 按钮
        5. 在弹出的 “Edit Session Description” 窗口中：
           - 输入新的会话描述（最多 200 个字符）
           - 按 `Enter` 保存，或按 `Escape` 取消
           - 也可以点击 `Save` 或 `Cancel`
        6. 成功后会看到对应的 toast 提示

        会话名称会显示在侧边栏的 `Chat` 区域、`Window` 菜单，以及 macOS 的 Dock 菜单或 Windows 的任务栏菜单中。

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        goose 会基于你第一条 prompt 的上下文自动生成会话名称。

        如果你想在启动会话时显式指定名称，可以直接在命令里传入。例如把会话命名为 `react-migration`：

        ```sh
        goose session --name react-migration
        ```

        如果你想确认这个会话名，可以运行：

        ```sh
        goose session list -l 1
        ```

        示例输出：
        
        ```text
        Available sessions:
        20260213_9 - react-migration - 2026-02-13 16:20:37 UTC
        ```

上面的 `20260213_9` 是 session ID。session ID 的格式为 `YYYYMMDD_<COUNT>`。很多 [goose CLI 命令](/zh-CN/docs/guides/goose-cli-commands) 都支持通过名称（`--name` / `-n`）来定位会话，作为 `--session-id` 的替代方式。
    </TabItem>
</Tabs>

:::tip 关闭 AI 自动命名
如果你想保留默认名称，而不是调用模型生成会话名，可以使用 [`GOOSE_DISABLE_SESSION_NAMING`](/zh-CN/docs/guides/environment-variables#session-management)。在 goose Desktop 中默认名称是 “New Chat”，在 goose CLI 中默认名称是 “CLI Session”。
:::

## 退出会话 {#exit-session}
退出时会话会自动保存。
<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
    要退出会话，直接关闭应用即可。
    </TabItem>    
    <TabItem value="cli" label="goose CLI">
        要退出会话，可以输入 `exit`。你也可以按下 `Ctrl+C` 退出。

        会话会保存在 goose 的[本地 SQLite 数据库](/zh-CN/docs/guides/logs#session-records)中。
    </TabItem>
</Tabs>

## 搜索会话 {#search-sessions}

搜索可以帮助你查找会话中的具体内容，也可以帮助你定位某个会话。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

    你可以通过快捷键和搜索栏按钮，在 goose Desktop 中搜索会话。

    | 操作 | macOS | Windows/Linux |
    |--------|-------|---------------|
    | 打开搜索 | `Cmd+F`  | `Ctrl+F`  |
    | 下一个匹配项 | `Cmd+G` 或 `↓` | `Ctrl+G` 或 `↓` |
    | 上一个匹配项 | `Shift+Cmd+G` 或 `↑` | `Shift+Ctrl+G` 或 `↑` |
    | 用当前选中内容搜索 | `Cmd+E` | n/a |
    | 切换大小写敏感 | `Aa` | `Aa` |
    | 关闭搜索 | `Esc` 或 `X` | `Esc` 或 `X` |

    :::tip 自定义搜索快捷键
    你可以在 **Settings** → **Keyboard** 中自定义 Find、Find Next 和 Find Previous 的快捷键。
    :::

    :::info 不支持正则或搜索操作符
    搜索框不支持正则表达式或高级搜索操作符。
    :::

    支持以下两种场景：

    #### 在当前会话中搜索
    
    如果你想在当前会话里查找某段内容：

    1. 使用 `Cmd+F` 打开搜索栏
    2. 输入搜索关键词
    3. 使用快捷键或搜索栏按钮浏览匹配结果

    #### 跨会话搜索
    
    如果你想在所有会话历史中搜索消息内容：

    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
    2. 点击 `Chat` 区域底部的 `View All`
    3. 使用 `Cmd+F`（或 `Ctrl+F`）打开搜索栏
    4. 输入搜索关键词
    5. 使用快捷键和搜索栏按钮浏览结果（此处不支持 `Cmd+E`）

    这里搜索的是对话中的消息内容。搜索结果最多只会返回最近的 10 条匹配消息。如果某个关键词命中很多消息，最终返回的会话只会是其中一部分。

    :::tip 直接问 goose
    你也可以使用内置的 [Chat Recall 扩展](../../mcp/chatrecall-mcp.md)，直接让 goose 搜索你的对话历史：
    - “Find my earlier conversation about React hooks from last week”
    - “Show me sessions where I worked on database migrations”
    :::

  </TabItem>
  <TabItem value="cli" label="goose CLI">

    #### 在当前会话中搜索

    搜索能力由你的终端界面提供。常见终端环境对应的快捷键如下：

    | Terminal | Operating System | Shortcut |
    |----------|-----------------|-----------|
    | iTerm2 | macOS | `Cmd+F` |
    | Terminal.app | macOS | `Cmd+F` |
    | Windows Terminal | Windows | `Ctrl+F` |
    | Linux Terminal | Linux | `Ctrl+F` |

    如果你想在当前会话里搜索：

    1. 使用对应快捷键打开搜索栏
    2. 输入搜索关键词
    3. 使用快捷键或搜索栏按钮浏览结果

    :::info
    你的具体终端模拟器可能使用不同的快捷键。请以终端自身的文档或设置为准。
    :::

    #### 跨会话搜索所有内容
    
    如果你想搜索所有会话中的对话内容，可以先启动一个 goose 会话，然后直接问：

    - “Find my earlier conversation about React hooks from last week”
    - “Show me sessions where I worked on database migrations”

    goose 会搜索你的会话历史，并展示命中的相关上下文。
    
    :::info
    这项功能需要启用内置的 [Chatrecall 扩展](../../mcp/chatrecall-mcp.md)。
    :::

    #### 直接搜索会话数据
    
    带特定参数的 [`session list`](/zh-CN/docs/guides/goose-cli-commands#session-list-options) 子命令，也可以帮助你完成一部分搜索需求。

    你还可以直接查询 SQLite 数据库：

    ```bash
    # 搜索会话描述
    sqlite3 ~/.local/share/goose/sessions/sessions.db \
      "SELECT id, description FROM sessions WHERE description LIKE '%your search term%';"

    # 按工作目录搜索
    sqlite3 ~/.local/share/goose/sessions/sessions.db \
      "SELECT id, description, working_dir FROM sessions WHERE working_dir LIKE '%project-name%';"

    # 列出最近会话
    sqlite3 ~/.local/share/goose/sessions/sessions.db \
      "SELECT id, description, created_at FROM sessions ORDER BY created_at DESC LIMIT 10;"
    ```

    :::info 会话存储迁移
    从 `1.10.0` 开始，goose 使用 SQLite 数据库（`sessions.db`）来保存会话，而不是单独的 `.jsonl` 文件。旧的 `.jsonl` 文件仍会保留在磁盘上，但 goose 不再管理它们。
    :::

  </TabItem>
</Tabs>

## 恢复会话 {#resume-session}

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
    
    你既可以在侧边栏切换活跃会话，也可以从历史记录中恢复任意会话。
    
    #### 切换活跃会话

    goose Desktop 支持在同一窗口中来回切换多个聊天会话。你可以在一个会话中启动任务，切到另一个会话做别的事情，再回到原会话继续推进。

    最近的会话（最多 10 个）会显示在侧边栏中，方便快速切换：

    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
    2. 在 `Chat` 区域点击任意会话即可切换
    
    你可以通过以下视觉标记判断会话状态：
    - **蓝色旋转图标**：会话正在处理请求
    - **绿色圆点**：你切到别的会话时，该会话完成了任务
    - **红色圆点**：该会话发生错误

    此外，如果某个会话是从 recipe 启动的，会显示 <ChefHat className="inline" size={16} /> 图标。

    :::tip
    你可以先[重命名会话](#name-session)，这样更容易识别。
    :::

    #### 从历史记录恢复会话

    如果目标会话不在最近的 10 个之内：

    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
    2. 点击 `Chat` 区域底部的 `View All`
    3. 找到你要恢复的会话。你可以使用上面的[搜索能力](#search-sessions)更快定位它。
    4. 选择恢复方式：
       - 点击 `Resume` 在当前窗口继续
       - 点击 `New Window` 在新窗口中打开

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        要恢复最近一次会话，可以运行：

        ```
         goose session -r
        ```

        要恢复指定会话，可以这样运行：

        ```
        goose session -r --name <name>
        ```
        例如，要恢复名为 `react-migration` 的会话：

        ```
        goose session -r --name react-migration
        ```
    </TabItem>
</Tabs>

Desktop 中创建的会话也可以在 CLI 中恢复，反之亦然。所有会话都保存在[同一个数据库](/zh-CN/docs/guides/logs#session-records)里。

:::tip 新任务建议开新会话
虽然你可以恢复旧会话，但对于新任务，更建议直接新建会话，以减少进入 [doom spiraling](/zh-CN/docs/troubleshooting/known-issues#stuck-in-a-loop-or-unresponsive) 的概率。
:::

### 恢复项目型会话 {#resume-project-based-sessions}

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        项目型会话目前只支持通过 CLI 使用。
    </TabItem>
    <TabItem value="cli" label="goose CLI">
        你可以使用 [`project`](/zh-CN/docs/guides/goose-cli-commands#project) 和 [`projects`](/zh-CN/docs/guides/goose-cli-commands#projects) 命令，从一个 project 启动或恢复会话。project 本质上是带有会话元数据的受追踪工作目录。完整说明请参考[管理项目](../managing-projects.md)。
    </TabItem>
</Tabs>

## 复制会话 {#duplicate-sessions}

如果你想复用配置、尝试不同方案或保留某次工作结果，可以创建某个会话的完整副本。

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        
        在会话列表中复制一个会话：

        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
        2. 点击 `Chat` 区域底部的 `View All`
        3. 找到你想复制的会话
        4. 将鼠标悬停到会话卡片上，显示操作按钮
        5. 点击右上角出现的 <Copy className="inline" size={16} /> 按钮

        复制出的会话会包含：
        - 完整对话历史
        - 所有会话元数据和设置
        - Provider 和模型配置
        - 扩展数据与配置
        - Recipe 信息（如果有）

        新会话会继承原会话名称，并显示在你的会话列表顶部。

        :::tip Duplicate 与 Fork Session 的区别
        - **Duplicate**（会话列表中的 Copy 按钮）：复制整个会话，适合保留某个工作版本或复用配置。
        - **[Fork Session](./in-session-actions.md#fork-session)**（消息上的 Edit 按钮）：从某条被编辑的消息位置切出一个新会话，适合在某个历史节点尝试不同方向。
        :::

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        
        你可以通过 `--fork` 配合 `--resume`，基于旧会话复制出一个新会话。

        ```bash
        # Fork 最近一次会话
        goose session --resume --fork

        # 按会话名 fork
        goose session --resume --fork --name my-project

        # 按会话 ID fork
        goose session --resume --fork --session-id 20251108_3

        # Fork 并显示消息历史
        goose session --resume --fork --history
        ```

        Fork 出来的会话会包含：
        - 原会话的完整对话历史
        - 所有会话元数据和设置
        - Provider 和模型配置
        - 扩展数据与配置
        - Recipe 信息（如果适用）
    </TabItem>
</Tabs>

## 删除会话 {#delete-sessions}

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        你可以直接在 Desktop 应用中删除会话：

        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
        2. 点击 `Chat` 区域底部的 `View All`
        3. 找到你想删除的会话
        4. 将鼠标悬停到会话卡片上，显示操作按钮
        5. 点击出现的 <Trash2 className="inline" size={16} /> 按钮
        6. 在弹窗中确认删除

        :::warning 永久删除
        在 goose Desktop 中删除会话，也会同步从 CLI 中删除。这个操作不可撤销。
        :::

        会话会立刻从历史记录中消失，底层的本地会话记录也会被一并删除。
    </TabItem>
    <TabItem value="cli" label="goose CLI">
        你可以通过 CLI 命令删除会话。具体参数请参考 [CLI Commands 文档](/zh-CN/docs/guides/goose-cli-commands#session-remove-options)。
    </TabItem>
</Tabs>

## 导入会话 {#import-sessions}

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        你可以从 JSON 文件导入完整会话，用于恢复、分享或在不同 goose 实例之间迁移。导入时会创建一个新的会话 ID，而不是覆盖现有会话。

        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
        2. 点击 `Chat` 区域底部的 `View All`
        3. 点击右上角的 <Upload className="inline" size={16} /> `Import Session` 按钮
        4. 选择一个之前从 goose 导出的 `.json` 会话文件
        5. 该会话会以新的 session ID 导入
        6. 成功后会收到通知

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        目前只有 Desktop 应用支持导入会话。
    </TabItem>
</Tabs>

## 导出会话 {#export-sessions}

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        你可以把完整会话导出成 JSON 文件，用于备份、分享、迁移或归档。导出文件会保留完整对话、元数据和设置。

        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
        2. 点击 `Chat` 区域底部的 `View All`
        3. 找到你想导出的会话
        4. 将鼠标悬停到会话卡片上，显示操作按钮
        5. 点击出现的 <Download className="inline" size={16} /> 按钮
        6. 会话会被下载成一个 `.json` 文件，文件名使用该会话的描述

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        你也可以通过 CLI 导出会话，用于备份、迁移或生成文档。CLI 支持导出成 JSON，保留完整会话数据，也支持导出成 Markdown，得到更适合阅读的格式化对话记录。

        在终端中运行 [`session export`](/zh-CN/docs/guides/goose-cli-commands#session-export-options) 子命令：
        
        ```bash
        goose session export
        ```

    </TabItem>
</Tabs>
