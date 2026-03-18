---
title: "MCP Elicitation"
description: "介绍扩展如何在任务进行中向你请求结构化信息。"
sidebar_position: 55
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# MCP Elicitation

MCP Elicitation 让 goose 在扩展需要额外信息时暂停下来，向你请求结构化输入。它不会猜测或自行假设，而是直接展示一个表单，精确询问继续执行所需的信息。

这个功能在 goose 中默认启用。只要某个支持 elicitation 的扩展需要你补充信息，你就会在当前会话里看到一个表单。

:::info
[MCP Elicitation](https://modelcontextprotocol.io/specification/draft/client/elicitation) 是 Model Context Protocol 的一部分。goose 支持其中的表单模式请求。
:::

## MCP Elicitation 如何工作

当扩展需要你提供信息时，goose 会暂停并展示一个表单。你可以提交答案，也可以取消这次请求。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

表单会以内联方式出现在聊天窗口中，包含：

- 用于填写所需信息的字段
- 用星号（`*`）标记的必填项
- 可直接接受或修改的默认值
- 用于提交答案的 **Submit** 按钮

提交后，你会看到一条确认消息。

  </TabItem>
  <TabItem value="cli" label="goose CLI">

终端里会出现一个交互式提示，包含：

- 一段说明当前需要什么信息的消息（青色）
- 带说明的字段名（黄色）
- 用红色星号（`*`）标记的必填项
- 用方括号显示的默认值，例如 `[default]`

你可以逐项输入答案并按 Enter。对于是 / 否问题，会看到交互式切换控件。

如果要取消这次请求，按 `Ctrl+C` 即可。

  </TabItem>
</Tabs>

:::info Timeout
Elicitation 请求的超时时间是 5 分钟。如果你没有及时回应，请求会被取消，goose 会在没有这部分信息的情况下继续执行。
:::

## 给扩展开发者

如果你想在自己的扩展里加入 elicitation，可以直接参考 [MCP Elicitation 规范](https://modelcontextprotocol.io/specification/draft/client/elicitation)，了解 MCP server 如何向用户请求结构化输入。
