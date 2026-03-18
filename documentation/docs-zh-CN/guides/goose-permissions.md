---
sidebar_position: 20
title: "goose 权限模式"
sidebar_label: "goose 权限模式"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, Tornado } from 'lucide-react';

goose 的权限模式决定了它在修改文件、使用扩展和执行自动化操作时拥有多大自治权。通过选择不同模式，你可以精确控制 goose 与开发环境的交互边界。

<details>
  <summary>权限模式视频演示</summary>
  <iframe
  class="aspect-ratio"
  src="https://www.youtube.com/embed/bMVFFnPS_Uk"
  title="goose Permission Modes Explained"
  frameBorder="0"
  allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
  allowFullScreen
  ></iframe>
</details>

## 权限模式

| Mode | Description | Best For |
|------|-------------|----------|
| **Completely Autonomous** | goose 可以**无需审批**地修改文件、使用扩展和删除文件 | 希望获得**完全自动化**体验，并把 goose 深度接入日常流程的用户 |
| **Manual Approval** | goose 在使用任何工具或扩展前都会**逐项请求确认**（支持细粒度[工具权限](/zh-CN/docs/guides/managing-tools/tool-permissions)） | 希望**逐项审查并批准**每次变更和工具调用的用户 |
| **Smart Approval** | goose 基于风险判断，**自动放行低风险操作**，并对其他操作请求审批（支持细粒度[工具权限](/zh-CN/docs/guides/managing-tools/tool-permissions)） | 希望在自治和可控之间取得**平衡**的用户 |
| **Chat Only** | goose **只参与聊天**，不会使用扩展，也不会修改文件 | 只想获得**对话式 AI 能力**，用于分析、写作、推理而不做自动操作的用户 |

:::warning
默认启用的是 `Autonomous Mode`。
:::

## 配置 goose mode {#configuring-goose-mode}

可以按下面的方法进行配置：

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

    你可以在会话开始前或会话进行中切换模式，修改会立即生效。

     <Tabs groupId="method">
      <TabItem value="session" label="会话中切换" default>

      点击应用底部菜单中的 <Tornado className="inline" size={16} /> 模式按钮。 
      </TabItem>
      <TabItem value="settings" label="在 Settings 中配置">
        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏。
        2. 点击侧边栏中的 `Settings`。
        3. 点击 `Chat`。
        4. 在 `Mode` 下选择你想使用的模式。
      </TabItem>
    </Tabs>   
  </TabItem>
  <TabItem value="cli" label="goose CLI">

    <Tabs groupId="method">
      <TabItem value="session" label="会话中切换" default>
        如果要在会话中途切换模式，可以使用 `/mode` 命令。

        * Autonomous：`/mode auto`
        * Smart Approve：`/mode smart_approve`
        * Approve：`/mode approve`
        * Chat：`/mode chat`     
      </TabItem>
      <TabItem value="settings" label="在 Settings 中配置">
        1. 运行以下命令：

        ```sh
        goose configure
        ```

        2. 在菜单中选择 `goose settings` 并回车。

        ```sh
        ┌ goose-configure
        │
        ◆ What would you like to configure?
        | ○ Configure Providers
        | ○ Add Extension
        | ○ Toggle Extensions
        | ○ Remove Extension
        // highlight-start
        | ● goose settings (Set the goose mode, Tool Output, Tool Permissions, Experiment, goose recipe github repo and more)
        // highlight-end
        └
        ```

        3. 选择 `goose mode` 并回车。

        ```sh
        ┌   goose-configure
        │
        ◇  What would you like to configure?
        │  goose settings 
        │
        ◆  What setting would you like to configure?
        // highlight-start
        │  ● goose mode (Configure goose mode)
        // highlight-end
        │  ○ Router Tool Selection Strategy 
        │  ○ Tool Permission 
        │  ○ Tool Output 
        │  ○ Max Turns 
        │  ○ Toggle Experiment 
        │  ○ goose recipe github repo 
        │  ○ Scheduler Type 
        └
        ```

        4. 选择你想配置的 goose mode。

        ```sh
        ┌   goose-configure
        │
        ◇  What would you like to configure?
        │  goose settings
        │
        ◇  What setting would you like to configure?
        │  goose mode
        │
        ◆  Which goose mode would you like to configure?
        // highlight-start
        │  ● Auto Mode (Full file modification, extension usage, edit, create and delete files freely)
        // highlight-end
        |  ○ Approve Mode
        |  ○ Smart Approve Mode    
        |  ○ Chat Mode
        |
        └  Set to Auto Mode - full file modification enabled
        ```     
      </TabItem>
    </Tabs>
  </TabItem>
</Tabs>

  :::info
  在 manual 和 smart approval 模式下，当发生工具调用时，你会在会话窗口中看到 “Allow” 和 “Deny” 按钮。  
  goose 通常只会对它判断为“写操作”的工具请求审批，例如文本编辑器写入 / 编辑，或 `bash - rm, cp, mv` 这类命令。
  
  读 / 写分类本质上是一个 best-effort 判断，最终解释仍然依赖你使用的 LLM provider。
  :::
