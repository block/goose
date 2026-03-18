---
title: "通过安全隧道实现移动访问"
description: "介绍如何通过安全隧道从移动设备远程访问 goose。"
sidebar_position: 3
---

import { PanelLeft } from 'lucide-react';
import ContentCardCarousel from '@site/src/components/ContentCardCarousel';
import mobileShots from '@site/blog/2025-12-19-goose-mobile-terminal/mobile_shots.png';

# 通过安全隧道实现移动访问

移动访问功能允许你通过安全隧道，从 iOS 设备远程连接到 goose。

:::warning Experimental Feature
移动访问目前还是预览特性，功能和配置方式后续可能继续变化。
:::

## 工作原理

移动访问会通过一条安全隧道，把你的 iOS 设备连接到 goose Desktop。安装并配置好 **goose AI** 应用后，你就可以在任意地点访问正在运行的 goose。

关键点：

- 使用 [Lapstone](https://github.com/michaelneale/lapstone-tunnel) 公共 HTTPS 隧道服务
- 通过带唯一 secret key 的二维码完成配置
- 隧道 URL 跨会话保持稳定，通常只需首次配置一次
- 你的电脑必须保持唤醒，且 goose Desktop 正在运行
- 隧道断开后会自动重连，重新启动 goose Desktop 时也会自动恢复

## 设置步骤

### 安装 App

1. 在 iOS 设备上从 [App Store](https://apps.apple.com/app/goose-ai/id6752889295) 安装 **goose AI**

:::tip App Store QR Code
完成下面的步骤进入 `Remote Access` 区域后，可以直接点击 “scan QR code” 旁的信息框，快速跳转到 App Store。
:::

### 启动隧道

1. 打开 goose Desktop
2. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
3. 点击 `Settings`
4. 打开 `App`
5. 滚动到 `Remote Access` 区域，点击 `Start Tunnel`

隧道启动后，你会看到一个用于配置 App 的 `Remote Access Connection` 二维码。

:::info
你可以随时点击 `Stop Tunnel` 关闭连接。
:::

### 连接 App

1. 在 iOS 设备上打开 **goose AI**
2. 扫描 goose Desktop 中显示的 `Remote Access Connection` 二维码
3. App 会自动完成连接配置

完成后，你就可以从移动设备访问正在运行的 goose Desktop。

## 你可以做什么

移动端 App 基本可以访问完整 goose 能力：

- 新建对话或继续已有 session
- 使用你现有的 extensions 和配置
- 在移动端发起任务，由电脑继续承担实际执行和计算

## 额外资源

<ContentCardCarousel
  items={[
    {
      type: 'blog',
      title: 'goose Mobile Access and Native Terminal Support',
      description: '了解两种新的 goose 使用方式：iOS 移动访问，以及带有会话连续性的原生终端支持。',
      thumbnailUrl: mobileShots,
      linkUrl: '/goose/blog/2025/12/19/goose-mobile-terminal',
      date: '2025-12-19',
      duration: '4 min read'
    }
  ]}
/>
