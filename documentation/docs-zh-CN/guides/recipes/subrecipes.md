---
title: "用于专门任务的 Subrecipes"
sidebar_label: "Subrecipes"
description: "介绍 recipe 如何通过 subrecipes 拆分复杂任务，并把专门工作交给独立 recipe。"
sidebar_position: 4
---

Subrecipes 是被另一个 recipe 调用来执行特定任务的 recipe。它们能帮助你实现：

- **多步骤工作流**：把复杂任务拆成不同阶段，每个阶段由专门的 recipe 处理
- **可复用组件**：把常见任务封装成模块，在不同工作流里复用

:::warning Experimental Feature
Subrecipes 仍是一个持续开发中的实验特性，后续行为和配置方式可能继续变化。
:::

## Subrecipes 如何工作

主 recipe 会在 `sub_recipes` 字段中注册自己的 subrecipes，这个字段通常包含：

- `name`：subrecipe 的唯一标识，用于生成工具名称
- `path`：subrecipe 文件路径，可为相对路径或绝对路径
- `values`：可选，预先配置好的参数值，运行时总是传给 subrecipe

当主 recipe 运行时，goose 会为每个 subrecipe 生成一个对应工具，这个工具会：

- 接收该 subrecipe 定义的参数
- 在独立 session 中执行 subrecipe，并拥有自己的上下文
- 把执行结果返回给主 recipe

Subrecipe 的会话彼此隔离。它们不会与主 recipe 或其他 subrecipes 共享对话历史、memory 或运行状态。另外，subrecipe 不能继续定义自己的 subrecipes，也就是不支持嵌套。

## 参数处理

Subrecipe 接收到的参数可以通过 `{{ parameter_name }}` 语法在 prompt 和 instructions 中使用。Subrecipe 的参数主要有两种来源：

1. **预设值**：在 `values` 中写死的固定参数，运行时会自动提供，且不能被覆盖
2. **基于上下文的参数**：AI 可以从对话上下文中提取参数，包括前一个 subrecipe 的输出结果

如果上下文和 `values` 同时提供了同名参数，以 `values` 中的值为准。

:::tip
如果你要向 subrecipe 传递多行文本参数，建议使用 `indent()` 过滤器保持 YAML 合法，例如：`{{ content | indent(2) }}`。详见[模板支持](/zh-CN/docs/guides/recipes/recipe-reference#template-support)。
:::

## 示例

### 顺序处理

下面的 Code Review Pipeline 展示了一个主 recipe，如何通过两个 subrecipes 完成一次完整的代码评审：

**用法：**
```bash
goose run --recipe code-review-pipeline.yaml --params repository_path=/path/to/repo
```

**主 Recipe：**

```yaml
# code-review-pipeline.yaml
version: "1.0.0"
title: "Code Review Pipeline"
description: "Automated code review using subrecipes"
instructions: |
  Perform a code review using the available subrecipe tools.
  Run security analysis first, then code quality analysis.

parameters:
  - key: repository_path
    input_type: string
    requirement: required
    description: "Path to the repository to review"

sub_recipes:
  - name: "security_scan"
    path: "./subrecipes/security-analysis.yaml"
    values:
      scan_level: "comprehensive"
  
  - name: "quality_check"
    path: "./subrecipes/quality-analysis.yaml"

extensions:
  - type: builtin
    name: developer
    timeout: 300
    bundled: true

prompt: |
  Review the code at {{ repository_path }} using the subrecipe tools.
  Run security scan first, then quality analysis.
```

**Subrecipes：**

<details>
  <summary>security_scan</summary>
  ```yaml
  # subrecipes/security-analysis.yaml
  version: "1.0.0"
  title: "Security Scanner"
  description: "Analyze code for security vulnerabilities"
  instructions: |
    You are a security expert. Analyze the provided code for security issues.
    Focus on common vulnerabilities like SQL injection, XSS, and authentication flaws.

  parameters:
    - key: repository_path
      input_type: string
      requirement: required
      description: "Path to the code to analyze"
    
    - key: scan_level
      input_type: string
      requirement: optional
      default: "standard"
      description: "Depth of security scan (basic, standard, comprehensive)"

  extensions:
    - type: builtin
      name: developer
      timeout: 300
      bundled: true

  prompt: |
    Perform a {{ scan_level }} security analysis on the code at {{ repository_path }}.
    Report any security vulnerabilities found with severity levels and recommendations.
  ```
</details>

<details>
  <summary>quality_check</summary>
  ```yaml
  # subrecipes/quality-analysis.yaml
  version: "1.0.0"
  title: "Code Quality Analyzer"
  description: "Analyze code quality and best practices"
  instructions: |
    You are a code quality expert. Review code for maintainability, 
    readability, and adherence to best practices.

  parameters:
    - key: repository_path
      input_type: string
      requirement: required
      description: "Path to the code to analyze"

  extensions:
    - type: builtin
      name: developer
      timeout: 300
      bundled: true

  prompt: |
    Analyze the code quality at {{ repository_path }}.
    Check for code smells, complexity issues, and suggest improvements.
  ```
</details>

:::tip
如果多个 subrecipes 彼此独立，想让执行更快，可以参考[并行运行 Subrecipes](/zh-CN/docs/tutorials/subrecipes-in-parallel)。
:::

### 条件处理

下面的 Smart Project Analyzer 演示了如何先分析项目类型，再根据结果决定使用哪个 subrecipe：

**用法：**
```bash
goose run --recipe smart-analyzer.yaml --params repository_path=/path/to/project
```

**主 Recipe：**

```yaml
# smart-analyzer.yaml
version: "1.0.0"
title: "Smart Project Analyzer"
description: "Analyze project and choose appropriate processing based on type"
instructions: |
  First examine the repository to determine the project type (web app, CLI tool, library, etc.).
  Based on what you find:
  - If it's a web application, use the web_security_audit subrecipe
  - If it's a CLI tool or library, use the api_documentation subrecipe
  Only run one subrecipe based on your analysis.

parameters:
  - key: repository_path
    input_type: string
    requirement: required
    description: "Path to the repository to analyze"

sub_recipes:
  - name: "web_security_audit"
    path: "./subrecipes/web-security.yaml"
    values:
      check_cors: "true"
      check_csrf: "true"
  
  - name: "api_documentation"
    path: "./subrecipes/api-docs.yaml"
    values:
      format: "markdown"

extensions:
  - type: builtin
    name: developer
    timeout: 300
    bundled: true

prompt: |
  Analyze the project at {{ repository_path }} and determine its type.
  Then run the appropriate subrecipe tool based on your findings.
```

**Subrecipes：**

<details>
  <summary>web_security_audit</summary>
  ```yaml
  # subrecipes/web-security.yaml
  version: "1.0.0"
  title: "Web Security Auditor"
  description: "Security audit for web applications"
  instructions: |
    You are a web security specialist. Audit web applications for 
    security vulnerabilities specific to web technologies.

  parameters:
    - key: repository_path
      input_type: string
      requirement: required
      description: "Path to the web application code"
    
    - key: check_cors
      input_type: string
      requirement: optional
      default: "false"
      description: "Whether to check CORS configuration"
    
    - key: check_csrf
      input_type: string
      requirement: optional
      default: "false"
      description: "Whether to check CSRF protection"

  extensions:
    - type: builtin
      name: developer
      timeout: 300
      bundled: true

  prompt: |
    Perform a web security audit on {{ repository_path }}.
    {% if check_cors == "true" %}Check CORS configuration for security issues.{% endif %}
    {% if check_csrf == "true" %}Verify CSRF protection is properly implemented.{% endif %}
    Focus on web-specific vulnerabilities like XSS, authentication flaws, and session management.
  ```
</details>

<details>
  <summary>api_documentation</summary>
  ```yaml
  # subrecipes/api-docs.yaml
  version: "1.0.0"
  title: "API Documentation Generator"
  description: "Generate documentation for APIs and libraries"
  instructions: |
    You are a technical writer specializing in API documentation.
    Create comprehensive documentation for code libraries and APIs.

  parameters:
    - key: repository_path
      input_type: string
      requirement: required
      description: "Path to the code to document"
    
    - key: format
      input_type: string
      requirement: optional
      default: "markdown"
      description: "Output format for documentation (markdown, html, rst)"

  extensions:
    - type: builtin
      name: developer
      timeout: 300
      bundled: true

  prompt: |
    Generate {{ format }} documentation for the code at {{ repository_path }}.
    Include API endpoints, function signatures, usage examples, and installation instructions.
    Focus on making it easy for developers to understand and use this code.
  ```
</details>

### 基于上下文传递参数

下面的 Travel Planner 展示了 subrecipe 如何从对话上下文获取参数，包括前一个 subrecipe 的输出结果：

**用法：**
```bash
goose run --recipe travel-planner.yaml
```

**主 Recipe：**

```yaml
# travel-planner.yaml
version: "1.0.0"
title: "Travel Activity Planner"
description: "Get weather data and suggest appropriate activities"
instructions: |
  Plan activities by first getting weather data, then suggesting activities based on conditions.

prompt: |
  Plan activities for Sydney by first getting weather data, then suggesting activities based on the weather conditions we receive.

sub_recipes:
  - name: weather_data
    path: "./subrecipes/weather-data.yaml"
    # No values - location parameter comes from prompt context
  
  - name: activity_suggestions
    path: "./subrecipes/activity-suggestions.yaml"
    # weather_conditions parameter comes from conversation context

extensions:
  - type: builtin
    name: developer
    timeout: 300
    bundled: true
```

**Subrecipes：**

<details>
  <summary>weather_data</summary>
  ```yaml
  # subrecipes/weather-data.yaml
  version: "1.0.0"
  title: "Weather Data Collector"
  description: "Fetch current weather conditions for a location"
  instructions: |
    You are a weather data specialist. Gather current weather information
    including temperature, conditions, and seasonal context.

  parameters:
    - key: location
      input_type: string
      requirement: required
      description: "City or location to get weather data for"

  extensions:
    - type: stdio
      name: weather
      cmd: uvx
      args:
        - mcp_weather@latest
      timeout: 300
      description: "Weather data for trip planning"
    - type: builtin
      name: developer
      timeout: 300
      bundled: true

  prompt: |
    Get the current weather conditions for {{ location }}.
    Include temperature, weather conditions (sunny, rainy, etc.), 
    and any relevant seasonal information.
  ```
</details>

<details>
  <summary>activity_suggestions</summary>
  ```yaml
  # subrecipes/activity-suggestions.yaml
  version: "1.0.0"
  title: "Activity Recommender"
  description: "Suggest activities based on weather conditions"
  instructions: |
    You are a travel expert. Recommend appropriate activities and attractions
    based on current weather conditions.

  parameters:
    - key: weather_conditions
      input_type: string
      requirement: required
      description: "Current weather conditions to base recommendations on"

  extensions:
    - type: builtin
      name: developer
      timeout: 300
      bundled: true

  prompt: |
    Based on these weather conditions: {{ weather_conditions }}, 
    suggest appropriate activities, attractions, and travel tips.
    Include both indoor and outdoor options as relevant.
  ```
</details>

在这个示例里：
- `weather_data` subrecipe 从 prompt 上下文中提取地点参数，也就是 AI 从自然语言 prompt 中识别出 “Sydney”
- `activity_suggestions` subrecipe 从对话上下文中获取天气条件，也就是直接使用前一个 subrecipe 的结果

## 最佳实践
- **单一职责**：每个 subrecipe 只做一件清晰明确的事
- **参数清晰**：使用易懂的参数名和说明
- **固定值放进 `values`**：把不会变化的参数提前写死
- **先独立测试**：先确认 subrecipe 单独运行没有问题，再把它接入主 recipe

:::tip 控制 Subrecipe 执行轮数
每个 subrecipe 都可以通过 `settings.max_turns` 单独设置执行上限。如果未设置，它会继承父 recipe 的 `max_turns`。详见 [Recipe Settings](/zh-CN/docs/guides/recipes/recipe-reference#settings)。

```yaml
# subrecipes/quick-scan.yaml
version: "1.0.0"
title: "Quick Security Scan"
settings:
  max_turns: 10  # Limit this subrecipe to 10 turns
instructions: "Perform a quick security scan"
prompt: "Scan for common vulnerabilities"
```
:::

## 了解更多
继续阅读[Recipes 指南](/zh-CN/docs/guides/recipes)，查看更多文档、工具和资源，进一步掌握 goose recipes。
