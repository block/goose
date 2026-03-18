---
title: "管理工具权限"
description: "介绍如何配置工具权限，控制 goose 在什么条件下调用工具。"
sidebar_position: 1
sidebar_label: "工具权限"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, Tornado, Settings } from 'lucide-react';

工具权限允许你更细粒度地控制 goose 在各个扩展中如何使用工具。本指南会帮助你理解并正确配置这些权限。

## 理解工具与扩展

在讨论权限之前，先明确两个核心概念：

- **Extensions**：为 goose 增加能力的扩展包，例如 Developer、Google Drive 等
- **Tools**：每个扩展内部可以被 goose 调用的具体函数

例如，Developer 扩展就包含多个不同工具：

- 用于编辑文件的文本编辑器工具
- 用于执行命令的 shell 工具
- 用于截图的屏幕捕获工具

:::warning 性能优化建议
goose 在总共启用少于 25 个工具时表现最好。建议只打开当前任务真正需要的扩展。
:::

## 权限级别

工具权限会和 [goose 权限模式](/zh-CN/docs/guides/goose-permissions) 一起工作。权限模式决定默认行为，而工具权限允许你覆盖某个具体工具的行为。

每个工具都可以设置为以下三种权限级别之一：

| 权限级别 | 说明 | 适用场景 | 示例 |
|-----------------|-------------|-----------|----------|
| **Always Allow** | 工具运行时不需要额外确认 | 安全、只读类操作 | • 文件读取<br></br>• 目录列举<br></br>• 信息查询 |
| **Ask Before** | 每次执行前都要求确认 | 会修改状态的操作 | • 文件写入 / 编辑<br></br>• 系统命令<br></br>• 创建资源 |
| **Never Allow** | 完全禁止使用该工具 | 高敏感操作 | • 凭据访问<br></br>• 系统关键文件<br></br>• 删除资源 |

## 配置工具权限

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    当你使用 `Manual` 或 `Smart` 审批模式时，可以为已启用扩展配置细粒度工具权限。这些规则既可以从模式切换入口进入，也可以从 `Settings` 页面进入。

    <Tabs>
      <TabItem value="toggle" label="模式切换入口" default>
        1. 点击应用底部的 <Tornado className="inline" size={16} /> 按钮
        2. 点击当前已选 `Manual` 或 `Smart` 模式旁边的 <Settings className="inline" size={16} /> 按钮
        3. 点击你想配置的扩展
        4. 通过每个工具旁边的下拉框设置权限级别
        5. 点击 `Save Changes`
      </TabItem>
      <TabItem value="settings" label="Settings 页面" default>
        1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
        2. 点击侧边栏中的 `Settings`
        3. 点击 `Chat`
        4. 在 `Mode` 下方，点击当前已选 `Manual` 或 `Smart` 模式旁边的 <Settings className="inline" size={16} /> 按钮
        5. 点击你要配置工具权限的扩展
        6. 使用每个工具旁边的下拉框设置对应权限级别
        7. 点击 `Save Changes`
      </TabItem>
    </Tabs>
  
  </TabItem>
  <TabItem value="cli" label="goose CLI">

    1. 运行配置命令：
    ```sh
    goose configure
    ```

    2. 在菜单中选择 `goose settings`
    ```sh
    ┌ goose-configure
    │
    ◆ What would you like to configure?
    | ○ Configure Providers
    | ○ Add Extension
    | ○ Toggle Extensions
    | ○ Remove Extension
    // highlight-start
    | ● goose settings
    // highlight-end
    └
    ```

    3. 选择 `Tool Permission`
    ```sh
    ┌   goose-configure
    │
    ◇  What would you like to configure?
    │  goose settings
    │
    ◆  What setting would you like to configure?
    │  ○ goose mode
    // highlight-start
    │  ● Tool Permission
    // highlight-end
    |  ○ Tool Output
    └
    ```

    4. 选择某个扩展，并为它的工具配置权限：
    ```sh
    ┌   goose-configure
    │
    ◇  What setting would you like to configure?
    │  Tool Permission 
    │
    ◇  Choose an extension to configure tools
    │  developer 
    │
    ◇  Choose a tool to update permission
    │  developer__image_processor 
    │
    ◆  Set permission level for tool developer__image_processor, current permission level: Not Set
    │  ○ Always Allow 
     // highlight-start
    │  ● Ask Before (Prompt before executing this tool)
    // highlight-end
    │  ○ Never Allow 
    └
    ```
  </TabItem>
</Tabs>

## 为什么要管理工具权限

:::tip
随着任务变化，记得定期复查并更新工具权限。你可以在会话进行中随时修改这些设置。
:::

配置工具权限主要有以下几个原因：

1. **性能优化**
   - 让启用工具总数控制在 25 个以内，以获得更好的性能
   - 关闭当前任务不需要的工具
   - 减少上下文窗口占用，提升响应质量
   - 避免 goose 因工具过多而出现决策瘫痪

2. **安全控制**
   - 限制对敏感操作的访问
   - 避免误改文件
   - 控制系统资源使用范围

3. **任务聚焦**
   - 只为当前任务启用必要工具
   - 帮助 goose 做出更合理的工具选择
   - 减少响应中的噪音

## 示例权限配置

### 按任务类型配置

你可以根据当前任务来决定权限策略：

```
Development Task:
✓ File reading → Always Allow
✓ Code editing → Ask Before
✓ Test running → Always Allow
✗ System commands → Ask Before

Documentation Task:
✓ File reading → Always Allow
✓ Markdown editing → Always Allow
✗ Code editing → Never Allow
✗ System commands → Never Allow
```
