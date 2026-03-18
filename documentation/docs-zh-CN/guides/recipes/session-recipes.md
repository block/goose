---
sidebar_position: 1
title: 可分享 Recipes
description: "把 goose 会话中的工具、目标和指令打包成可一键启动的复用 recipe。"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, ChefHat, SquarePen, Link, Clock, Terminal, Share2 } from 'lucide-react';
import RecipeFields from '@site/src/components/RecipeFields';

有时你在 goose 里完成一个任务后，会意识到：“这套配置以后肯定还会再用。” 也许你已经搭好了合适的工具组合、定义了清晰目标，想把整个流程保存下来；也可能你想让别人复现你刚完成的工作，而不用再一步步口头指导。

你可以把当前 goose 会话打包成一个可复用的 recipe。它会把当前使用的工具、目标和启动配置保存下来，变成一个新 Agent，其他人或未来的你都可以一键启动。

## 创建 Recipe {#create-recipe}

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

  你可以从当前会话创建 recipe，也可以从模板新建 recipe。

  <Tabs>
    <TabItem value="session" label="当前会话" default>
      1. 在你想保存为 recipe 的会话中，点击应用底部的 <ChefHat className="inline" size={16} /> 按钮
      2. 在弹出的对话框中，按需检查并编辑 recipe 字段：
         <RecipeFields />
      3. 完成后，你可以：
         - 点击 `Create Recipe`，将 recipe 保存到 Recipe Library
         - 点击 `Create & Run Recipe`，保存后立刻在新会话中运行
    </TabItem>
    <TabItem value="new" label="模板">
      1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
      2. 在侧边栏里点击 `Recipes`
      3. 点击 `Create Recipe`
      4. 在弹出的对话框中按需填写 recipe 字段：
         <RecipeFields />
      5. 完成后，你可以：
         - 复制 recipe 链接，分享给其他人
         - 点击 `Save Recipe`，保存到 Recipe Library
         - 点击 `Save & Run Recipe`，保存后立刻在新会话中运行
    </TabItem>
  </Tabs>

   :::warning
   你不能基于一个“已由 recipe 启动的会话”再次创建 recipe，但你仍然可以[编辑该 recipe](#edit-recipe)。
   :::

  </TabItem>

  <TabItem value="cli" label="goose CLI">
   Recipe 文件可以是 JSON（`.json`）或 YAML（`.yaml`）格式。处于某个[会话](/zh-CN/docs/guides/sessions/session-management#start-session)中时，执行下面的命令，会在当前目录生成 `recipe.yaml`：

   ```sh
   /recipe
   ```

   如果你想指定其他文件名，可以把它作为参数传入：

   ```sh
   /recipe my-custom-recipe.yaml
   ```

   <details>
   <summary>recipe 文件结构</summary>

   ```yaml
   # Required fields
   version: 1.0.0
   title: $title
   description: $description
   instructions: $instructions    # Define the model's behavior

   # Optional fields
   prompt: $prompt                # Initial message to start with
   extensions:                    # Tools the recipe needs
   - $extensions
   activities:                    # Example prompts to display in the Desktop app
   - $activities
   settings:                      # Additional settings
     goose_provider: $provider    # Provider to use for this recipe
     goose_model: $model          # Specific model to use for this recipe
     temperature: $temperature    # Model temperature setting for this recipe (0.0 to 1.0)
   retry:                         # Automated retry logic with success validation
     max_retries: $max_retries    # Maximum number of retry attempts
     checks:                      # Success validation checks
     - type: shell
       command: $validation_command
     on_failure: $cleanup_command # Optional cleanup command on failure
   ```
   </details>

    如果你想查看每个 recipe 字段的详细说明和配置示例，请阅读[Recipe 参考指南](/zh-CN/docs/guides/recipes/recipe-reference)。

   :::warning
   你不能从一个已由 recipe 启动的会话中再创建 recipe，`/recipe` 命令不会生效。
   :::

   :::tip 先校验你的 Recipe
   建议先[校验 recipe](#validate-recipe)，确认它完整且格式正确。
   :::

   #### 可选参数

   你可以给 recipe 加上参数，让运行者在启动时填写具体值。参数可用于 recipe 的任意位置，例如 instructions、prompt、activities 等。

   使用方式：
   1. 在 recipe 内容中通过 `{{ variable_name }}` 声明模板变量
   2. 在 YAML 的 `parameters` 段中定义每个变量

   <details>
   <summary>带参数的 recipe 示例</summary>

   ```yaml
   version: 1.0.0
   title: "{{ project_name }} Code Review" # Wrap the value in quotes if it starts with template syntax to avoid YAML parsing errors
   description: Automated code review for {{ project_name }} with {{ language }} focus
   instructions: You are a code reviewer specialized in {{ language }} development.
   prompt: |
      Apply the following standards:
      - Complexity threshold: {{ complexity_threshold }}
      - Required test coverage: {{ test_coverage }}%
      - Style guide: {{ style_guide }}
   activities:
   - "Review {{ language }} code for complexity"
   - "Check test coverage against {{ test_coverage }}% requirement"
   - "Verify {{ style_guide }} compliance"
   settings:                     
     goose_provider: "anthropic"   
     goose_model: "claude-3-7-sonnet-latest"          
     temperature: 0.7 
   parameters:
   - key: project_name
     input_type: string
     requirement: required # could be required, optional or user_prompt
     description: name of the project
   - key: language
     input_type: string
     requirement: required
     description: language of the code
   - key: complexity_threshold
     input_type: number
     requirement: optional
     default: 20 # default is required for optional parameters
     description: a threshold that defines the maximum allowed complexity
   - key: test_coverage
     input_type: number
     requirement: optional
     default: 80
     description: the minimum test coverage threshold in percentage
   - key: style_guide
     input_type: string
     description: style guide name
     requirement: user_prompt
     # If style_guide param value is not specified in the command, user will be prompted to provide a value, even in non-interactive mode
   ```
   </details>

   更多字段说明见[Recipe 参考指南](/zh-CN/docs/guides/recipes/recipe-reference)。

  </TabItem>


  <TabItem value="generator" label="Recipe Generator">
    你也可以使用在线的 [Recipe Generator](https://block.github.io/goose/recipe-generator) 生成 recipe。首先选择输出格式：

    - **URL 格式**：生成一个可分享链接，可直接在 goose Desktop 中打开会话
    - **YAML 格式**：生成 YAML 内容，保存到文件后可由 goose CLI 运行

    然后填写 recipe 表单：
      - recipe 的 **title**
      - **description**
      - 一组 **instructions**
      - 可选的初始 **prompt**
        - 在 Desktop 中，prompt 会显示在聊天输入框
        - 在 CLI 中，prompt 会作为第一次执行的消息。注意：在 headless（非交互）模式下，prompt 是必需的
      - 可选的 **activities**，用于在 Desktop 中展示快捷入口
      - 仅 YAML 格式：可选的 **author** 联系方式以及该 recipe 依赖的 **extensions**

  </TabItem>
</Tabs>

:::tip 自定义 Recipe 生成方式
你可以编辑 `recipe.md` [prompt template](/zh-CN/docs/guides/prompt-templates)，来自定义 goose 生成 recipe 的方式。
:::

## 编辑 Recipe {#edit-recipe}
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

   1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
   2. 在侧边栏里点击 `Recipes`
   3. 找到你要编辑的 recipe，并点击 <SquarePen className="inline" size={16} /> 按钮
   4. 在弹出的对话框中，编辑以下任意字段：
      <RecipeFields />
   5. 完成后，你可以：
      - 复制 recipe 链接，分享给其他人
      - 点击 `Save Recipe` 保存修改
      - 点击 `Save & Run Recipe` 保存后立即在新会话中运行

  :::tip 编辑正在使用的 Recipe
  如果你当前就在一个 recipe 会话中，也可以直接点击应用底部的 <ChefHat className="inline" size={16} /> 按钮打开编辑对话框。这个按钮会在你发出第一条消息后出现。
  :::
   
  </TabItem>

  <TabItem value="cli" label="goose CLI">
  生成 recipe 文件后，直接用你熟悉的文本编辑器打开它，修改任意字段即可。

</TabItem> 
</Tabs>

## 使用 Recipe {#use-recipe}

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

  1. 通过直接链接、手动输入 URL，或者在 Recipe Library 中打开 recipe：

     **直接链接：**

         1. 点击别人分享给你的 recipe 链接

     **手动输入 URL：**

         1. 把 recipe 链接粘贴到浏览器地址栏
         2. 按 `Enter`，再点击 `Open Goose.app` 提示
       
     **Recipe Library：**

         1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
         2. 在侧边栏里点击 `Recipes`
         3. 在 Recipe Library 中找到目标 recipe
         4. 点击 `Use` 打开

     **Slash Command：**

         1. 在任意 goose 聊天会话中输入一个[自定义 slash command](/zh-CN/docs/guides/context-engineering/slash-commands)

  2. 第一次运行某个 recipe 时，系统会显示一个警告对话框，展示 recipe 的标题、描述和 instructions 供你确认。如果你信任该 recipe 的内容，点击 `Trust and Execute` 继续。之后只要 recipe 内容不变，就不会再次提示。

  3. 如果 recipe 定义了参数，在 `Recipe Parameters` 对话框中输入参数值，然后点击 `Start Recipe`。
  
     Recipe 参数是用于定制 recipe 行为的动态值：

     - **必填参数** 会用红色星号（*）标记
     - **可选参数** 会显示可修改的默认值

  4. recipe 会自动提交并开始执行。如果 recipe 包含 [prompt](#core-components)，它会作为第一条消息直接发送；如果没有 prompt，你可以点击 activity 气泡，或自行发送一条 prompt 来开始。

  :::info 隐私与隔离
  - 每个使用者都会得到自己的私有会话
  - 用户之间不会共享数据
  - 你的会话不会影响原始 recipe 创建者的会话
  :::
  </TabItem>

  <TabItem value="cli" label="goose CLI">

  在 goose CLI 中使用 recipe，通常包含以下几个任务：
  - 配置 recipe 存放位置
  - 运行 recipe
  - 定时执行 recipe

   <span id="configure-recipe-location" />

   #### 配置 Recipe 存放位置

  Recipes 可以存放在本地设备上，也可以存放在 GitHub 仓库中。你可以使用 `goose configure` 命令，或者通过[配置文件](/zh-CN/docs/guides/config-files)来设置 recipe 仓库。

  :::tip 仓库结构建议
  - 每个 recipe 单独放在自己的目录中
  - 目录名应与命令中使用的 recipe 名称一致
  - recipe 文件可以是 `recipe.yaml` 或 `recipe.json`
  :::

   <Tabs>
     <TabItem value="configure" label="使用 goose configure" default>

       运行配置命令：
       ```sh
       goose configure
       ```

       你会看到类似下面的提示：

       ```sh
       ┌  goose-configure 
       │
       ◆  What would you like to configure?
       │  ○ Configure Providers 
       │  ○ Add Extension 
       │  ○ Toggle Extensions 
       │  ○ Remove Extension 
       // highlight-start
       │  ● goose settings (Set the goose mode, Tool Output, Tool Permissions, Experiment, goose recipe github repo and more)
       // highlight-end
       │
       ◇  What would you like to configure?
       │  goose settings 
       │
       ◆  What setting would you like to configure?
       │  ○ goose mode 
       │  ○ Tool Permission 
       │  ○ Tool Output 
       │  ○ Toggle Experiment 
       // highlight-start
       │  ● goose recipe github repo (goose will pull recipes from this repo if not found locally.)
       // highlight-end
       └  
       ┌  goose-configure 
       │
       ◇  What would you like to configure?
       │  goose settings 
       │
       ◇  What setting would you like to configure?
       │  goose recipe github repo 
       │
       ◆  Enter your goose recipe GitHub repo (owner/repo): eg: my_org/goose-recipes
       // highlight-start
       │  squareup/goose-recipes (default)
       // highlight-end
       └  
       ```

     </TabItem>

     <TabItem value="config" label="使用配置文件">

       把下面配置写入你的配置文件：
       ```yaml title="~/.config/goose/config.yaml"
       GOOSE_RECIPE_GITHUB_REPO: "owner/repo"
       ```

     </TabItem>
   </Tabs>

   <span id="run-a-recipe" />

   #### 运行 Recipe

   <Tabs groupId="interface">
     <TabItem value="local" label="本地 Recipe" default>

       **基本用法**：运行一次后退出（更多选项见 [run options](/zh-CN/docs/guides/goose-cli-commands#run-options) 和 [recipe commands](/zh-CN/docs/guides/goose-cli-commands#recipe)）
       ```sh
       # 使用当前目录或 [`GOOSE_RECIPE_PATH`](/zh-CN/docs/guides/environment-variables) 中的 recipe 文件
       goose run --recipe recipe.yaml

       # 使用完整路径
       goose run --recipe ./recipes/my-recipe.yaml
       ```

       **预览 Recipe**：运行前可使用 [`explain`](/zh-CN/docs/guides/goose-cli-commands#run-options) 查看详情
 
       **交互模式**：启动一个交互式会话
       ```sh
       goose run --recipe recipe.yaml --interactive
       ```
       交互模式会提示输入必填参数：
       ```sh
       ◆ Enter value for required parameter 'language':
       │ Python
       │
       ◆ Enter value for required parameter 'style_guide':
       │ PEP8
       ```

       **带参数运行**：运行 recipe 时直接传参。详细选项见 [`run` 命令文档](/zh-CN/docs/guides/goose-cli-commands#run-options)。

       基本示例：
       ```sh
       goose run --recipe recipe.yaml --params language=Python
       ```

       **Slash Command**：在任意 goose 会话里输入一个[自定义 slash command](/zh-CN/docs/guides/context-engineering/slash-commands)

     </TabItem>

     <TabItem value="github" label="GitHub Recipe">

       配好 GitHub recipe 仓库后，就可以直接用 recipe 名称运行：

       **基本用法**：从配置好的仓库中按目录名运行 recipe（更多选项见 [run options](/zh-CN/docs/guides/goose-cli-commands#run-options) 和 [recipe commands](/zh-CN/docs/guides/goose-cli-commands#recipe)）

       ```sh
       goose run --recipe recipe-name
       ```

       例如，如果你的仓库结构如下：
       ```
       my-repo/
       ├── code-review/
       │   └── recipe.yaml
       └── setup-project/
           └── recipe.yaml
       ```
       
       那么要运行 code review 这个 recipe，对应命令就是：
       ```sh
       goose run --recipe code-review
       ```

      **预览 Recipe**：运行前可用 [`explain`](/zh-CN/docs/guides/goose-cli-commands#run-options) 查看详情

       **交互模式**：运行时提示输入参数
       ```sh
       goose run --recipe code-review --interactive
       ```
       交互模式会提示你输入必填参数：
       ```sh
       ◆ Enter value for required parameter 'project_name':
       │ MyProject
       │
       ◆ Enter value for required parameter 'language':
       │ Python
       ```

       **带参数运行**：同样可以在运行时直接传参，详细写法见 [`run` 命令文档](/zh-CN/docs/guides/goose-cli-commands#run-options)。

     </TabItem>
   </Tabs>
  :::info 隐私、隔离与 Secrets
  - 每个使用者都会拥有自己的私有会话
  - 数据不会在用户之间共享
  - 你的会话不会影响 recipe 创建者原本的会话
  - CLI 可以在运行时提示用户输入必需的 [extension secrets](/zh-CN/docs/guides/recipes/recipe-reference#extension-secrets)
  :::

   </TabItem>
</Tabs>

## 校验 Recipe {#validate-recipe}

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    Recipe 校验目前只通过 CLI 提供。
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    使用校验命令确认 recipe 配置正确。校验会验证：
    - 所有必填字段都已填写
    - 参数格式正确
    - 被引用的 extensions 存在且合法
    - YAML / JSON 语法正确

   ```sh
   goose recipe validate recipe.yaml
   ```

   :::info
   如果你想校验刚刚创建的 recipe，需要先[退出当前会话](/zh-CN/docs/guides/sessions/session-management#exit-session)，再运行 [`validate` 子命令](/zh-CN/docs/guides/goose-cli-commands#recipe)。
   :::

   Recipe 校验适用于：
    - 排查 recipe 无法按预期运行的问题
    - 手工编辑后做一次确认
    - 在 CI/CD 流水线里做自动测试

  </TabItem>
</Tabs>

## 分享 Recipe {#share-recipe}
通过 recipe 链接或 recipe 文件，你可以把自己的 recipe 分享给其他 goose 用户。

:::info 隐私与隔离
每位接收者在使用共享 recipe 时，都会启动自己的私有会话。用户之间不共享数据，你原本的会话和 recipe 也不会受影响。
:::

### 通过 Recipe Link 分享 {#share-via-recipe-link}
你可以通过 recipe link 把 recipe 分享给 Desktop 用户。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    从 Recipe Library 复制 deeplink 并分享给他人：
    1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
    2. 在侧边栏里点击 `Recipes`
    3. 找到要分享的 recipe，点击 <Link className="inline" size={16} /> 按钮复制链接

  </TabItem>
  <TabItem value="cli" label="goose CLI">
    你也可以从 recipe 文件生成一个 deeplink：
    ```sh
    goose recipe deeplink <FILE>
    ```

    还可以提前带上参数，用于预填 `Recipe Parameters` 对话框：
    ```sh
    goose recipe deeplink <FILE> --param key1=value1 --param key2=value2
    ```
  </TabItem>
</Tabs>

当别人点击这个链接时，goose Desktop 会使用你的 recipe 配置打开一个新会话。对方也可以把这个链接用于[导入 recipe](/zh-CN/docs/guides/recipes/storing-recipes#importing-recipes)，以便未来重复使用。

### 通过 Recipe 文件分享
你也可以直接把 recipe 文件发给 Desktop 或 CLI 用户。

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

  在 goose Desktop 中，你可以导出 recipe 文件，或者复制 recipe 的 YAML 内容后分享给别人。

  1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
  2. 在侧边栏里点击 `Recipes`
  3. 找到要分享的 recipe，点击 <Share2 className="inline" size={16} /> 按钮
  4. 选择分享方式：
     - 下载为 `.yaml` 文件：选择 `Export to File`，选择保存位置后点击 `Save`
     - 复制 YAML 到剪贴板：选择 `Copy YAML`

  其他 Desktop 用户可以把它[导入 Recipe Library](/zh-CN/docs/guides/recipes/storing-recipes#importing-recipes)。

  </TabItem>
  <TabItem value="cli" label="goose CLI">

  导出或复制 recipe 内容本身只在 Desktop 中支持，但 CLI 用户可以直接复制本地 recipe 文件。

  CLI 用户可以通过 `goose run --recipe <FILE>` 直接运行共享的 recipe 文件，也可以用 `goose recipe open <FILE>` 直接在 goose Desktop 中打开。详见 [CLI Commands 指南](/zh-CN/docs/guides/goose-cli-commands#recipe)。

  </TabItem>
</Tabs>

## 定时执行 Recipe {#schedule-recipe}
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
Automate goose recipes by running them on a schedule. When creating a schedule, you'll configure:
- **Name**：调度任务的名称
- **Source**：要运行的 recipe 来源
- **Execution mode**：后台运行（无窗口、保存结果）或前台运行（若 goose Desktop 正在运行则打开窗口，否则回退到后台）
- **Frequency and time**：运行频率与时间（例如每 20 分钟一次，或每周五上午 10 点执行）。你的选择最终会被转换成 goose 使用的 [cron expression](https://en.wikipedia.org/wiki/Cron#Cron_expression)

**从 Recipe Library 创建调度：**

   1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
   2. 在侧边栏里点击 `Recipes`
   3. 找到要定时运行的 recipe，点击 <Clock className="inline" size={16} /> 按钮
   4. 点击 `Create Schedule`
   5. 在弹出的对话框中配置调度。`Source` 会自动填入当前 recipe 的链接
   6. 点击 `Create Schedule`

**从 Scheduler 页面创建调度：**

   1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
   2. 点击 `Scheduler`
   3. 点击 `Create Schedule`
   4. 在弹出的对话框中配置调度。`Source` 可选择 `.yaml` / `.yml` 文件，或填写[recipe link](#share-recipe)
   5. 点击 `Create Schedule`

**管理已调度的 Recipes**

所有定时 recipes 都会列在 `Scheduler` 页面中。
点击某条 schedule 可以查看详情、上次运行时间，并执行以下操作：
- `Run Schedule Now`：立即手动触发一次
- `Edit Schedule`：修改调度频率
- `Pause Schedule`：暂停自动运行

在 `Schedule Details` 页底部，还能查看该调度创建出的会话列表，并打开或恢复这些会话。

  </TabItem>
  <TabItem value="cli" label="goose CLI">
  你也可以使用 [cron expression](https://en.wikipedia.org/wiki/Cron#Cron_expression) 在 CLI 中定时执行 recipe。

  ```bash
  # 添加一个每天上午 9 点运行的调度任务
  goose schedule add --schedule-id daily-report --cron "0 0 9 * * *" --recipe-source ./recipes/daily-report.yaml
  ```
  goose 支持 5 位、6 位或 7 位 cron 表达式，以满足不同精度需求，格式为：`seconds minutes hours day-of-month month day-of-week year`。

  更完整的用法请参考 [`schedule` 命令文档](/zh-CN/docs/guides/goose-cli-commands#schedule)。
</TabItem>
</Tabs>

## 核心组成 {#core-components}

一个 recipe 需要这些核心组件：

- **Instructions**：定义 agent 的行为与能力
  - 相当于 agent 的任务宣言
  - 让 agent 随时为相关任务做好准备
  - 如果没有提供 prompt，则 instructions 必填

- **Prompt**（可选）：自动开启对话
  - 没有 prompt 时，agent 会等待用户输入
  - 适合某个特定、立即开始的任务
  - 如果没有提供 instructions，则 prompt 必填

- **Activities**：显示为可点击气泡的示例任务
  - 帮助用户快速理解这个 recipe 能做什么
  - 降低上手门槛

## 高级能力

### 自动重试逻辑

Recipe 可以定义 retry 逻辑，在未满足成功条件时自动继续尝试。这尤其适合：

- **自动化工作流**：必须确保任务最终成功
- **开发任务**：例如运行测试、构建等可能需要多次尝试的场景
- **系统操作**：需要校验结果并在失败时做清理

**基础 retry 配置：**
```yaml
retry:
  max_retries: 3
  checks:
    - type: shell
      command: "test -f output.txt"  # Check if output file exists
  on_failure: "rm -f temp_files*"   # Cleanup on failure
```

**执行流程：**
1. recipe 按正常方式运行
2. 任务完成后，通过 success checks 校验结果
3. 如果校验失败且仍允许重试：
   - 先执行可选的清理命令
   - 再把 agent 状态重置回初始条件
   - 从头重新执行 recipe
4. 直到成功，或达到最大重试次数

更多 retry 配置选项和示例，请查看[Recipe 参考指南](/zh-CN/docs/guides/recipes/recipe-reference#retry)。

### 面向自动化的结构化输出 {#structured-output-for-automation}

Recipes 可以强制输出[结构化 JSON](/zh-CN/docs/guides/recipes/recipe-reference#response)，这让它们非常适合需要可靠解析 agent 结果的自动化工作流。主要优点包括：

- **稳定可解析**：适合脚本、自动化任务和 CI/CD 流水线
- **内建校验**：保证输出满足你的 schema 要求
- **方便抽取**：最终输出会作为单独一行，便于程序读取

结构化输出特别适合：
- **开发工作流**：代码分析报告、测试通过/失败计数、构建状态与发布就绪标记
- **数据处理**：带计数与校验结果的数据输出、结构化内容分析结果
- **文档生成**：带稳定 metadata 的项目报告，便于后续加工

**结构化输出配置示例：**
```yaml
response:
  json_schema:
    type: object
    properties:
      build_status:
        type: string
        enum: ["success", "failed", "warning"]
        description: "Overall build result"
      tests_passed:
        type: number
        description: "Number of tests that passed"
      tests_failed:
        type: number
        description: "Number of tests that failed"
      artifacts:
        type: array
        items:
          type: string
        description: "Generated build artifacts"
      deployment_ready:
        type: boolean
        description: "Whether the build is ready for deployment"
    required:
      - build_status
      - tests_passed
      - tests_failed
      - deployment_ready
```

**执行流程：**
1. recipe 正常运行
2. goose 调用 `final_output` 工具，并提交符合 schema 的 JSON
3. 系统按 JSON schema 校验输出
4. 如果校验失败，goose 会收到错误详情并修正输出
5. 校验通过后的最终 JSON 会作为最后一行输出，方便脚本抽取

**自动化示例：**
```bash
# 运行 recipe 并提取 JSON 输出
goose run --recipe analysis.yaml --params project_path=./src > output.log
RESULT=$(tail -n 1 output.log)
echo "Analysis Status: $(echo $RESULT | jq -r '.build_status')"
echo "Issues Found: $(echo $RESULT | jq -r '.tests_failed')"
```

:::info
结构化输出既支持通过 goose CLI 运行的 recipe，也支持在 goose Desktop 中运行的 recipe。但 `json_schema` 的创建和编辑目前仍需要手动修改 recipe 文件。
:::

## Recipe 会打包哪些内容

一个 recipe 会包含：

- AI instructions（目标 / 职责）
- 推荐 activities（用户可直接点击的示例任务）
- 已启用的 extensions 及其配置
- 项目目录或文件上下文
- 初始配置（但不包含完整对话历史）
- 运行该 recipe 时使用的 model 和 provider（可选）
- retry 逻辑与成功校验配置（如果有）

为了保护隐私和系统完整性，goose 不会打包以下内容：

- 全局和本地 memory
- API keys 与个人凭据
- 系统级 goose 设置

这意味着，如果某个 recipe 依赖这些内容，接收者需要自行补充对应凭据或 memory 上下文。

## 了解更多
继续阅读[Recipes 指南](/zh-CN/docs/guides/recipes)，查看更多文档、工具和资源，进一步掌握 goose recipes。
