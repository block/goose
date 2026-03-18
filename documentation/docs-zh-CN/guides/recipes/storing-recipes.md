---
title: "保存 Recipes"
description: "介绍如何保存、整理、导入并再次找到 goose recipes。"
sidebar_position: 4
sidebar_label: "保存 Recipes"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, ChefHat } from 'lucide-react';

本指南介绍当你后续还需要再次使用 recipe 时，如何保存、组织、导入和查找 goose recipes。

:::info Desktop UI vs CLI
- **goose Desktop** 提供可视化的 Recipe Library，用来浏览和管理已保存的 recipes
- **goose CLI** 把 recipes 存成文件，并通过路径或环境变量查找它们
:::

## 理解 Recipe 存储

在保存 recipe 之前，先理解 recipe 可以存放在哪里，以及这会如何影响它的可用范围。

### Recipe 存储位置

| 类型 | 位置 | 可用范围 | 适合场景 |
|------|------|----------|----------|
| **全局** | `~/.config/goose/recipes/` | 所有项目和会话 | 个人工作流、通用 recipe |
| **本地** | `YOUR_WORKING_DIRECTORY/.goose/recipes/` | 仅当前项目可见 | 项目专属流程、团队共享 recipe |

**适合使用全局存储的情况：**
- 你希望 recipe 在所有项目里都能使用
- 它是个人工作流或通用 recipe
- 主要由你自己使用

**适合使用本地存储的情况：**
- recipe 只适用于某个特定项目
- 你在团队里协作，希望项目成员共享同一套 recipe
- recipe 依赖项目专属文件或配置

## 保存 Recipes

<Tabs groupId="interface">
  <TabItem value="desktop" label="goose Desktop" default>

**保存新 Recipe：**

1. 若要从当前会话创建 recipe，请先参考[创建 Recipe](/zh-CN/docs/guides/recipes/session-recipes#create-recipe)
2. 进入 Recipe Editor 后，点击 `Save Recipe` 将其保存到 Recipe Library

**保存已修改的 Recipe：**

如果你已经在使用某个 recipe，并想把修改后的版本保存下来：

1. 发送第一条消息后，点击应用底部的 <ChefHat className="inline" size={16}/> 按钮
2. 修改 instructions、prompt 或其他字段
3. 点击 `Save Recipe`

:::info
如果你修改 recipe 后使用了新名称保存，系统会生成一个新的 recipe 和新链接。原来的 recipe 仍可通过 Recipe Library 或旧链接继续运行。

如果你编辑 recipe 时没有改名，Recipe Library 里的版本会被更新，但原始链接仍然可以继续打开旧版本。
:::

  </TabItem>
  <TabItem value="cli" label="goose CLI">

当你[创建 recipe](/zh-CN/docs/guides/recipes/recipe-reference)时，它会被保存到：

- 当前工作目录，默认是 `./recipe.yaml`
- 你显式指定的任意路径，例如：`/recipe /path/to/my-recipe.yaml`
- 项目本地 recipe 目录，例如：`/recipe .goose/recipes/my-recipe.yaml`

:::note
CLI 只会把 recipe 保存成 `.yaml` 文件。虽然 CLI 也可以运行 `.json` 格式的 recipe，但它本身不提供直接保存为 JSON 的选项。
:::

  </TabItem>
</Tabs>

### 导入 Recipes {#importing-recipes}

<Tabs groupId="interface">
  <TabItem value="desktop" label="goose Desktop" default>
    你可以通过 deeplink 或 recipe 文件导入 recipe：

    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 在侧边栏里点击 `Recipes`
    3. 点击 `Import Recipe`
    4. 选择导入方式：
       - 通过链接导入：在 `Recipe Deeplink` 区域粘贴[recipe 链接](/zh-CN/docs/guides/recipes/session-recipes#share-via-recipe-link)
       - 通过文件导入：在 `Recipe File` 区域点击 `Choose File`，选择 recipe 文件后点击 `Open`
    5. 点击 `Import Recipe`，将 recipe 的副本保存到你的 Recipe Library

  :::warning Recipe 文件格式
  goose Desktop 接受 `.yaml`、`.yml` 和 `.json` 文件，但 **CLI 只支持 `.yaml` 和 `.json`**。如果你希望同时兼容 Desktop 和 CLI，最好不要使用 `.yml` 扩展名。

  所有 recipe 文件都遵循相同的 [schema 结构](/zh-CN/docs/guides/recipes/recipe-reference#core-recipe-schema)。
  :::

  </TabItem>
  <TabItem value="cli" label="goose CLI">
    通过 GUI 导入 recipe 目前只在 goose Desktop 中支持。
  </TabItem>
</Tabs>

## 查找可用 Recipes

<Tabs groupId="interface">
  <TabItem value="desktop" label="goose Desktop" default>

**进入 Recipe Library：**
1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
2. 点击 `Recipes` 打开 Recipe Library
3. 浏览你当前可用的 recipes，列表中会显示：
   - Recipe 标题与描述
   - 最近修改时间
   - 它是全局存储还是本地存储

:::info Desktop 与 CLI 的 recipe 发现方式不同
Desktop 的 Recipe Library 只展示你显式保存或导入过的 recipes。它不会像 CLI 那样自动扫描文件系统中的 recipe 文件。
:::

  </TabItem>
  <TabItem value="cli" label="goose CLI">

使用 `goose recipe list` 可以从多个来源查找当前可用的 recipes：

**基本用法**

```bash
# 列出所有可用 recipes
goose recipe list

# 输出更详细的信息，包括标题和完整路径
goose recipe list --verbose

# 输出 JSON，方便自动化处理
goose recipe list --format json
```

**Recipe 发现顺序**

goose 会按以下顺序查找 recipe：

1. **当前目录**：`.`，搜索 `*.yaml` 和 `*.json`
2. **自定义路径**：[`GOOSE_RECIPE_PATH`](/zh-CN/docs/guides/environment-variables) 环境变量指定的目录
3. **全局 recipe 库**：`~/.config/goose/recipes/`（或操作系统对应位置）
4. **项目本地 recipe**：`./.goose/recipes/`
5. **GitHub 仓库**：如果配置了 [`GOOSE_RECIPE_GITHUB_REPO`](/zh-CN/docs/guides/environment-variables)

**示例输出**

*默认文本格式：*
```bash
$ goose recipe list
Available recipes:
goose-self-test - A comprehensive meta-testing recipe - local: ./goose-self-test.yaml
hello-world - A sample recipe demonstrating basic usage - local: ~/.config/goose/recipes/hello-world.yaml
job-finder - Find software engineering positions - local: ~/.config/goose/recipes/job-finder.yaml
```

*Verbose 模式：*
```bash
$ goose recipe list --verbose
Available recipes:
  goose-self-test - A comprehensive meta-testing recipe - local: ./goose-self-test.yaml
    Title: goose Self-Testing Integration Suite
    Path: ./goose-self-test.yaml
  hello-world - A sample recipe demonstrating basic usage - local: ~/.config/goose/recipes/hello-world.yaml
    Title: Hello World Recipe
    Path: /Users/username/.config/goose/recipes/hello-world.yaml
```

*用于自动化的 JSON 格式：*
```json
[
  {
    "name": "goose-self-test",
    "source": "Local",
    "path": "./goose-self-test.yaml",
    "title": "goose Self-Testing Integration Suite",
    "description": "A comprehensive meta-testing recipe"
  },
  {
    "name": "hello-world",
    "source": "GitHub",
    "path": "recipes/hello-world.yaml",
    "title": "Hello World Recipe",
    "description": "A sample recipe demonstrating basic usage"
  }
]
```

**配置 recipe 来源**

添加自定义 recipe 目录：
```bash
export GOOSE_RECIPE_PATH="/path/to/my/recipes:/path/to/team/recipes"
goose recipe list
```

配置 GitHub recipe 仓库：
```bash
export GOOSE_RECIPE_GITHUB_REPO="myorg/goose-recipes"
goose recipe list
```

更多配置方式见[环境变量指南](/zh-CN/docs/guides/environment-variables)。

**手动浏览目录（高级用法）**

如果你确实需要手动查看 recipe 目录，可以使用：

```bash
# 列出默认的全局 recipe 目录
ls ~/.config/goose/recipes/

# 列出当前项目的本地 recipes
ls .goose/recipes/

# 搜索所有 recipe 文件
find . -name "*.yaml" -path "*/recipes/*" -o -name "*.json" -path "*/recipes/*"
```

:::tip
推荐优先使用 `goose recipe list` 查找 recipe。它会自动搜索所有已配置来源，并以一致的格式输出结果。
:::

  </TabItem>
</Tabs>

## 使用已保存的 Recipes

<Tabs groupId="interface">
  <TabItem value="desktop" label="goose Desktop" default>

1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
2. 点击 `Recipes`
3. 在 Recipe Library 中找到目标 recipe
4. 选择以下操作之一：
   - 点击 `Use` 立即运行
   - 点击 `Preview` 先查看详情，再点击 **Load Recipe** 运行

  </TabItem>
  <TabItem value="cli" label="goose CLI">

定位到 recipe 文件之后，你可以直接[运行 recipe](/zh-CN/docs/guides/recipes/session-recipes) 或[在 goose Desktop 中打开它](/zh-CN/docs/guides/goose-cli-commands#recipe)。

:::tip 格式兼容性
CLI 可以直接运行 goose Desktop 保存的 recipe，无需任何转换。无论 recipe 是由 CLI 创建还是由 Desktop 保存，所有 recipe 命令都能正常使用。
:::

  </TabItem>
</Tabs>
