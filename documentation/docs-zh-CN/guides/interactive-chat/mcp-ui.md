---
title: "使用 MCP Apps 和 MCP-UI"
description: "介绍 goose 如何把 MCP Apps 和 MCP-UI 扩展渲染成可交互界面。"
sidebar_position: 1
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import { PanelLeft } from 'lucide-react';

# 使用 MCP Apps 和 MCP-UI

基于 MCP Apps 或 MCP-UI 构建的扩展，可以让 goose Desktop 提供更可交互、参与感更强的用户体验。你不再只是阅读纯文本回复、手动输入提示词，而是可以直接点击和操作图形界面。

:::info MCP Apps is the official specification
[MCP Apps](/zh-CN/docs/tutorials/building-mcp-apps) 现在已经是交互式 UI 的官方 MCP 规范。MCP-UI 扩展在 goose 中仍然可用，但如果你要开发新扩展，推荐直接使用 MCP Apps。
:::

:::warning Experimental Features
本页介绍的功能仍属实验特性，正在持续开发中。后续版本中行为和支持范围可能会变化。
:::

## MCP Apps

MCP Apps 通过官方的 [MCP Apps 规范](https://github.com/modelcontextprotocol/ext-apps) 为 goose 带来交互式界面。根据扩展实现方式，应用可以在独立沙箱窗口中启动，也可以直接嵌入聊天窗口。

### 在独立窗口中启动 App

有些 MCP Apps 可以单独弹出窗口，这样你不用先给 goose 发消息，就能直接进入界面。

1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
2. 点击侧边栏里的 `Apps`
3. 浏览当前可用的 MCP Apps
4. 点击 `Launch` 在新窗口中打开对应应用

:::info Apps Extension
如果你想在侧边栏里看到 `Apps` 页面，需要先在 `Extensions` 页面启用 [Apps 扩展](/zh-CN/docs/mcp/apps-mcp)。你也可以用它来创建自己的独立 App。
:::

`Apps` 页面会展示三类内容：通过 Apps 扩展创建的自定义 HTML 应用、导入的 HTML 应用，以及已启用 MCP Apps 扩展中提供的应用。你可以在这些应用里点击按钮、填写表单、触发其它控件操作。只要通过 CORS 放行，它们就可以通过 MCP 调用工具和读取资源，但不能直接和 goose 本体对话。

#### 导入 HTML App

你也可以导入别人通过 Apps 扩展生成并分享给你的应用：

1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
2. 点击 `Apps`
3. 点击 `Import App`，选择本地 `.html` 文件，然后点击 `Open`

### 在聊天窗口中使用 Apps

有些 MCP Apps 会在 goose 调用返回 UI 的工具时，直接内嵌到当前对话中。这样你就能在聊天流里完成选择、填写表单或触发操作，而不用离开当前会话。

如果不确定某个 UI 是否支持在聊天窗口内加载，直接问 goose 即可。

<div style={{ width: '100%', maxWidth: '800px', margin: '0 auto' }}>
  <video 
    controls 
    playsInline
    style={{ 
      width: '100%', 
      aspectRatio: '2876/2160',
      borderRadius: '8px'
    }}
  >
    <source src={require('@site/static/videos/plan-trip-demo.mp4').default} type="video/mp4" />
    Your browser does not support the video tag.
  </video>
</div>

## MCP-UI

MCP-UI 是更早期的一套交互式 UI 规范，主要把内容嵌入聊天窗口中。虽然现在更推荐 MCP Apps，但 MCP-UI 扩展在 goose 中仍然可以正常工作。

### 试用一下

你可以亲自体验 goose 中的交互式回复。本练习会添加一个扩展，连接到 Andrew Harvard 提供的 [MCP-UI Demos](https://mcp-aharvard.netlify.app/)。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="richdemo"
      extensionName="Rich Demo"
      description="Demo interactive extension"
      type="http"
      url="https://mcp-aharvard.netlify.app/mcp"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
      <CLIExtensionInstructions
        name="rich_demo"
        description="Demo interactive extension"
        type="http"
        url="https://mcp-aharvard.netlify.app/mcp"
        timeout={300}
      />
  </TabItem>
</Tabs>

在 goose Desktop 中，你可以试着问：

- `Help me select seats for my flight`

这时你看到的就不只是纯文本，而会是一个可交互的界面，例如：

- 可视化座位图，显示可选和已占座位
- 可实时点击选择
- 包含航班细节的预订确认界面

你也可以继续试这些 demo：

- `Plan my next trip based on my mood`
- `What's the weather in Philadelphia?`

## 给扩展开发者

如果你想为自己的扩展加入交互能力，可以参考：

- [构建 MCP Apps](/zh-CN/docs/tutorials/building-mcp-apps) - 推荐的分步教程
- [MCP Apps SDK and Specification](https://modelcontextprotocol.github.io/ext-apps/api/)
- [MCP Apps SDK Guide](https://mcpui.dev/guide/introduction)
