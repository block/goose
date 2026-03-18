---
title: "会话内操作"
description: "介绍会话中可用的共享、协作和上下文操作。"
sidebar_position: 2
sidebar_label: "会话内操作"
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, Paperclip, Edit2, Send, GripVertical, X, ChevronUp, ChevronDown, FolderDot, Puzzle, Bot, Tornado } from 'lucide-react';

goose 提供了一组会话内能力，帮助你在对话过程中管理上下文、共享信息并维持协作节奏。

## 编辑消息 {#edit-message}

你可以编辑已经发送过的消息，用来微调对话、纠正方向，或者尝试不同的提问方式。

**示例消息流：**

你原来的对话有 5 条消息。若你编辑第 3 条消息，那么第 4 条和第 5 条之后的消息与响应上下文都会被删除。

```
┌─────┐   ┌─────┐   ┌─────┐   ┌─────┐   ┌─────┐
│  1  │ → │  2  │ → │  3  │ → │  4  │ → │  5  │
└─────┘   └─────┘   └─────┘   └─────┘   └─────┘
           
                   Edit here
                       ↓
┌─────┐   ┌─────┐   ┌─────┐    continue from here in
│  1  │ → │  2  │ → │  3  │ →  current session, or
└─────┘   └─────┘   └─────┘    copy to new session
```

编辑之后，你可以选择继续在当前会话中工作，或者把它分叉到新会话：

- 使用 [Edit in Place](#edit-in-place) 直接覆盖当前会话，并删除被编辑消息之后的所有上下文。这适合从某个历史点重新开始推进。
- 使用 [Fork Session](#fork-session) 创建一个新会话，新会话会保留被编辑消息及其之前的全部历史。这样你就能尝试不同路线，同时保留原会话。

### 就地编辑 {#edit-in-place}

Edit in Place 会直接覆盖被编辑消息之后的会话上下文。你既可以只是修正上一条消息里的一个路径，也可以从某个历史节点开始彻底换一个方向。

适合使用就地编辑的场景：

- 你发现自己发出去的 prompt 不够清晰或信息不完整
- goose 误解了你的意图，已经往错误方向走远了

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>

        1. 把鼠标悬停到任意一条你之前发送的消息上
        2. 点击出现的 <Edit2 className="inline" size={16} /> `Edit` 按钮
        3. 在行内编辑器中修改内容
        4. 点击 `Edit in Place` 保存修改，并重新触发 goose

        goose 会删除被编辑消息之后的所有对话历史，并从该位置继续生成后续响应。

        :::warning 会删除后续上下文
        使用 Edit in Place 时，被编辑消息之后的会话历史会被永久删除，并从 goose 的上下文中移除。只有在你确定不再需要这些上下文时，才建议这么做。
        :::

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        goose CLI 目前不支持编辑消息。
    </TabItem>
</Tabs>

### 分叉会话 {#fork-session}

Fork Session 会在保留原对话的同时，把你编辑后的消息作为起点创建一个新会话。这样你可以测试不同思路、比较不同结果，同时把原会话保留下来作为参考。

适合 fork 的场景：
- 并排比较同一个问题的不同解法
- 测试不同 prompt 对 goose 响应的影响

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        1. 将鼠标悬停到任意一条你之前发送的消息上
        2. 点击出现的 <Edit2 className="inline" size={16} /> `Edit` 按钮
        3. 在行内编辑器里修改内容
        4. 点击 `Fork Session` 保存修改并开始新会话（或使用 `Cmd+Enter` / `Ctrl+Enter`）

        goose 会创建一个新会话，其中包含被编辑消息及其之前的全部对话历史。新会话会以 “(edited)” 结尾命名，原会话保持不变。

        :::tip Fork 与 Duplicate 的区别
        - **Fork Session**（消息上的 Edit 按钮）：从某一条被编辑消息处分叉，适合在对话中的某个时间点尝试不同方案。
        - **[Duplicate Session](./session-management.md#duplicate-sessions)**（会话列表中的 Copy 按钮）：复制整个会话，适合保留一个完整工作副本或复用配置。
        :::
</TabItem>
    <TabItem value="cli" label="goose CLI">
    goose CLI 不支持编辑单条消息，但你可以使用 `--fork` 来[复制整个会话](./session-management.md#duplicate-sessions)。
    </TabItem>
</Tabs>

### 编辑场景建议

- **迭代优化 prompt**：先发一个基础 prompt，再根据 goose 的响应持续编辑优化，通常比一开始就试图写出完美 prompt 更有效。
- **什么时候编辑，什么时候中断**：如果对话已经明显跑偏，直接编辑前面的消息往往比在后面追加新消息或使用[任务中断](#interrupt-task)更有效。编辑是在重写历史；中断只会影响当前消息之后的走向。
- **保留进度**：如果当前会话已经取得了不错的进展，但你还想测试另一种方案，优先使用 Fork Session。这样即使新方向失败，你也能随时回到原会话。

## 消息队列 {#queue-messages}

当 goose 正在处理任务时，你可以提前排队后续消息。这在以下场景尤其有用：

- 你希望趁 goose 工作时先把下一步输入好
- 你手上有一连串相关任务需要依次完成
- 你正在使用[语音输入](#voice-dictation)，希望快速把思路记下来

:::tip
对于复杂任务，先拆成多个子任务再依次推进，通常会比一次性扔给 goose 一个超大 prompt 效果更好。这种方式常被称为 [*prompt chaining*](https://www.promptingguide.ai/techniques/prompt_chaining)。
:::

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
      添加消息到队列：
      1. 当 goose 正在处理响应时，先输入下一条消息
      2. 按 `Enter` 把它加入队列（如果你用了[中断关键词](#interrupt-task)，也可能直接中断当前任务）
      
      队列中的消息会以编号卡片显示，表示执行顺序。当前任务完成后，队列中的第一条消息会自动被发送。
      
      :::info 相关行为
      - 一般来说，goose 正在工作时按 `Enter` 会把消息加入队列；点击 `Send` 则会立刻发送，并[中断当前任务](#interrupt-task)
      - 如果排队消息中包含 `stop`、`wait`、`hold on` 等常见中断关键词，goose 会先暂停，等你发送下一条消息后再继续处理队列
      :::

      #### 队列管理
    
      排队消息会按顺序自动执行，但你也可以手动管理：
      - **编辑消息**：点击消息文本，显示编辑控件，修改后点击 `Save`
      - **调整顺序**：将鼠标悬停到消息卡片上，显示 <GripVertical className="inline" size={16} /> 按钮，然后拖拽调整顺序
      - **立即发送**：点击 <Send className="inline" size={16} /> 按钮，立刻发送该消息并中断当前任务
      - **删除消息**：点击 <X className="inline" size={16} /> 按钮删除消息
      - **清空队列**：点击 **Message Queue** 卡片上的 `Clear All`
      - **折叠或展开队列**：点击 **Message Queue** 卡片上的 <ChevronUp className="inline" size={16} /> 或 <ChevronDown className="inline" size={16} /> 按钮

      #### 示例消息流

      **不使用队列：**

      你一次性发送：

      “Can you refactor our authentication code to support OAuth 2.0 and add proper error handling? Also include unit tests for the OAuth flow, update the API documentation to reflect these changes, and create a migration script to help existing users transition to the new system.”

      这种方式很容易得到一个过于庞杂的响应，一些关键细节会被忽略，或者每项任务都只被浅尝辄止地处理。即使你在一个 prompt 里按顺序列步骤，也无法让 goose 真正逐步聚焦、逐步建立上下文。

      **使用队列：**

      1. 你先发送：“Refactor the authentication code to support OAuth 2.0”
      2. 在 goose 工作期间，再依次加入队列：
         - “And add proper error handling”
         - “Add unit tests for the OAuth flow”
         - “Update the API documentation”
         - “Create migration script for existing users”
      
      这样每一项任务都建立在前一步结果之上。

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        goose CLI 目前不支持消息排队。
    </TabItem>
</Tabs>

## 中断任务 {#interrupt-task}

如果 goose 正在处理任务，而你需要立刻改变方向、补充关键信息或接管当前流程，就可以中断任务。

常见适用场景：

- goose 正在走错方向
- 你突然意识到还缺少关键上下文
- 你想切换到一个完全不同的任务

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        
        有两种方式可以中断当前任务：

        #### 发送中断关键词
        1. 输入包含常见中断关键词的 prompt，例如 `stop`、`wait`、`hold on`、`actually`、`instead`
        2. 点击 `Send`
        
        goose 会停止当前任务，并等待你提供更多信息。
        
        #### 直接重定向
        1. 输入一条更明确的新消息，补充上下文或直接改变方向，例如：
           - “I forgot to mention this is for a mobile app”
           - “Let's focus on React instead of TypeScript”
        2. 点击 `Send`
        
        goose 会停止当前处理流程，并转向新的上下文继续工作。
        
        :::info 相关功能
        - goose 正在工作时，点击 `Send` 会中断当前任务；按 `Enter` 则会把消息[加入队列](#queue-messages)
        - 在排队消息里输入 stop / pause 关键词，也会让 goose 停止当前任务
        - 你也可以通过[编辑已发送消息](#edit-message)来补充上下文、纠正方向或直接改写会话轨迹
        :::

        <details>
          <summary>中断关键词列表</summary>

          **高优先级关键词**（任何上下文中都会触发中断）：
          ```
          stop, halt, cease, quit, end, abort, cancel, wait, hold, pause, hold on, wait up, hold up
          ```

          **中优先级关键词**（只有在精确匹配或位于句首时才会触发）：
          ```
          no, nope, nah, wrong, incorrect, not right, actually, instead, rather, better idea, change of plans, nevermind, never mind, forget it, ignore that, disregard
          ```

          **识别规则**：
          - **完全匹配**（100% 置信度）：词语 / 短语完全匹配时一定中断
          - **句首出现**（极高置信度）：如果词语 / 短语出现在消息开头，也一定中断
          - **短消息规则**（高置信度）：在长度不超过 20 个字符的消息中，只有高优先级关键词会触发中断
          - **不区分大小写**：所有检测都不区分大小写

          **示例**：
          - ✅ `stop` 会中断（完全匹配）
          - ✅ `Wait, I meant something else` 会中断（出现在句首）
          - ✅ `no` 会中断（短消息 + 高优先级）
          - ❌ `actually` 作为短消息不会中断（中优先级 + 短消息）
          - ✅ `Actually, let's try React instead` 会中断（出现在句首）

        </details>

    </TabItem>
    <TabItem value="cli" label="goose CLI">
        1. 按 `Ctrl+C` 中断当前任务
        2. 输入一条补充上下文或改变方向的新 prompt
        3. 按 `Enter`

        goose 会根据你新的请求继续响应。
        
    </TabItem>
</Tabs>

## 语音输入 {#voice-dictation}
你可以直接对 goose 说话，而不是只靠键盘输入 prompt。

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
    开启语音输入：
        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
        2. 点击侧边栏中的 `Settings`
        3. 点击 `Chat` 标签
        4. 在 `Voice Dictation Provider` 中，从下拉列表选择 provider：
           - Local：本地 Whisper 模型转录，不需要 API key。首次使用会提示你下载模型。
           - [Elevenlabs](https://elevenlabs.io/)
           - [Groq](https://groq.com/)
           - [OpenAI](https://platform.openai.com/api-keys)
        5. 如果系统提示，请输入所选 provider 的 API key

    使用语音输入：
        1. 在侧边栏 `Chat` 中点击你的会话
        2. 点击聊天框右侧的麦克风按钮并开始说话
        3. 若要发送消息，可以：
           - 直接说 “submit”，系统会发送当前消息并继续录制下一条。要停止录制，再点击一次麦克风按钮。
           - 点击麦克风按钮停止录制，然后点击 `Send` 或按 `Enter` 发送。这种方式允许你在发送前先编辑文本。
        
        第一次使用语音输入时，goose 会请求麦克风权限。录音期间你会看到 `Listening` 和 `Transcribing` 状态提示。goose 会在你自然停顿时进行转录，并把文字追加进输入框中。

        **如果你看不到麦克风按钮**，请检查你已配置的[模型](../../getting-started/providers.md)。例如，当你把 OpenAI 设为语音输入 provider 时，即便聊天使用的是别的 LLM provider，也仍然需要在 goose 中配置一个 OpenAI 模型。

       #### 重要说明
        * 最多可录制 50MB 音频
        * ElevenLabs、Groq 和 OpenAI 会把录音上传到它们的服务器处理；Local provider 则完全在本机处理，数据不会离开你的电脑
        * 语音输入会追加到输入框中已有文本的后面
        * 录音在转录完成后不会保存在本地
        * 如果要关闭语音输入，把 provider 切换为 `Disabled`

  </TabItem>
    <TabItem value="cli" label="goose CLI">
        goose CLI 目前不支持语音输入。
    </TabItem>
</Tabs>

## 拼写检查 {#spellcheck}

默认情况下，goose Desktop 聊天输入框会启用拼写检查。

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
    关闭或重新开启拼写检查：
        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
        2. 点击 `Settings`
        3. 点击 `Chat`
        4. 滚动到 `Enable Spellcheck`，切换开关
        5. 重启 goose 让修改生效
        
    </TabItem>
    <TabItem value="cli" label="goose CLI">
        goose CLI 目前不支持拼写检查。
    </TabItem>
</Tabs>

## 在会话中共享文件 {#share-files-in-session}

你可以把代码、文档或其他文件交给 goose，帮助它获得更准确、更有针对性的上下文。

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>
        你可以通过多种方式与 goose 共享文件：

        1. **拖拽上传**：直接把文件从 Finder / 文件管理器拖进聊天窗口，文件路径会自动加入消息中

        2. **文件浏览器**：点击应用底部的 <Paperclip className="inline" size={16} /> 按钮，打开系统文件选择器来选文件

        3. **手动输入路径**：直接在聊天输入框中输入或粘贴文件路径

        4. **快速文件搜索**：使用 [`@` 快捷键](/zh-CN/docs/guides/file-management#quick-file-search-in-goose-desktop) 快速查找并插入文件
    </TabItem>
    <TabItem value="cli" label="goose CLI">
        在 CLI 中，你可以直接在消息里写文件路径。既然你已经处在终端中，也可以利用常见 shell 能力来补齐路径：

        ```bash
        # 引用某个具体文件
        What does this code do? ./src/main.rs

        # 使用 tab 补全
        Can you explain the function in ./src/lib<tab>

        # 使用 shell 展开
        Review these test files: ./tests/*.rs
        ```
    </TabItem>
</Tabs>

## 会话中途修改设置 {#mid-session-changes}

有些设置可以在会话进行中直接修改，并立即生效，而不需要重新开始一个新会话。这样你在与 goose 交互时就能更灵活地调整上下文和能力边界。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

  你可以通过应用底部工具栏，在会话过程中修改这些设置：

  | 设置项 | 工具栏入口 | 持久性* |
  |---------|--------------|-------------|
  | **工作目录** | <FolderDot className="inline" size={16} /> 目录切换器 | 新会话（重启后） |
  | [**启用的扩展**](/zh-CN/docs/getting-started/using-extensions#change-extensions-mid-session) | <Puzzle className="inline" size={16} /> 图标 | 仅当前会话 |
  | [**模型**](../../getting-started/providers.md#configure-provider-and-model) | <Bot className="inline" size={16} /> 模型切换器 | 新会话 |
  | [**goose Mode**](/zh-CN/docs/guides/goose-permissions#configuring-goose-mode) | <Tornado className="inline" size={16} /> 模式切换器 | 新会话 |

  </TabItem>
  <TabItem value="cli" label="goose CLI">

  你可以通过 slash commands 在会话中途调整这些设置：

  | 设置项 | Slash Command | 持久性* |
  |---------|--------------|-------------|
  | [**启用的扩展**](/zh-CN/docs/getting-started/using-extensions#change-extensions-mid-session) | `/extension` 或 `/builtin` | 仅当前会话 |
  | [**goose Mode**](/zh-CN/docs/guides/goose-permissions#configuring-goose-mode) | `/mode [options]` | 新会话 |

  :::info
  CLI 还支持更多 [slash commands](/zh-CN/docs/guides/goose-cli-commands#slash-commands)，但不支持在会话中途修改工作目录或模型。
  :::

  </TabItem>
</Tabs>

*持久性表示这个改动只影响当前会话，还是也会带入后续新会话
