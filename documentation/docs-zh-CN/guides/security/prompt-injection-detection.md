---
sidebar_position: 1
title: "Prompt Injection Detection"
sidebar_label: "Prompt Injection Detection"
description: "介绍如何在潜在有害命令执行前检测并阻止 prompt injection。"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, Settings } from 'lucide-react';

Prompt injection 指的是：攻击者把恶意指令藏在可执行内容中，诱导 AI agent 偏离预期。对 goose 这类 agent 来说，prompt injection 可能会让它执行危险命令，从而影响你的环境或数据安全。

你可以通过开启 prompt injection detection 来降低这类风险。这个功能会使用模式匹配来识别常见攻击手法，包括：
- 尝试删除系统文件或目录
- 下载并执行远程脚本的命令
- 试图访问或外传 SSH key 等敏感数据
- 可能危害安全的系统修改操作

此外，你还可以按需启用指定模型的 [ML 检测](#enhanced-detection-with-machine-learning)。

:::important
这些检查是防护措施，不是绝对保证。它们能识别已知模式，但无法覆盖所有攻击，特别是新型或更复杂的攻击方式。
:::

## 检测是怎么工作的

开启后，goose 会在执行前使用多层策略检测风险：

1. **拦截并分析工具调用**：当 goose 准备执行某个工具时，安全系统会提取工具参数文本，并与[威胁模式](https://github.com/block/goose/blob/main/crates/goose/src/security/patterns.rs)进行匹配。如果启用了 ML 检测，还会结合最近的对话消息做语义分析，以更好理解上下文并减少误报。
2. **评估风险**：检测到的威胁会被赋予置信度分数。
3. **暂停执行**：超过阈值的威胁需要你来决定是否继续。
4. **显示安全告警**：告警会展示置信度、命中的风险说明以及唯一的 finding ID。例如：
   ```
   🔒 Security Alert: This tool call has been flagged as potentially dangerous.
   
   Confidence: 95%
   Explanation: Detected 1 security threat: Recursive file deletion with rm -rf
   Finding ID: SEC-abc123...
   
   [Allow Once] [Deny]
   ```
5. **由你决定**是否继续执行。需要注意：
   - 每次决策都会连同 finding ID 一起记录到 [goose system logs](/zh-CN/docs/guides/logs#system-logs)
   - 如果你选择允许，命令仍会以你当前的完整权限继续执行

**面对告警时，建议这样处理：**

- 先阅读解释，理解触发原因
- 结合当前上下文判断：这是否符合你原本的意图？
- 尝试把请求改写得更具体一些
- 确认提示来源；对于未知来源的 prompt，务必更谨慎

如果拿不准，优先拒绝。

## 开启检测

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
    2. 点击侧边栏中的 `Settings`
    3. 点击 `Chat` 标签
    4. 打开 `Enable Prompt Injection Detection`
    5. 如有需要，调整 `Detection Threshold`，以[配置灵敏度](#configuring-detection-threshold)
    6. 如有需要，启用 ML 检测：
       1. 打开 `Enable ML-based Detection`
       2. 配置推理服务：
          - `Endpoint URL`：分类服务的 URL（例如 Hugging Face）
          - `API Token`：如果服务需要鉴权，就填写对应 token

  </TabItem>
  <TabItem value="config" label="goose config file">

    你也可以把相关安全设置写入 [`config.yaml`](/zh-CN/docs/guides/config-files)：

    ```yaml
    SECURITY_PROMPT_ENABLED: true
    SECURITY_PROMPT_THRESHOLD: 0.8  # Optional, default is 0.8

    # Optional: Enable ML-based detection (Hugging Face example)
    SECURITY_PROMPT_CLASSIFIER_ENABLED: true
    SECURITY_PROMPT_CLASSIFIER_ENDPOINT: "https://router.huggingface.co/hf-inference/models/protectai/deberta-v3-base-prompt-injection-v2"
    SECURITY_PROMPT_CLASSIFIER_TOKEN: "YOUR_HUGGING_FACE_TOKEN"
    ```

  </TabItem>
</Tabs>

:::info 其他安全机制
除了 prompt injection detection，goose 还会自动：
- 在运行新的或更新过的 recipe 前给出警告
- 在导入包含不可见 Unicode Tag Block 字符的 recipe 时给出警告
- 在安装本地运行的 MCP server 扩展时，[检查已知恶意软件](/zh-CN/docs/troubleshooting/known-issues#malicious-package-detected)
:::

### 配置检测阈值 {#configuring-detection-threshold}

阈值（0.01-1.0）决定了检测有多严格：

| Threshold | Sensitivity | Use When |
|-----------|------------|----------|
| **0.01-0.50** | 非常宽松 | 你很熟悉风险边界，且愿意自行判断 |
| **0.50-0.70** | 均衡 | 一般开发工作（推荐默认段） |
| **0.70-0.90** | 严格 | 正在处理敏感数据或敏感系统 |
| **0.90-1.00** | 最严格 | 高安全要求环境 |

开启 prompt injection detection 后，默认阈值是 0.8（适合大多数用户）。

阈值越低，告警越少，但可能漏掉风险；阈值越高，潜在威胁抓得更多，但也更容易误报。你可以根据自己的风险偏好，在灵敏度和便利性之间做取舍。

## 使用机器学习增强检测 {#enhanced-detection-with-machine-learning}

默认情况下，prompt injection detection 依赖模式匹配；但你也可以启用 ML 检测，以提高准确率并减少误报。

ML 检测可以：
- 分析工具调用和最近消息的语义内容
- 发现模式匹配可能漏掉的更复杂攻击
- 通过理解对话上下文来减少误报
- 需要你提供分类服务的 endpoint URL，以及必要时的 API token

:::warning 隐私提示
启用 ML 检测后，工具调用内容和最近消息会被发送到你配置的 endpoint 做分析。
:::

#### 自托管 ML 检测端点
如果你想自己搭建分类 endpoint，请参考 [Classification API Specification](/zh-CN/docs/guides/security/classification-api-spec)。这个 API 遵循 Hugging Face Inference API 的格式。

## 另见

- [goose Permission Modes](/zh-CN/docs/guides/goose-permissions) - 控制 goose 的自治程度
- [Managing Tool Permissions](/zh-CN/docs/guides/managing-tools/tool-permissions) - 更细粒度地限制工具行为
