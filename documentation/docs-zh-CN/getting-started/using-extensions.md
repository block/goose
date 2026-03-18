---
sidebar_position: 3
title: 使用扩展
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, Settings, Puzzle } from 'lucide-react';

Extensions 是 goose 的附加能力。它们可以把你工作流里已经在使用的应用、服务和工具接入 goose，从而扩展 goose 的能力边界，让它能够访问更多数据、资源，或与其他系统协同工作。

扩展基于 [Model Context Protocol (MCP)](https://github.com/modelcontextprotocol)，因此你可以把 goose 接到一个非常庞大的能力生态中。

goose 会在激活外部扩展前自动检查已知恶意软件。如果检测到恶意包，相关[扩展会被阻止](/zh-CN/docs/troubleshooting/known-issues#malicious-package-detected)，并给出清晰的错误提示。

:::tip 教程
如果你想按步骤接入和使用具体的 goose 扩展，可以查看 [MCP Servers 教程列表](/zh-CN/docs/category/mcp-servers)。
:::

## 内置扩展 {#built-in-extensions}
goose 自带了多种开箱即用的内置扩展：

- [Developer](/zh-CN/docs/mcp/developer-mcp)：提供一组通用开发工具，适合软件开发工作流。**默认启用**
- [Computer Controller](/zh-CN/docs/mcp/computer-controller-mcp)：提供通用的计算机控制能力，适合网页自动化、文件缓存与自动化流程
- [Memory](/zh-CN/docs/mcp/memory-mcp)：让 goose 在使用过程中逐步记住你的偏好
- [Tutorial](/zh-CN/docs/mcp/tutorial-mcp)：提供交互式教程，帮助你学习 goose
- [Auto Visualiser](/zh-CN/docs/mcp/autovisualiser-mcp)：在对话中自动生成图形化数据可视化内容

:::warning 访问控制
goose 默认具备较强自治能力。结合 Developer 扩展后，goose 可以在未经你逐次确认的情况下执行命令和修改文件。如果你希望更精细地控制这些行为，可以配置[goose 权限模式](/zh-CN/docs/guides/goose-permissions)、[工具权限](/zh-CN/docs/guides/managing-tools/tool-permissions)以及 [`.gooseignore` 文件](/zh-CN/docs/guides/using-gooseignore)。简要概览可参考 [Developer 扩展说明](/zh-CN/docs/mcp/developer-mcp)。
:::

### 内置平台扩展

平台扩展也是内置扩展的一种，提供像会话搜索、任务跟踪和扩展管理这类全局能力。它们始终可用，也可以按需打开或关闭。

- [Apps](/zh-CN/docs/mcp/apps-mcp)：创建、管理并以独立窗口方式启动自定义 HTML 应用
- [Chat Recall](/zh-CN/docs/mcp/chatrecall-mcp)：在全部历史会话中搜索对话内容
- [Code Mode](/zh-CN/docs/mcp/code-mode-mcp)：执行 JavaScript 代码，用于工具发现和调用
- [Extension Manager](/zh-CN/docs/mcp/extension-manager-mcp)：在会话中动态发现、启用和停用扩展（默认启用）
- [Summon](/zh-CN/docs/mcp/summon-mcp)：加载 skills 与 recipes，并把任务委派给 subagents（默认启用）
- [Todo](/zh-CN/docs/mcp/todo-mcp)：管理任务列表，并跨会话追踪进度（默认启用）
- [Top of Mind](/zh-CN/docs/mcp/tom-mcp)：在每轮对话时把持久指令注入 goose 的工作记忆

### 开关内置扩展

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏。
  2. 点击侧边栏中的 `Extensions`。
  3. 在 `Extensions` 区域中，可以直接打开或关闭内置扩展。
  </TabItem>

  <TabItem value="cli" label="goose CLI">
    
    如果你已经知道想添加哪个扩展，可以直接运行：

    ```sh
    goose mcp {name}
    ```

    如果你想浏览可用扩展：

    1. 运行：
        ```sh
        goose configure
        ```
    2. 在菜单中选择 `Add Extension`，通过上下方向键选择后按 `Enter`
    3. 选择 `Built-In Extension`
    4. 选择要启用的扩展
    5. 设置扩展 timeout（秒）
    6. 按 `Enter`

    **示例：添加内置扩展**

    ```
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Built-in Extension 
    │
    ◇  Which built-in extension would you like to enable?
    │  Auto Visualiser
    │        
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │ 
    └  Enabled Auto Visualiser extension    
    ```
  </TabItem>
</Tabs>

:::info
goose 自带的扩展本身也是 MCP servers。如果你希望把这些 MCP servers 提供给其他 agent 使用，也完全可以单独接入。
:::

## 发现扩展

goose 提供了一个[集中式扩展目录][extensions-directory]，你可以从中发现、安装并使用扩展。

即使某个扩展没有出现在官方目录中，你仍然可以把任何其他 [MCP Server](#mcp-servers) 接入 goose。

## 添加扩展

你可以通过[扩展目录][extensions-directory]、CLI 或 UI 直接安装扩展。

:::warning Airgapped Environments
如果你处在企业内网或 airgapped 环境中，而扩展无法激活，请查看[Airgapped / Offline Environments](/zh-CN/docs/troubleshooting/known-issues#corporate-proxy-or-firewall-issues)。
:::

### MCP Servers {#mcp-servers}

任何符合 MCP 规范的 server 都可以作为 goose 扩展安装。

:::tip MCP Server Directory
可用的 MCP servers 也可以在 **[MCP Server Directory](https://www.pulsemcp.com/servers)** 中查找。
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
 
  1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏。
  2. 点击侧边栏中的 `Extensions`。
  3. 在 `Extensions` 区域点击 `Add custom extension`。
  4. 在 `Add custom extension` 弹窗中填写所需信息：
     - 如果需要环境变量，点击变量右侧的 `Add`
     - `Timeout` 字段用于设置 goose 等待这个扩展返回工具调用结果的最长时间
  5. 点击 `Add`
  
  #### 示例：添加 [Knowledge Graph Memory MCP Server](https://github.com/modelcontextprotocol/servers/tree/main/src/memory)
    * **Type**：`Standard IO`
    * **ID**：`kgm-mcp`（这个值可以按需自定义）
    * **Name**：`Knowledge Graph Memory`（这个值可以按需自定义）
    * **Description**：`maps and stores complex relationships between concepts`（这个值可以按需自定义）
    * **Command**：`npx -y @modelcontextprotocol/server-memory`
  </TabItem>

  <TabItem value="cli" label="goose CLI">
  
  1. 运行：

    ```sh
    goose configure
    ```

  2. 选择 `Add Extension`。

  3. 选择要添加的扩展类型：
      - `Built-In Extension`：使用 goose 自带的扩展
      - `Command-Line Extension`：把本地命令或脚本作为扩展接入
      - `Remote Extension (Streamable HTTP)`：通过 Streamable HTTP 连接远程系统

  4. 按照你所选扩展类型完成后续提示。

  #### 示例：添加 [Knowledge Graph Memory MCP Server](https://github.com/modelcontextprotocol/servers/tree/main/src/memory)

<Tabs groupId="extensions">
   <TabItem value="node" label="Node">
  ```
 ┌   goose-configure 
 │
 ◇  What would you like to configure?
 │  Add Extension 
 │
 ◇  What type of extension would you like to add?
 │  Command-line Extension 
 │
 ◇  What would you like to call this extension?
 │  Knowledge Graph Memory
 │
 ◇  What command should be run?
 │  npx -y @modelcontextprotocol/server-memory
 │
 ◇  Please set the timeout for this tool (in secs):
 │  300
 │
 ◆  Would you like to add environment variables?
 │  No 
 │
 └  Added Knowledge Graph Memory extension
 ```

   </TabItem>
   <TabItem value="python" label="Python">

  ```
 ┌   goose-configure
 │
 ◇  What would you like to configure?
 │  Add Extension
 │
 ◇  What type of extension would you like to add?
 │  Command-line Extension
 │
 ◇  What would you like to call this extension?
 │  Wikipedia Reader
 │
 ◇  What command should be run?
 │  uvx mcp-wiki
 │
 ◇  Please set the timeout for this tool (in secs):
 │  300
 │
 ◆  Would you like to add environment variables?
 │  No
 │
 └  Added Wikipedia Reader extension
 ```

   </TabItem>
   <TabItem value="java" label="Java">

Note: Java and Kotlin extensions are only support on Linux and macOS

  ```
 ┌   goose-configure
 │
 ◇  What would you like to configure?
 │  Add Extension
 │
 ◇  What type of extension would you like to add?
 │  Command-line Extension
 │
 ◇  What would you like to call this extension?
 │  Spring Data Explorer
 │
 ◇  What command should be run?
 │  jbang -Dspring.profiles.active=dev org.example:spring-data-mcp:1.0.0
 │
 ◇  Please set the timeout for this tool (in secs):
 │  300
 │
 ◆  Would you like to add environment variables?
 │  Yes
 │
 ◇  Environment variable name:
 │  SPRING_DATASOURCE_URL
 │
 ◇  Environment variable value:
 │  jdbc:postgresql://localhost:5432/mydb
 │
 ◇  Add another environment variable?
 │  No
 │
 └  Added Spring Data Explorer extension
 ```

   </TabItem>
  </Tabs>

  </TabItem>
</Tabs>

### Deeplinks

Extensions 也可以通过 goose 的 deeplink 协议安装。URL 格式会根据扩展类型而变化：

<Tabs groupId="interface">
  <TabItem value="stdio" label="StandardIO" default>
```
goose://extension?cmd=<command>&arg=<argument>&id=<id>&name=<name>&description=<description>
```

必填参数：
- `cmd`：要执行的基础命令，必须是 `jbang`、`npx`、`uvx`、`goosed` 或 `docker` 之一
- `arg`：仅 `cmd` 类型使用。命令参数；如果有多个参数，可以重复写多个 `arg`
- `timeout`：等待扩展响应的最长时间（秒）
- `id`：扩展的唯一标识
- `name`：扩展显示名称
- `description`：扩展功能简介

例如，`npx -y @modelcontextprotocol/server-github` 这条命令会对应到：

```
goose://extension?cmd=npx&arg=-y&arg=%40modelcontextprotocol/server-github&timeout=<timeout>&id=<id>&name=<name>&description=<description>
```

注意，`npx` 命令中的每个参数都必须单独作为一个 `arg` 传递。
  </TabItem>
  <TabItem value="streamable_http" label="Streamable HTTP">
```
goose://extension?url=<remote-streamable-http-url>&type=streamable_http&id=<id>&name=<n>&description=<description>
```

参数说明：
- `url`：远程 Streamable HTTP server 的 URL
- `type`：必须设置为 `streamable_http`
- `timeout`：等待扩展响应的最长时间（秒）
- `id`：扩展的唯一标识
- `name`：扩展显示名称
- `description`：扩展功能简介

例如，如果远程地址是 `https://example.com/streamable`，则 URL 编码后的 deeplink 类似这样：

```
goose://extension?url=https%3A%2F%2Fexample.com%2Fstreamable&type=streamable_http&timeout=<timeout>&id=<id>&name=<n>&description=<description>
```

  </TabItem>
</Tabs>

:::note
deeplink 中的所有参数都必须做 URL 编码。例如空格要替换为 `%20`，`@` 要替换为 `%40`。
:::


### Config Entry
对高级用户来说，也可以直接编辑配置文件（`~/.config/goose/config.yaml`），添加、删除或修改扩展：

```yaml
extensions:
  github:
    name: GitHub
    cmd: npx
    args: [-y @modelcontextprotocol/server-github]
    enabled: true
    envs: { "GITHUB_PERSONAL_ACCESS_TOKEN": "<YOUR_TOKEN>" }
    type: stdio
    timeout: 300
```
    

## 启用 / 停用扩展

你可以随时启用或停用已安装扩展，既可以作为新会话的默认配置，也可以只调整当前会话使用的扩展。

### 为新会话设置默认扩展

对默认扩展的改动只会影响后续新会话，不会影响当前已经在运行的会话。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

  1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏。
  2. 点击侧边栏中的 `Extensions`。
  3. 用扩展旁边的开关来启用或停用它。

  </TabItem>

  <TabItem value="cli" label="goose CLI">

  1. 运行以下命令，打开 goose 配置：
      ```sh
      goose configure
      ```
  2. 选择 `Toggle Extensions`。
  3. 系统会列出当前已安装扩展。
  4. 按 `space` 切换扩展状态，实心表示启用。

  **示例：**

  ```
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Toggle Extensions 
  │
  ◆  enable extensions: (use "space" to toggle and "enter" to submit)
  │  ◼ developer 
  │  ◻ fetch 
  └   
  ```
  </TabItem>
</Tabs>

### 在会话中途修改扩展 {#change-extensions-mid-session}

会话内修改扩展不会丢失当前对话，也不需要重新开始。中途启用或停用的扩展只作用于当前会话，不会改动新会话的默认配置。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

  1. 点击应用底部的 <Puzzle className="inline" size={16} /> 按钮。
  2. 用扩展旁边的开关来启用或停用它。

  </TabItem>

  <TabItem value="cli" label="goose CLI">

  在交互式会话中，你可以通过 slash commands 动态添加扩展：

  **添加一个 stdio 扩展：**
  ```bash
  /extension npx -y @modelcontextprotocol/server-memory
  ```

  **添加一个内置扩展：**
  ```bash
  /builtin developer
  ```
  </TabItem>
</Tabs>

## 自动启用的扩展

goose 的 Smart Extension Recommendation 系统会根据你的任务自动识别并建议合适的扩展。本节说明如何使用这套机制，以及它的能力边界。

当你提出一个任务时，goose 会先检查当前已启用的扩展及其工具是否足够完成请求。如果不够，它就会建议你安装或启用其他扩展。你也可以直接按扩展名字点名要求启用。


:::warning
动态启用的扩展只会在当前会话中生效。若要跨会话保留，请参考[启用 / 停用扩展](#启用--停用扩展)。
:::

### 自动检测

goose 会根据任务需求自动判断是否需要某个扩展。下面是一个示例：当对话中需要访问 PostgreSQL 数据库时，goose 会主动识别并启用数据库扩展。

<Tabs groupId="interface">
<TabItem value="ui" label="goose Desktop" default>

#### goose Prompt
```plaintext
Find all orders with pending status from our production database
```

#### goose Output

```plaintext
I'll help you search for available extensions that might help us interact with PostgreSQL databases.

🔍 Search Available Extensions
└─ Output ▼

 I see there's a PostgreSQL extension available. Let me enable it so we can query your database.

🔧 Manage Extensions
└─ action           enable
   extension_name   postgresql

The extension 'postgresql' has been installed successfully

Great! Now I can help you query the database...
```

</TabItem>
<TabItem value="cli" label="goose CLI">

#### goose Prompt
```plaintext
Find all orders with pending status from our production database
```

#### goose Output

```sh
I apologize, but I notice that I don't currently have access to your database. Let me search if there are any database-related extensions available.
─── search_available_extensions | platform ──────────────────────────

I see that there is a "postgresql" extension available. Let me enable it so I can help you query your database.
─── enable_extension | platform ──────────────────────────
extension_name: postgresql


■  goose would like to enable the following extension, do you approve?
// highlight-start
| ● Yes, for this session 
// highlight-end
| ○ No
```

</TabItem>
</Tabs>

### 直接请求

goose 也支持显式启用某个扩展。也就是说，你可以直接告诉它要用哪个工具。

<Tabs groupId="interface">
<TabItem value="ui" label="goose Desktop" default>

#### goose Prompt

```plaintext
Use PostgreSQL extension
```

#### goose Output

```plaintext
I'll help enable the PostgreSQL extension for you.

🔧 Manage Extensions
└─ action           enable
   extension_name   postgresql

The extension 'postgresql' has been installed successfully

The PostgreSQL extension is now ready to use. What would you like to do with it?
```

</TabItem>
<TabItem value="cli" label="goose CLI">

#### goose Prompt

```sh
Use the PostgreSQL extension
```

#### goose Output

```sh
I'll help enable the PostgreSQL extension for you.
─── enable_extension | platform ──────────────────────────
extension_name: postgresql


■  goose would like to enable the following extension, do you approve?
// highlight-start
| ● Yes, for this session 
// highlight-end
| ○ No
```

</TabItem>
</Tabs>

## 更新扩展属性

goose 会依赖扩展属性来决定如何处理某个扩展。如果你想调整扩展的显示方式或运行行为，例如名称、timeout、环境变量等，就需要修改这些属性。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 点击侧边栏里的 `Extensions`
    3. 在目标扩展旁点击 <Settings className="inline" size={16} /> 按钮
    4. 在弹窗里编辑需要修改的属性
    5. 点击 `Save Changes`
  </TabItem>

  <TabItem value="cli" label="配置文件">
    1. 找到 goose 的[配置文件](/zh-CN/docs/guides/config-files)，例如 macOS 上的 `~/.config/goose/config.yaml`
    2. 直接修改该扩展对应的属性并保存
  </TabItem>
</Tabs>

## 删除扩展

你可以移除已经安装的扩展。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 点击侧边栏里的 `Extensions`
    3. 在目标扩展旁点击 <Settings className="inline" size={16} /> 按钮
    4. 在弹窗里点击 `Remove Extension`
  </TabItem>

  <TabItem value="cli" label="配置文件">
    :::info
    删除前需要先把扩展[停用](#启用--停用扩展)。
    :::

    1. 运行：

       ```sh
       goose configure
       ```

    2. 选择 `Remove Extension`
    3. 从列表中找到你要删除的扩展
    4. 按 `space` 选中它，实心表示已选中

       ```
       ┌   goose-configure
       │
       ◇  What would you like to configure?
       │  Remove Extension
       │
       ◆  Select extensions to remove (note: you can only remove disabled extensions - use "space" to toggle and "enter" to submit)
       │  ◼ fetch
       └
       ```

    5. 按 `Enter` 保存
  </TabItem>
</Tabs>

## 带扩展启动会话

你可以直接在 CLI 启动一个“只在当前会话启用这些扩展”的定制 session。

:::info 说明
- 这样做不会把扩展安装成永久配置
- 如果这些扩展本来就已经启用，就没必要重复加
:::

### 内置扩展

要在启动会话时启用内置扩展，可以运行：

```bash
goose session --with-builtin "{extension_id}"
```

例如，要同时启用 Developer 和 Computer Controller：

```bash
goose session --with-builtin "developer,computercontroller"
```

或者写成：

```bash
goose session --with-builtin developer --with-builtin computercontroller
```

### 外部扩展

要在启动会话时启用外部扩展，可以运行：

```bash
goose session --with-extension "{extension command}" --with-extension "{another extension command}"
```

例如，要启用 [Fetch 扩展](https://github.com/modelcontextprotocol/servers/tree/main/src/fetch)：

```bash
goose session --with-extension "uvx mcp-server-fetch"
```

#### 环境变量

有些扩展要求环境变量。你可以把它们直接写进命令：

```bash
goose session --with-extension "VAR=value command arg1 arg2"
```

例如，要启用 [GitHub 扩展](https://github.com/github/github-mcp-server)：

```bash
goose session --with-extension "GITHUB_PERSONAL_ACCESS_TOKEN=<YOUR_TOKEN> npx -y @modelcontextprotocol/server-github"
```

:::info
如果你用的是 `npx` 命令，需要本机先安装 [Node.js](https://nodejs.org/)。
:::

### 通过 Streamable HTTP 启用远程扩展

要在启动会话时启用远程 Streamable HTTP 扩展，可以运行：

```bash
goose session --with-streamable-http-extension "{extension URL}" --with-streamable-http-extension "{another extension URL}"
```

例如：

```bash
goose session --with-streamable-http-extension "https://example.com/streamable"
```

### 在容器中运行扩展

goose 还支持通过 `--container` 参数，让扩展直接在 Docker 容器里运行，特别适合 devcontainer 工作流。具体做法见 [在 Docker 容器中运行扩展](/zh-CN/docs/tutorials/goose-in-docker#running-extensions-in-docker-containers)。

## 开发扩展

goose 扩展建立在 MCP 之上。MCP 是一个标准协议，允许 AI 模型和 agent 安全连接本地或远程资源。如果你要自己开发扩展，可以从[把扩展实现成 MCP server](https://modelcontextprotocol.io/quickstart/server)开始。

**相关教程：**

- [构建自定义扩展](/zh-CN/docs/tutorials/custom-extensions) - 创建一个基于 Python 的 MCP 扩展
- [构建 MCP Apps](/zh-CN/docs/tutorials/building-mcp-apps) - 创建带交互界面的应用型扩展

[extensions-directory]: /extensions
