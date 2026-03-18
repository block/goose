---
title: "自定义 Slash Commands"
description: "介绍如何创建自定义快捷指令，在任意 goose 会话里快速注入复用指令。"
sidebar_position: 4
sidebar_title: "Slash Commands"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, Terminal } from 'lucide-react';

自定义 slash command 是一种个性化快捷指令，用来快速运行 [recipes](/zh-CN/docs/guides/recipes)。如果你有一个会生成日报的 recipe，就可以创建一个自定义 slash command，在会话里直接调用它：

```
/daily-report
```

## 创建 Slash Command

给某个 recipe 绑定一个自定义命令。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
   2. 点击侧边栏中的 `Recipes`
   3. 找到你要使用的 recipe，点击 <Terminal className="inline" size={16} /> 按钮
   4. 在弹出的窗口中输入自定义命令（不带前导 `/`）
   5. 点击 `Save`
 
  命令会显示在 `Recipes` 菜单里对应 recipe 的下方。如果该 recipe 不在你的 Recipe Library 中，请改用 `goose CLI` 的方式配置。

  </TabItem>
  <TabItem value="cli" label="goose CLI">

  在你的[配置文件](/zh-CN/docs/guides/config-files)中配置 slash command。把命令名（不带前导 `/`）和 recipe 文件在本机上的路径一起写进去：

```yaml title="~/.config/goose/config.yaml"
slash_commands:
  - command: "run-tests"
    recipe_path: "/path/to/recipe.yaml"
  - command: "daily-report"
    recipe_path: "/Users/me/.local/share/goose/recipes/report.yaml"
```

   </TabItem>
</Tabs>

## 使用 Slash Command

在任意聊天会话中，用带前导斜杠的方式输入自定义命令，并放在消息开头：

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

```
/run-tests
```

:::tip 可用命令提示
在 goose Desktop 中输入 `/`，会弹出一个菜单，展示当前可用的 slash commands。
:::

  </TabItem>
  <TabItem value="cli" label="goose CLI">

```sh
Context: ●○○○○○○○○○ 5% (9695/200000 tokens)
( O)> /run-tests
```

  </TabItem>
</Tabs>

如果有需要，你也可以在命令后额外传一个参数，引号可加可不加：

```
/translator where is the library
```

当你通过 slash command 运行一个 recipe 时，recipe 的 instructions 和 prompt 字段会被发送给模型，并加载进当前对话上下文，但不会直接显示在聊天记录里。模型会像你直接打开 recipe 一样，使用它的上下文和指令来回应。

## 限制

- Slash command 只能接收一个 [parameter](/zh-CN/docs/guides/recipes/recipe-reference#parameters)。如果 recipe 里定义了更多参数，其余参数必须有默认值。
- 命令名不区分大小写（`/Bug` 和 `/bug` 会被当作同一个命令）。
- 命令名必须唯一，且不能包含空格。
- 不能使用与[内置 CLI slash commands](/zh-CN/docs/guides/goose-cli-commands#slash-commands) 冲突的名称，例如 `/recipe`、`/compact` 或 `/help`。
- 如果 recipe 文件不存在或格式无效，这个命令会被当作普通文本发送给模型。

## 更多资源

import ContentCardCarousel from '@site/src/components/ContentCardCarousel';

<ContentCardCarousel
  items={[
    {
      type: 'topic',
      title: 'Recipes',
      description: '查看 Recipes 指南，了解更多文档、工具和资源，掌握 goose recipes 的使用方式。',
      linkUrl: '/goose/docs/guides/recipes'
    },
    {
      type: 'topic',
      title: 'Research → Plan → Implement Patterns',
      description: '了解 slash commands 如何轻松融入交互式的 RPI 工作流。',
      linkUrl: '/goose/docs/tutorials/rpi'
    }
  ]}
/>
