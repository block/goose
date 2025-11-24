---
title: Running Subrecipes In Parallel
sidebar_label: Subrecipes In Parallel
description: Run multiple subrecipes instances concurrently with real-time progress tracking
---

goose recipes can execute multiple [subrecipe](/docs/guides/recipes/subrecipes) instances concurrently using isolated worker processes. This feature enables efficient batch operations, parallel processing of different tasks, and faster completion of complex workflows.

:::warning Experimental Feature
Running subrecipes in parallel is an experimental feature in active development. Behavior and configuration may change in future releases.
:::

Here are some common use cases:

- **Monorepo build failures**: When 3 services fail in a monorepo build, use a "diagnose failure" subrecipe with each build URL to diagnose all failures in parallel
- **Document summarization**: Process a CSV file with document links by running a "summarize document" subrecipe for each link simultaneously  
- **Code analysis across repositories**: Run security, quality, and performance analysis on multiple codebases simultaneously

## How It Works

Parallel subrecipe execution uses an isolated worker system that automatically manages concurrent task execution. goose creates individual tasks for each subrecipe instance and distributes them across up to 10 concurrent workers.

| Scenario | Default Behavior | Override Options |
|----------|------------------|------------------|
| **Different subrecipes** | Sequential | Add "in parallel" to prompt |
| **Same subrecipe** with different parameters | Parallel | • Set `sequential_when_repeated: true`<br />• Add "sequentially" to prompt |

### Different Subrecipes

When running different subrecipes, goose determines the execution mode based on:
1. **Explicit user request** in the prompt ("in parallel", "sequentially") 
2. **Sequential execution by default**: Different subrecipes run one after another unless explicitly requested to run in parallel

In your prompt, you can simply mention "in parallel" in your prompt when calling different subrecipes:

```yaml
prompt: |
  run the following subrecipes in parallel:
    - use weather subrecipe to get the weather for Sydney
    - use things-to-do subrecipe to find activities in Sydney
```

### Same Subrecipe

When running the same subrecipe with different parameters, goose determines the execution mode based on:
1. **[Recipe-level configuration](#choosing-between-execution-modes)** (`sequential_when_repeated` flag) - when set to true, this forces sequential execution
2. **User request** in the prompt ("sequentially" to override default parallel behavior)
3. **Parallel execution by default**: Multiple instances of the same subrecipe run concurrently

If your prompt implies multiple executions of the same subrecipe, goose will automatically create parallel instances:

```yaml
prompt: |
  get the weather for three biggest cities in Australia
```

In this example, goose recognizes that "three biggest cities" requires running the weather subrecipe multiple times for different cities, so it executes them in parallel.

If you wanted to run them sequentially, you can just tell goose:

```yaml
prompt: |
  get the weather for three biggest cities in Australia one at a time
```

### Real-Time Progress Monitoring

When running multiple tasks in parallel from the CLI, you can track progress through a real-time dashboard that automatically appears during execution. The dashboard provides:
- **Live progress tracking**: Monitor task completion in real-time with statistics for completed, running, failed, and pending counts
- **Task details**: View unique task IDs, parameter sets, execution timing, output previews, and error information as tasks progress from Pending → Running → Completed/Failed 

## Examples

### Running Different Subrecipes in Parallel

This example runs the `weather` and `things-to-do` subrecipes in parallel:

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

### Running the Same Subrecipe in Parallel (with Different Parameters)

This example runs three instances of the `weather` subrecipe in parallel for different cities:

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

**Subrecipes:**

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

## Choosing Between Execution Modes

While parallel execution offers speed benefits, sequential execution is sometimes necessary or preferable. Here's how to decide:

**Use Sequential When:**
- Tasks modify shared resources
- Order of execution matters
- Memory or CPU constraints exist
- Debugging complex failures in parallel mode

**Use Parallel When:**
- Tasks are independent
- Faster completion is desired
- System resources can handle concurrent executions for up to 10 parallel workers
- Processing large datasets or multiple files

**Recipe-Level Configuration:**

For subrecipes that should never run in parallel, set `sequential_when_repeated: true` to override user requests:

```yaml
sub_recipes:
  - name: database-migration
    path: "./subrecipes/migrate.yaml"
    sequential_when_repeated: true  # Always sequential
```

## Learn More
Check out the [Recipes](/docs/guides/recipes) guide for more docs, tools, and resources to help you master goose recipes.