---
sidebar_position: 16
title: 自定义侧边栏
sidebar_label: 自定义侧边栏
description: 调整 goose Desktop 侧边栏的样式、位置和行为，让中文界面下的导航更顺手
---

import { PanelLeft } from "lucide-react";

:::info 仅 goose Desktop 支持
侧边栏自定义功能仅在 goose Desktop 中可用。
:::

侧边栏是你在 goose Desktop 中进入 `Home`、`Chat`、`Recipes`、`Apps`、`Scheduler`、`Extensions` 和 `Settings` 的主要入口。你可以根据自己的习惯调整它的外观、位置和行为。

进入设置路径：

1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
2. 点击 `Settings`
3. 进入 `App` 标签
4. 滚动到 **Navigation** 区域

## 样式

你可以选择侧边栏项目的展示方式。

| 样式 | 说明 |
| --- | --- |
| **Tile**（默认） | 使用较大的图标和标签，以网格形式展示，适合快速扫视。 |
| **List** | 使用更紧凑的单列列表，在较小空间里显示更多项目。 |

:::note
当窗口较窄（小于 700px）时，`List` 样式会自动折叠为仅图标显示。
:::

## 位置

你可以把侧边栏移动到窗口任意一侧。

| 位置 | 说明 |
| --- | --- |
| **Left**（默认） | 位于左侧的竖向侧边栏 |
| **Right** | 位于右侧的竖向侧边栏 |
| **Top** | 位于顶部的横向导航栏 |
| **Bottom** | 位于底部的横向导航栏 |

## 模式

控制侧边栏打开时如何影响主内容区域。

| 模式 | 说明 |
| --- | --- |
| **Push**（默认） | 侧边栏占用布局空间，主内容会被推开，适合长时间保持导航可见。 |
| **Overlay** | 侧边栏浮在内容之上，不改变原有布局；点击空白处或完成导航后即可关闭。 |

:::note
在 `Overlay` 模式下，`Style` 和 `Position` 选项不可用，侧边栏会始终以全屏 tile 覆盖层显示。
:::

## 自定义项目

你可以在 **Customize Items** 区域里调整项目顺序和显示状态：

- 拖拽项目可调整顺序
- 点击眼睛图标可显示或隐藏单个项目

点击 `Reset to defaults` 可恢复默认顺序和可见性。

## 快速切换侧边栏

除了进入设置页，你也可以直接切换侧边栏显示状态：

- 菜单栏：`View → Toggle Navigation`
- 键盘快捷键：快捷键会显示在该菜单项旁边，并可在 `Settings` → `Keyboard` 中修改

侧边栏的开关状态会在会话之间保留。
