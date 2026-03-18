---
title: "并行运行 Subrecipes"
description: "介绍如何并发运行多个 subrecipe，并实时观察进度。"
---

# 并行运行 Subrecipes

goose recipes 可以通过隔离的 worker 进程并发执行多个 [subrecipe](/zh-CN/docs/guides/recipes/subrecipes) 实例。这个能力适合批处理、并行处理不同任务，以及加速复杂工作流的整体完成时间。

:::warning Experimental Feature
并行运行 subrecipes 仍是一个持续迭代中的实验功能。后续版本中，行为和配置方式仍可能调整。
:::

常见场景包括：

- **Monorepo 构建失败排查**：当一个 monorepo 里有 3 个服务同时构建失败时，可以把每个构建 URL 交给同一个“诊断失败” subrecipe，并行排查全部问题
- **文档摘要**：读取一个包含文档链接的 CSV，对每个链接同时运行“总结文档” subrecipe
- **跨仓库代码分析**：同时对多个代码库执行安全、质量或性能分析

## 工作原理

并行 subrecipe 执行基于隔离 worker 系统，会自动管理并发任务。goose 会为每个 subrecipe 实例创建独立任务，并最多分发到 10 个并发 worker 中。

| 场景 | 默认行为 | 覆盖方式 |
|---|---|---|
| **不同 subrecipe** | 串行 | 在 prompt 中加上 “in parallel” |
| **相同 subrecipe** 但参数不同 | 并行 | 设置 `sequential_when_repeated: true` 或在 prompt 中要求串行 |

### 不同 subrecipe

运行不同 subrecipe 时，goose 会根据以下因素决定执行模式：

1. prompt 中是否有明确用户要求，例如 “in parallel” 或 “sequentially”
2. 默认策略是串行：如果没有明确要求，不同 subrecipe 会一个接一个执行

如果你想让不同 subrecipe 并行运行，只要在 prompt 里明确写出即可：

```yaml
prompt: |
  run the following subrecipes in parallel:
    - use weather subrecipe to get the weather for Sydney
    - use things-to-do subrecipe to find activities in Sydney
```

### 同一个 subrecipe，不同参数

当同一个 subrecipe 会以不同参数多次执行时，goose 会根据以下顺序决定执行模式：

1. [Recipe 级配置](#choosing-between-execution-modes) `sequential_when_repeated`
2. prompt 中的用户要求，例如 “sequentially”
3. 默认并行：同一个 subrecipe 的多个实例会并发运行

如果 prompt 暗示需要多次调用同一个 subrecipe，goose 会自动创建并行实例：

```yaml
prompt: |
  get the weather for three biggest cities in Australia
```

在这个例子中，goose 会理解 “three biggest cities” 意味着要针对不同城市多次运行 weather subrecipe，因此默认并行执行。

如果你希望它们按顺序执行，只需要明确说明：

```yaml
prompt: |
  get the weather for three biggest cities in Australia one at a time
```

### 实时进度监控

当你在 CLI 中并行运行多个任务时，可以通过自动出现的实时仪表盘查看进度。这个仪表盘会提供：

- **实时进度**：显示已完成、运行中、失败和等待中的任务数量
- **任务细节**：展示任务 ID、参数、耗时、输出预览和错误信息，并随着任务从 Pending → Running → Completed/Failed 自动更新

## 示例

### 并行运行不同的 Subrecipes

下面这个例子会并行运行 `weather` 和 `things-to-do` 两个 subrecipe：

```yaml
# plan_trip.yaml
version: 1.0.0
title: Plan Your Trip
description: Get weather forecast and find things to do for your destination
instructions: You are a travel planning assistant that helps users prepare for their trips.
prompt: |
  run the following subrecipes in parallel to plan my trip:
    - use weather subrecipe to get the weather forecast for Sydney
    - use things-to-do subrecipe to find activities and attractions in Sydney
sub_recipes:
- name: weather
  path: "./subrecipes/weather.yaml"
  values:
    city: Sydney
- name: things-to-do
  path: "./subrecipes/things-to-do.yaml"
  values:
    city: Sydney
    duration: "3 days"
extensions:
- type: builtin
  name: developer
  timeout: 300
  bundled: true
```

### 并行运行相同的 Subrecipe（但参数不同）

下面这个例子会针对不同城市并行运行三个 `weather` subrecipe 实例：

```yaml
# multi_city_weather.yaml
version: 1.0.0
title: Multi-City Weather Comparison
description: Compare weather across multiple cities for trip planning
instructions: You are a travel weather specialist helping users compare conditions across cities.
prompt: |
  get the weather forecast for the three biggest cities in Australia
  to help me decide where to visit
sub_recipes:
- name: weather
  path: "./subrecipes/weather.yaml"
extensions:
- type: builtin
  name: developer
  timeout: 300
  bundled: true
```

**Subrecipes：**

<details>
  <summary>weather</summary>
    ```yaml
    # subrecipes/weather.yaml
    version: 1.0.0
    title: Find weather
    description: Get weather data for a city
    instructions: You are a weather expert. You will be given a city and you will need to return the weather data for that city.
    prompt: |
      Get the weather forecast for {{ city }} for today and the next few days.
    parameters:
      - key: city
        input_type: string
        requirement: required
        description: city name
    extensions:
      - type: stdio
        name: weather
        cmd: uvx
        args:
          - mcp_weather@latest
        timeout: 300
    ```
</details>

<details>
  <summary>things-to-do</summary>
    ```yaml
    # subrecipes/things-to-do.yaml
    version: 1.0.0
    title: Things to do in a city
    description: Find activities and attractions for travelers
    instructions: You are a local travel expert who knows the best activities, attractions, and experiences in cities around the world.
    prompt: |
      Suggest the best things to do in {{ city }} for a {{ duration }} trip.
      Include a mix of popular attractions, local experiences, and hidden gems.
      {% if weather_context %}
      Consider the weather conditions: {{ weather_context }}
      {% endif %}
    parameters:
      - key: city
        input_type: string
        requirement: required
        description: city name
      - key: duration
        input_type: string
        requirement: required
        description: trip duration (e.g., "2 days", "1 week")
      - key: weather_context
        input_type: string
        requirement: optional
        default: ""
        description: weather conditions to consider for activity recommendations
    ```
</details>

## 如何选择执行模式 {#choosing-between-execution-modes}

虽然并行执行更快，但有些时候顺序执行更安全或者更合适。

**以下情况建议用串行：**

- 多个任务会修改共享资源
- 执行顺序本身很重要
- 当前机器的内存或 CPU 资源有限
- 你正在排查并行模式下的复杂失败，希望过程更可控

**以下情况适合优先并行：**

- 各任务彼此独立
- 你更关注整体完成速度
- 系统能承载最多 10 个并发 worker
- 需要处理大批量文件、链接或数据集

**Recipe 级配置：**

如果某个 subrecipe 无论如何都不应该并行运行，可以设置 `sequential_when_repeated: true` 来覆盖用户请求：

```yaml
sub_recipes:
  - name: database-migration
    path: "./subrecipes/migrate.yaml"
    sequential_when_repeated: true
```

## 继续阅读

查看 [Recipes 指南](/zh-CN/docs/guides/recipes) 获取更多文档、工具和示例，进一步掌握 goose recipes 的用法。
