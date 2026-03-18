---
sidebar_position: 87
title: 开工前先创建计划
sidebar_label: Creating Plans
---

import ContentCardCarousel from '@site/src/components/ContentCardCarousel';
import blogBanner from '@site/blog/2025-12-19-does-your-ai-agent-need-a-plan/blog-banner.png';

在没有清晰计划的情况下直接开始项目，就像盖房子却没有蓝图，通常会导致：

* 不知道下一步该做什么
* 浪费时间和精力
* 项目规模失控

一个好的计划能让所有参与者保持同步，也更容易衡量进度。这也是为什么 goose CLI 提供了 `/plan` 这个 prompt completion 命令，帮助你把项目拆解成清晰、可执行的步骤。
 
:::tip goose Desktop 中的计划
goose Desktop 没有 `plan` 这个关键字命令。如果你想让 goose Desktop 先给你做计划，需要直接写出类似这样的 prompt：

```
"Hey goose, can you create a plan to convert my CLI project into a locally hosted web page that gives me input fields for each CLI command I can run? Please don't start the actual work"
```
除非你明确要求 goose “create a plan”，否则它很可能会直接开始动手做项目。
:::

goose CLI 的计划模式是交互式的。它会先通过澄清问题理解你的项目，然后再生成计划。如果你能为这些问题提供有信息量、经过思考的答案，goose 通常能生成非常有用而且真正可执行的计划。

## 设置 planner provider 和 model
在一些工作流中，规划和执行使用不同 LLM 会更有效。例如，GPT-4.1 更擅长策略性规划和把复杂任务拆成清晰逻辑步骤；而 Claude Sonnet 3.5 更擅长输出干净、高效的代码并精确遵循要求。让 GPT-4.1 做规划、Claude 负责执行，往往能让结果更好。

goose CLI 的计划模式主要使用两个配置值：

- `GOOSE_PLANNER_PROVIDER`：规划阶段使用哪个 provider
- `GOOSE_PLANNER_MODEL`：规划阶段使用哪个 model

:::tip Plan 模式之外的多模型方案
goose 也支持自动模型切换的 [Lead/Worker mode](/zh-CN/docs/guides/environment-variables)，可以在两个模型之间按回合切换，在成本、速度和能力之间做平衡。
:::

:::tip 自定义计划格式
如果你想调整 goose 生成计划的格式，也可以直接编辑 `plan.md` [prompt template](/zh-CN/docs/guides/prompt-templates)。
:::

### 设置 goose planner 环境变量
你可以把下面这些配置写入 shell 配置文件（例如 `.bashrc`）：
```bash
export GOOSE_PLANNER_PROVIDER=<my-chosen-provider>
export GOOSE_PLANNER_MODEL=<my-chosen-model>
```
保存后，需要重新启动 goose 会话，新的变量才会生效。

如果这两个变量没有设置，goose 会回退使用默认的 provider 和 model。只有当你明确发现某些模型更擅长拆分任务时，才有必要为 planner 单独指定模型；否则默认配置通常已经足够。

要验证 planner provider 是否已生效，可以运行：

```bash
goose info -v
```

下面是一个示例输出，显示当前配置和配置文件路径：

```bash
goose Version:
  Version:          1.0.18

goose Locations:
  Config file:      /Users/alincoln/.config/goose/config.yaml
  Sessions dir:     /Users/alincoln/.local/share/goose/sessions
  Logs dir:         /Users/alincoln/.local/state/goose/logs

goose Configuration:
  GOOSE_PROVIDER: anthropic
  GOOSE_MODEL: claude-3.5-sonnet
  GOOSE_PLANNER_PROVIDER: openai
  GOOSE_MODE: smart_approve
  GOOSE_PLANNER_MODEL: gpt-4.1
```

如果 `GOOSE_PLANNER_PROVIDER` 或 `GOOSE_PLANNER_MODEL` 任意一个没有设置，goose 会退回使用 `GOOSE_PROVIDER` 和 `GOOSE_MODEL` 来生成计划。

## 描述你的项目
虽然 goose 可以处理复杂的项目描述，但它最擅长的仍然是清晰、简洁的项目说明。重点是讲清楚项目目的和预期结果。如果这些信息还不够清楚，goose 会不断追问，直到它真正理解你的目标。

## 一个简单的建房计划示例
即使是相对简单的项目，goose 也能产出不错的计划，比如下面这个建房示例：

```bash
( O)> /plan

Entering plan mode. You can provide instructions to create a plan and then act on it. To exit early, type /endplan

( O)> Build a four bedroom house
◓  Measuring semantic distance...                                                                                                         

I need to gather more information before creating a detailed house-building plan. Please provide clarification on the following:

1. What is the purpose of this plan? Are you looking for architectural designs, construction steps, or a project management timeline?
2. What is your budget range for this four-bedroom house?
3. What is the approximate square footage or size requirements?
4. Do you have a specific location or lot already selected? What are the dimensions and any zoning restrictions?
5. Are there any specific architectural styles or features you want included?
6. How many bathrooms and other rooms (e.g., kitchen, living room, dining room, garage) do you want?
7. Do you have any specific sustainability or energy efficiency requirements?
8. What is your timeline for completion?

( O)> 1 construction steps and a project management timeline. 2 $600,000. 3 4,000 sq feet. 4 the lot is not yet selected. 5 one story rambler. 6 four bedrooms, two ful baths, a kitchen, diningroom, family room, and garage. 7 the house must be efficient and powered by solar energy. 8 complete in six months. 
◓  Taming tensors...                                                                                                          I'll create a detailed plan for building a 4,000 sq ft, one-story rambler with four bedrooms, two full bathrooms, kitchen, dining room, family room, garage, and home office. The house will be energy efficient with solar power, with a $600,000 budget and 6-month timeline.

◇  Do you want to clear message history & act on this plan?
│  Yes 
│

Exiting plan mode and acting on the above plan

### Introduction to Building a 4,000 Sq Ft Rambler
To create a detailed plan for your 4,000 sq ft, one-story rambler, we need to consider several factors including design, energy efficiency, budget, and timeline. Given your requirements, here's an overview of how to approach this project:

### Step 1: Design and Planning
- **Architectural Design**: Hire an architect to design the house, ensuring it meets your space requirements and is energy efficient. Consider factors like natural lighting, insulation, and window placement.
- **Floor Plan**: Create a floor plan that includes four bedrooms, two full bathrooms, a kitchen, dining room, family room, garage, and home office. Ensure the layout is functional and spacious.
- **Land Preparation**: Ensure the building site is prepared, including clearing the land, grading, and preparing the foundation.

### Step 2: Energy Efficiency and Solar Power
- **Solar Power System**: Design and install a solar power system that can meet the energy needs of a 4,000 sq ft house. This may include a large solar panel array and a battery storage system for nighttime and cloudy days.
- **Insulation and Windows**: Use high-quality insulation in the walls, floor, and ceiling to minimize heat loss. Install energy-efficient windows that allow natural light in while keeping the heat or cold out.
- **Energy-Efficient Appliances**: Specify energy-efficient appliances and lighting to minimize energy consumption.

### Step 3: Budgeting
- **Construction Costs**: Allocate approximately $200-250 per sq ft for construction, which translates to $800,000-$1,000,000 for a 4,000 sq ft house. However, your budget is $600,000, so you'll need to optimize costs without compromising on quality and energy efficiency.
- **Solar Power System Costs**: Budget an additional $20,000 to $50,000 for the solar power system, depending on its size and complexity.
- **Contingency Fund**: Set aside 10-20% of the budget for unexpected expenses.

### Step 4: Timeline
- **Month 1-2: Design and Planning**: Complete the architectural design, obtain necessary permits, and prepare the building site.
- **Month 3-4: Foundation and Framing**: Lay the foundation, frame the house, and install the roof.
- **Month 5: Installation of Electrical, Plumbing, and HVAC Systems**: Install the solar power system, electrical, plumbing, and HVAC systems.
- **Month 6: Finishing Work**: Complete the interior finishing work, including installation of energy-efficient appliances, flooring, and fixtures.

### Conclusion
Building a 4,000 sq ft, energy-efficient rambler with solar power within a $600,000 budget and 6-month timeline is challenging but feasible with careful planning and design. It's essential to work closely with your architect, contractor, and solar power system installer to ensure the project stays on track and within budget.

Given that no extensions are currently available for more specific tools or data sources, consider the following steps to proceed with your project:

* Search for available extensions using the <function=platform__search_available_extensions>{}</function>

```
这个建房计划之所以仍然偏高层，是因为 goose 当前模型更擅长技术和软件开发，而不是建筑施工。因此，像“做一个 Web 应用”这样的项目，通常会得到更具体、更细的计划。

### 为子步骤创建单独计划
继续看建房示例。虽然计划里包含了“雇佣建筑师”这一步，但这个高层步骤本身还需要更细的拆解，例如要找什么类型的建筑师、如何筛选、如何推进选择流程。

```
- **Architectural Design**: Hire an architect to design the house, ensuring it meets your space requirements and is energy efficient. Consider factors like natural lighting, insulation, and window placement.
```
如果你在审查建房计划时退出了计划模式，也完全可以重新进入，让 goose 继续协助你推进。

```
( O)> /plan hire an architect

Entering plan mode. You can provide instructions to create a plan and then act on it. To exit early, type /endplan

◓  Synchronizing flock algorithms...                                                                                                       I need some clarifying questions to better understand your request about hiring an architect:

1. What is your location or preferred region for the architect to be based in?
2. Do you have a specific budget range for architectural services?
3. Are you looking for an architect with specific expertise (e.g., energy-efficient homes, modern design, traditional styles)?
4. Do you need full architectural services (design through construction oversight) or just certain phases?
5. Do you have a timeline for when you'd like to begin working with an architect?
6. Have you already purchased land for your home, or will the architect need to help evaluate potential sites?
7. Are there any specific certifications or qualifications you're looking for in an architect?
8. Would you prefer to work with a solo practitioner or a larger architectural firm?
9. Do you have any examples of architectural styles or specific homes you admire?
10. How involved do you want to be in the design process?
```

在这些澄清问题拿到足够信息后，goose 就能继续生成一份“如何雇佣建筑师”的细化计划，并把它纳入更大的建房项目上下文中。

## 一个开发项目示例
下面是一个更贴合 goose 能力边界的例子：开发者写了一个 Python CLI，用于连接 Contentful CMS，把某些字符串搜索出来再替换成新字符串。如果把它改造成网站，这个 search / replace 功能会更容易使用，也更适合扩展更多特性。开发者希望用 goose CLI 先为这个转换项目做计划。

如果 goose 判断这个项目可能有多种实现方式、技术栈和组件组合，它就会在这些决策点上主动追问。例如，如果你这样开始：

```bash 
( O)> /plan

Entering plan mode. You can provide instructions to create a plan and then act on it. To exit early, type /endplan

( O)> Convert the CLI built by search_replace_routes.py into a web page
```
goose 会先解析你的项目描述，再结合当前配置的 LLM，如果还需要额外信息，就会启动一轮澄清问题。

## 澄清问题
把一个 Python CLI 改造成网站，看起来似乎不复杂，但 goose 仍然会询问样式、认证、功能、技术栈等关键细节。你可能会看到这样的提问：

```bash
1. Should the application support any keyboard shortcuts for common actions?
2. Would you like the application to remember user preferences (like case sensitivity setting) between sessions?
3. Should there be any form of notification when operations complete successfully?
4. How should the application handle very large text fields that might be difficult to display in the three-column layout?
5. Are there any specific CI/CD requirements for deployment to AWS?

```
你可以一题一题回答，也可以把答案一次性打包：

```bash
( O)> 1 no keyboard shortcuts. 2 do not remember preferences. 3 on success, open a dialog that says "success". 4 Truncate to 30 characters before the string to replace and then 30 characters after the string to replace.5 no CI/CD requirements 
```


:::tip
当 goose 在计划模式下要求你提供项目产物（例如源代码）时，你需要把内容直接粘贴到聊天里。只给文件路径是不够的；最简单的做法是把文件内容复制出来，并加一句类似 “Here’s the requested code:” 的说明。
:::

如果你一次回答多道问题，记得按题号组织回答。不要只回答一个 “no” 或 “don’t remember”，而是写成像 “2. Do not store my preferences.” 这样的形式。这样 goose 才能准确知道哪一题已经回答过，避免重复追问。

在这种复杂项目中，goose 可能会发起多轮澄清问题。每一轮通常都是因为你上一轮回答带来了新的上下文，或者还缺少某一块关键决策信息。

如果你已经回答了 _全部_ 问题，goose 就会开始生成计划。另一种常见情况是，你可能会觉得 goose 永远问不完。如果你此时只想先拿到一版计划，也可以直接要求它给出一份 “generic” plan：

```bash
I still need some critical information to create a comprehensive plan:

1. What specific functionality does your Python script perform with Contentful? Could you share the actual code or describe how it interacts with the Contentful API to search and replace content?
2. What are the 5 predefined keys/inputs you mentioned earlier?
3. Should the web application include all the command-line options from the script (environment, status, case-sensitive, partial-match, dry-run)?
4. How should the application handle errors from the Contentful API?
5. Should the application maintain any history of previous search/replace operations?

Without this information, I can only provide a generic plan that might not accurately capture your requirements.
( O)> please provide a generic plan
```
虽然 goose 生成的计划有一套标准结构，但里面的内容会根据你的回答做定制。更重要的是，goose 后续还能根据计划生成实现代码。你应该在退出计划模式（`/endplan`）并要求 goose 开始实现之前，先认真审查这份计划以及它生成的代码。

下面是一份针对这个项目的示例计划，省略了实际生成的网站代码：

```bash
# Plan for Converting CLI Script to React Web Application

## Step 1: Set up the React project and dependencies
1. Create a new React application using Create React App
2. Install necessary dependencies
3. Set up project structure
   src/
   ├── components/
   │   ├── Auth/
   │   ├── Layout/
   │   ├── Search/
   │   ├── Results/
   │   └── History/
   ├── services/
   │   ├── contentful.ts
   │   ├── auth.ts
   │   └── storage.ts
   ├── utils/
   │   ├── validation.ts
   │   └── helpers.ts
   ├── contexts/
   │   └── AppContext.tsx
   └── pages/
       ├── Home.tsx
       ├── Search.tsx
       └── History.tsx

## Step 2: Implement authentication with Okta
1. Create an Okta application in the Okta Developer Console
2. Configure the Okta authentication settings in the application
3. Implement protected routes using Okta's React components
## Step 3: Create the layout and UI components with Material UI
1. Implement the main layout with Material UI
2. Create the search form component
3. Create the results display component with three columns
4. Implement the "Load More" pagination component
## Step 4: Implement Contentful service
1. Create a service for interacting with Contentful
## Step 5: Implement local storage service for history
1. Create a service for managing search history in localStorage
## Step 6: Implement validation utilities
1. Create utility functions for input validation
## Step 7: Implement main pages
1. Create the Search page
2. Create the History page
## Step 8: Set up routing and main application
1. Create the main App component
## Step 9: Implement error handling and loading states
1. Create error boundary components
2. Add loading indicators for API operations
3. Implement error messages display

## Step 10: Set up deployment configuration
1. Create AWS deployment configuration
## Step 11: Testing and quality assurance
1. Write unit tests for key components
2. Implement integration tests for the main workflows
3. Perform manual testing of the application

This plan provides a comprehensive framework for converting your CLI script to a React web application with Material UI and Okta authentication. You'll need to adapt specific parts based on your exact requirements and the functionality of your original script.

```



## 基本用法
在把 CLI 切换到计划模式之前，你需要先有一个活动中的 goose 会话。如果你准备专门用一个会话来做计划，建议像下面这样给会话起个名字：

```bash
~ goose session -n web-project-plan
starting session | provider: databricks model: databricks-meta-llama
    session id: 20251110_5
    working directory: /Users/alincoln

goose is running! Enter your instructions, or try asking what goose can do.
```
进入计划模式时，输入 `/plan` 即可；也可以把计划描述直接跟在命令后面：
```bash
( O)> /plan  Build a four bedroom house
```

CLI 中的 plan mode 是一种特殊交互模式，goose 会在其中帮你把任务拆成更易执行的步骤。如果你想退出计划模式并回到当前活跃会话，输入 `/endplan`：

```bash
( O)> /endplan
```

## 额外资源

<ContentCardCarousel
  items={[
      {
      type: 'blog',
      title: 'Does Your AI Agent Need a Plan?',
      description: 'Knowing when and how to plan with an AI agent can produce better results.',
      thumbnailUrl: blogBanner,
      linkUrl: '/goose/blog/2025/12/19/does-your-ai-agent-need-a-plan',
      date: '2025-12-19',
      duration: '7 min read'
    },
    {
      type: 'topic',
      title: 'Planning Complex Tasks',
      description: 'Learn how to use the Plan feature to break down complex tasks into manageable, executable steps.',
      linkUrl: '/goose/zh-CN/docs/tutorials/plan-feature-devcontainer-setup',
    }
  ]}
/>
