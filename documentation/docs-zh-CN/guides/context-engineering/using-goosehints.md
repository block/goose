---
title: "为 goose 提供 Hints"
description: "介绍如何使用 AGENTS.md、.goosehints 等文件向 goose 提供项目上下文、偏好和指令。"
sidebar_position: 1
sidebar_label: "使用 goosehints"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft } from 'lucide-react';

`.goosehints` 是一个文本文件，用来补充你的项目上下文，帮助 goose 更准确地理解需求并提升协作效率。通过使用 `.goosehints`，你可以让 goose 更清楚你的约束、偏好和工作方式，从而更有效地执行任务。

<details>
  <summary>goose Hints 视频演示</summary>
  <iframe
  class="aspect-ratio"
  src="https://www.youtube.com/embed/kWXJC5p0608"
  title="goose Hints"
  frameBorder="0"
  allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
  allowFullScreen
  ></iframe>
</details>

如果你发现自己在反复输入类似提示、重复强调同一套要求，或者总要补充一大段项目背景，那么就是该考虑新增 `.goosehints` 文件的时候了。很多更适合写进文件的长期上下文，都可以放在这里。

本指南会带你了解如何创建和使用 `.goosehints` 文件，用自定义指令和上下文来简化日常工作流。

:::info 需要启用 Developer 扩展
要使用 hints 文件，你需要先[启用](/zh-CN/docs/getting-started/using-extensions) `Developer` 扩展。
:::

## 创建 Hints 文件

goose 支持两种 hints 文件：

- **全局 hints 文件**：对你所有目录下的 goose 会话都生效。全局 hints 存放在 `~/.config/goose/.goosehints`。
- **本地 hints 文件**：只在某个特定目录或目录层级中生效。

你可以同时使用全局 hints 和本地 hints。两者同时存在时，goose 会同时考虑你的全局偏好和项目级要求。如果本地 hints 和全局偏好冲突，goose 会优先采用本地 hints。

:::tip 自定义上下文文件
如果你想让 goose 加载其他 agent 规则文件，可以使用 [`CONTEXT_FILE_NAMES` 环境变量](#custom-context-files)。
:::

<Tabs groupId="interface">
    <TabItem value="ui" label="goose Desktop" default>

    #### 全局 hints 文件
    1. 在 `~/.config/goose` 下创建 `.goosehints` 文件

    #### 本地 hints 文件

    1. 点击应用底部的目录路径，打开你要创建文件的目录
    2. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
    3. 点击侧边栏中的 `Settings`
    4. 点击 `Chat`
    5. 向下滚动到 `Project Hints (.goosehints)` 区域并点击 `Configure`
    6. 在文本框中输入你的本地 hints
    7. 点击 `Save`
    8. 重启当前会话，让 goose 读取更新后的 `.goosehints`

    如果指定目录中已经存在 `.goosehints` 文件，你也可以直接编辑已有内容。

    </TabItem>
    <TabItem value="manual" label="手动">
    
    - **全局 hints 文件**：在 `~/.config/goose` 下创建 `.goosehints` 文件。
    - **本地 hints 文件**：在你的项目根目录，或目录层级中的任意目录下创建 `.goosehints` 文件。

    </TabItem>
</Tabs>

`.goosehints` 文件中可以写入任何与你的项目相关的指令或上下文信息。

## 编写 Hints

`.goosehints` 支持自然语言。建议使用清晰、直接、具体的表达方式，让 goose 更容易理解并执行。把与你的项目背景、协作方式和工作偏好相关的信息写进去，并把最重要的规则放在前面。

goose 会在会话开始时加载 hints，并把它们放进每次请求都会携带的 system prompt 中。这意味着 `.goosehints` 的内容会消耗 token，所以保持精简能同时节省成本和处理时间。

### 全局 `.goosehints` 示例

```
Always use TypeScript for new Next.js projects.

@coding-standards.md  # Contains our coding standards
docs/contributing.md  # Contains our pull request process

Follow the [Google Style Guide](https://google.github.io/styleguide/pyguide.html) for Python code.

Run unit tests before committing any changes.

Prefer functional programming patterns where applicable.
```

### 本地 `.goosehints` 示例

```
This is a simple example JavaScript web application that uses the Express.js framework. View [Express documentation](https://expressjs.com/) for extended guidance.

Go through the @README.md for information on how to build and test it as needed.

Make sure to confirm all changes with me before applying.

Run tests with `npm run test` ideally after each change.
```

这些示例展示了两种引用其他文件的方式：

- **`@` 语法**：自动把文件内容加入 goose 的即时上下文
- **普通引用**：提示 goose 在需要时查看该文件，适合可选文件或体积很大的文件

### 嵌套 `.goosehints` 文件

在 git 仓库中，goose 支持按层级加载本地 hints。它会自动从当前目录一路向上找到仓库根目录，把沿途所有 `.goosehints` 文件合并加载。如果你不在 git 仓库中工作，goose 只会读取当前目录中的 `.goosehints` 文件。

最佳实践是：每一层的 `.goosehints` 只放与该层级相关的内容：

- **根目录层级**：项目范围的规范、构建流程和通用要求
- **模块 / 功能层级**：该部分代码的专门要求
- **具体目录层级**：本地测试方式、组件约定等更细粒度的上下文

**示例项目结构：**
```sh
my-project/
├── .git/
├── .goosehints              # 项目级 hints
├── frontend/
│   ├── .goosehints          # 前端专属 hints
│   └── components/
│       ├── .goosehints      # 组件级 hints
│       └── Button.tsx
└── backend/
    ├── .goosehints          # 后端专属 hints
    └── api/
        └── routes.py
```

在上面的示例项目中，如果你正在 `frontend/components/` 下工作，goose 会按以下顺序加载上层目录中的 hints：
1. <details>
     <summary>`my-project/.goosehints`（项目根目录）</summary>
        ```
        This is a React + TypeScript project using Vite.

        @README.md                    # Project overview and setup instructions
        @docs/development-setup.md    # Development environment configuration

        Always run tests before committing: `npm test`
        Use conventional commits for all changes.
        ```
   </details>
2. <details>
     <summary>`frontend/.goosehints`</summary>
        ```
        This frontend uses React 18 with TypeScript and Tailwind CSS.

        @package.json                      # Dependencies and scripts
        @docs/frontend-architecture.md     # Frontend structure and patterns

        ## Development Standards
        - Use functional components with hooks (no class components)
        - Implement proper TypeScript interfaces for all props
        - Follow the component structure: /components/ComponentName/index.tsx
        - Use Tailwind classes instead of custom CSS when possible

        ## Testing Requirements  
        - Write unit tests for all components using React Testing Library
        - Test files should be co-located: ComponentName.test.tsx
        - Run `npm run test:frontend` before committing changes

        ## State Management
        - Use React Query for server state
        - Use Zustand for client state management
        - Avoid prop drilling - lift state appropriately

        Always confirm UI changes with design team before implementation.
        ```
   </details> 
3. <details>
     <summary>`frontend/components/.goosehints`（当前目录）</summary>
        ```
        Components in this directory use our design system.

        @docs/component-api.md    # Component interface standards and examples

        All components must:
        - Export a default component
        - Include TypeScript props interface
        - Have corresponding .test.tsx file
        - Follow naming convention: PascalCase
        ```
   </details>

## 常见使用场景

下面是一些常见的 hints 用法，用来给 goose 提供额外上下文：

- **决策方式**：说明 goose 是否可以自主修改，还是必须先征求你的确认。
- **校验流程**：写明 goose 需要执行哪些测试或验证步骤，确保改动符合要求。
- **反馈闭环**：加入允许 goose 接收反馈并持续调整建议的步骤。
- **指向更详细的文档**：告诉 goose 哪些文件值得重点参考，比如 `README.md`、`docs/setup-guide.md` 等。
- **通过 `@` 组织上下文**：对于经常要看的文档，用 `@filename.md` 或 `@relative/path/testing.md` 让 goose 直接把内容拉进上下文，而不是只给一个提示。核心文档（如 API schema、编码规范）适合用 `@` 立即引入；可选文件或很大的文件更适合普通引用。

和普通 prompt 一样，这些并不是塑造 `.goosehints` 文件的完整清单。你可以按需要加入任意有价值的上下文。

## 最佳实践

- **及时更新文件**：项目约定或优先级变化后，记得同步更新 `.goosehints`。
- **保持简洁**：内容尽量直接、明确，让 goose 能快速解析并执行。
- **从小处开始**：先写一小组清楚、具体的 hints，再根据需要逐步扩展。这样更容易观察 goose 如何理解和应用这些规则。
- **多引用、少重复**：把 goose 指向像 `/docs/style.md` 或 `/scripts/validation.js` 这样的相关文件，减少重复内容，让 hints 更轻量。

## 自定义上下文文件 {#custom-context-files}

默认情况下，goose 会查找 `AGENTS.md`，然后再查找 `.goosehints`。如果你想换成其他文件名，或者同时加载多个上下文文件，可以通过 `CONTEXT_FILE_NAMES` 环境变量来配置。这在以下场景尤其有用：

- **兼容其他工具**：沿用其他 AI 工具的命名约定，例如 `CLAUDE.md`
- **组织方式更清晰**：把常用规则拆分成多个自动加载的文件
- **遵循项目约定**：沿用项目既有工具链使用的上下文文件，例如 `.cursorrules`

工作方式如下：
1. goose 会在全局目录（`~/.config/goose/`）和本地目录（当前目录）中查找每个配置好的文件名
2. 找到的所有文件都会被加载并合并到上下文中

### 配置方式

把 `CONTEXT_FILE_NAMES` 环境变量设置为一个 JSON 数组。默认值是 `["AGENTS.md", ".goosehints"]`。

```bash
# 单个自定义文件
export CONTEXT_FILE_NAMES='["AGENTS.md"]'

# 项目工具链文件
export CONTEXT_FILE_NAMES='[".cursorrules", "AGENTS.md"]'

# 多个文件
export CONTEXT_FILE_NAMES='["CLAUDE.md", ".goosehints", "project_rules.txt"]'
```
