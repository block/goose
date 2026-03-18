---
title: "匿名使用数据"
description: "介绍 goose 收集哪些匿名使用数据以及如何控制。"
sidebar_position: 66
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft } from 'lucide-react';

# 匿名使用数据

首次使用 goose 时，系统会询问你是否同意收集匿名使用数据，以帮助改进产品。你可以随时修改这个设置。

## 会收集哪些使用数据

为了尽量保护隐私，只有在你明确开启后，goose 才会收集匿名使用指标。开启后，收集的数据包括：

- 操作系统、版本和架构
- goose 版本和安装方式
- 所使用的 provider 和 model
- 扩展名称以及工具使用次数
- 会话指标，例如时长、交互次数、token 使用量
- 错误类型，例如 `rate_limit`、`auth`，但不包含错误详情

收集的使用数据**不包括**你的对话内容、代码、工具参数、错误消息或任何个人数据。

:::info Provider Data Handling
你在 goose 中使用的 [LLM providers](/zh-CN/docs/getting-started/providers)，可能会接收你的对话、提示词以及 goose 访问到的信息；这些数据是否保留、如何处理，取决于对应 provider 的隐私和数据保留政策。
:::

## 修改你的偏好设置

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
    2. 点击侧边栏中的 `Settings`
    3. 进入 `App` 标签
    4. 在 `Privacy` 区域里打开或关闭 `Anonymous usage data`
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    使用方向键在选项间移动，按 `Enter` 选择。实心圆点表示当前选中项。

    1. 运行 `goose configure`
    2. 选择 `goose settings`
    3. 选择 `Telemetry`
    4. 界面会显示当前的 telemetry 状态，选择 `Yes` 启用匿名使用数据收集，选择 `No` 关闭

    ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  goose settings 
    │
    ◇  What setting would you like to configure?
    │  Telemetry 
    │
    ●  Current telemetry status: Disabled
    │  
    ◇  Share anonymous usage data to help improve goose?
    │  Yes 
    │
    └  Telemetry enabled - thank you for helping improve goose!
    └  Configuration saved successfully to /Users/julesv/.config/goose/config.yaml
    ```
  </TabItem>
</Tabs>

你也可以直接在 [`config.yaml` 文件](/zh-CN/docs/guides/config-files) 中设置 `GOOSE_TELEMETRY_ENABLED`，或者把它作为[环境变量](/zh-CN/docs/guides/environment-variables#security-and-privacy)使用，仅对当前会话生效。
