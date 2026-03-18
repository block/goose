---
title: "自定义 Prompt Templates"
description: "介绍如何定制 goose 在不同场景中的 prompt 模板。"
sidebar_position: 86
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft } from 'lucide-react';

# 自定义 Prompt Templates

goose 内置了一组 prompt templates，用来定义它在不同场景中的行为。你可以编辑这些模板，自定义 goose 的回复风格、规划方式、压缩上下文时保存什么，以及其它细节。

## 工作原理

goose 的默认 prompt template 定义在代码库里，并在构建时内嵌到应用中。你可以在本地配置目录里创建同名自定义模板来覆盖默认值，无论是手工创建还是通过 goose Desktop 编辑都可以。

当你自定义模板后：

- 你的改动会跨 goose 升级持续保留
- 代码库中默认模板的变更不会覆盖你的本地版本
- 你可以随时重置回默认模板
- 改动会在新会话中生效

你的修改可以很大，也可以只是小范围微调，例如：

- 编辑 `system.md`，加入 “Reply in Dutch”，让 goose 用荷兰语回复
- 编辑 `plan.md`，加入 “Include an estimated time for each step”，让每个步骤都带上时间预估

关于模板变量的修改方式，请先看 [Template Variable Syntax](#template-variable-syntax)。

:::info Related Configuration
除了 prompt templates，其他设置也会影响 goose 的行为或为它提供上下文，例如 [config files](/zh-CN/docs/guides/config-files)、[.goosehints](/zh-CN/docs/guides/context-engineering/using-goosehints) 以及 [skills](/zh-CN/docs/guides/context-engineering/using-skills)。
:::

## 管理 Prompt Templates

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

  goose Desktop 用户可以在 `Settings` 页面管理模板。

  **要自定义模板：**

  1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
  2. 点击 `Settings`
  3. 打开 `Prompts` 标签
  4. 在目标模板旁点击 `Edit`
  5. 在编辑器里修改内容。你也可以随时点击 `Restore Default`，从默认模板重新开始。
  6. 点击 `Save` 保存改动

  自定义过的模板会显示 `Customized` 徽标。

  **要把某个模板恢复到默认值：**

  1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
  2. 点击 `Settings`
  3. 打开 `Prompts` 标签
  4. 在目标模板旁点击 `Edit`
  5. 点击 `Reset to Default`，删除本地模板文件

  你也可以点击页面顶部的 `Reset All`，一次删除所有本地模板文件。

  </TabItem>
  <TabItem value="cli" label="goose CLI">

  goose CLI 用户可以直接在文件系统里编辑模板文件。

  自定义模板默认保存在：

  - **macOS/Linux:** `~/.config/goose/prompts/`
  - **Windows:** `%APPDATA%\\Block\\goose\\config\\prompts\\`

  **要自定义模板：**

  1. 如果目录不存在，先创建 `prompts` 目录
  2. 从下表中找到模板文件名，例如 `system.md`
  3. 在 prompts 目录中创建同名文件
  4. 写入你的自定义内容并保存。推荐先参考或复制默认模板，再在此基础上修改

  **要恢复默认值：**

  1. 直接删除 prompts 目录中对应的模板文件

  </TabItem>
</Tabs>

### 可自定义的 Prompt Templates

| Template | 说明 | 适用范围 |
|----------|------|----------|
| [system.md](https://github.com/block/goose/blob/main/crates/goose/src/prompts/system.md) | 定义 goose 角色、能力和回复格式的总 system prompt | Desktop 和 CLI |
| [apps_create.md](https://github.com/block/goose/blob/main/crates/goose/src/prompts/apps_create.md) | 用于生成新的独立 App（开发中） | 仅 Desktop |
| [apps_iterate.md](https://github.com/block/goose/blob/main/crates/goose/src/prompts/apps_iterate.md) | 用于更新已有独立 App（开发中） | 仅 Desktop |
| [compaction.md](https://github.com/block/goose/blob/main/crates/goose/src/prompts/compaction.md) | 在上下文达到限制时总结会话历史 | Desktop 和 CLI |
| [permission_judge.md](https://github.com/block/goose/blob/main/crates/goose/src/prompts/permission_judge.md) | 用于分析工具操作是否可视为只读 | Desktop 和 CLI |
| [plan.md](https://github.com/block/goose/blob/main/crates/goose/src/prompts/plan.md) | 生成详细、可执行计划并提出澄清问题 | 仅 CLI |
| [recipe.md](https://github.com/block/goose/blob/main/crates/goose/src/prompts/recipe.md) | 从对话中生成 recipe 文件 | Desktop 和 CLI |
| [subagent_system.md](https://github.com/block/goose/blob/main/crates/goose/src/prompts/subagent_system.md) | 分配给 subagent 时使用的 system prompt | Desktop 和 CLI |

可自定义模板由 [`prompt_template.rs`](https://github.com/block/goose/blob/main/crates/goose/src/prompt_template.rs) 中的 `TEMPLATE_REGISTRY` 数组统一登记。

### Template Variable Syntax

模板使用 [Jinja2](https://jinja.palletsprojects.com/) 语法来插入动态内容：

- `{{ variable }}`：插入变量值，例如 `{{ extensions }}`
- `{% if condition %}...{% endif %}`：条件判断
- `{% for item in list %}...{% endfor %}`：循环

你可以查看默认模板文件，了解常见变量，例如 `{{ extensions }}`、`{{ hints }}` 等。

#### 转义模板变量

如果你想在模板里原样写出变量语法，而不是让它被替换，可以这样写：

```markdown
This will substitute: {{ variable }}
This will appear literally: {{'{{variable}}'}}
```

:::warning
修改模板变量时要小心。错误的变量写法可能会直接破坏功能。建议在新会话中测试改动是否正常。
:::
