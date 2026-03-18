---
title: "goose Mobile"
description: "介绍 goose Mobile 相关的实验性能力与使用方向。"
sidebar_position: 4
---

# goose Mobile

:::info Archived
goose Mobile 已归档。现在从移动设备访问 goose 的推荐方式，是通过 [iOS 安全隧道访问](/zh-CN/docs/experimental/mobile-access)。
:::

goose Mobile 是一个实验性的 Android 项目，灵感来自 goose 本体。它尝试把 open agent 直接带到手机上，用来自动执行多步骤任务、响应通知，甚至替代主屏工作流。

:::danger Experimental
goose Mobile 需要对设备有较深的访问权限。请自行评估风险，最好在备用手机或模拟器上体验。
:::

<div style={{textAlign: 'center'}}>
  <img src="https://github.com/user-attachments/assets/af9d7d83-54f4-4ace-ad66-9e19f86c8fb9" alt="goose Mobile Screenshot" />
</div>

## 它能做什么

- **任务自动化**：编排手机中已安装应用完成多步骤任务
- **通知处理**：按你的规则响应收到的通知
- **可扩展性**：通过其它应用提供的扩展能力，在后台无缝完成任务

## 安装

- **预构建 APK**：可以直接通过 [Firebase distribution link](https://appdistribution.firebase.google.com/pub/i/3f111ea732d5f7f6) 安装
- **从源码构建**：开发说明见 [goose Mobile 仓库](https://github.com/block/goose-mobile)

## 扩展 goose Mobile

goose Mobile 支持 **mobile MCP** 机制，可以让它在不离开当前上下文的情况下调用其它 App 的工具，例如通过天气扩展查询天气。

示例代码和配置说明都在仓库的 [README](https://github.com/block/goose-mobile) 中。

## 参与贡献

欢迎贡献。具体方式可参考 [Contributing Guide](https://github.com/block/goose-mobile/blob/main/CONTRIBUTING.md)。

如果你要了解更多使用场景、开发方式和完整设置说明，直接查看 [goose Mobile 仓库](https://github.com/block/goose-mobile)。
