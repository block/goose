---
title: "使用 goose 做基准测试"
description: "说明如何使用 `goose bench` 运行 benchmark 并分析结果。"
---

# 使用 goose 做基准测试

goose 的 benchmarking 系统允许你在复杂任务上评估不同系统配置的表现。本教程主要围绕 `goose bench` 命令，介绍如何运行 benchmark、如何组织配置文件，以及如何分析结果。

## 快速开始

1. benchmarking 系统内置了多个 evaluation suite。先运行下面的命令，查看所有可用 selector：

```bash
goose bench selectors
```

2. 创建一个基础配置文件：

```bash
goose bench init-config -n bench-config.json
cat bench-config.json
{
  "models": [
    {
      "provider": "databricks",
      "name": "goose",
      "parallel_safe": true
    }
  ],
  "evals": [
    {
      "selector": "core",
      "parallel_safe": true
    }
  ],
  "repeat": 1
}
...etc.
```

3. 运行 benchmark：

```bash
goose bench run -c bench-config.json
```

## 配置文件结构

benchmark 使用 JSON 配置文件，结构大致如下：

```json
{
  "models": [
    {
      "provider": "databricks",
      "name": "goose",
      "parallel_safe": true,
      "tool_shim": {
        "use_tool_shim": false,
        "tool_shim_model": null
      }
    }
  ],
  "evals": [
    {
      "selector": "core",
      "post_process_cmd": null,
      "parallel_safe": true
    }
  ],
  "include_dirs": [],
  "repeat": 2,
  "run_id": null,
  "eval_result_filename": "eval-results.json",
  "run_summary_filename": "run-results-summary.json",
  "env_file": null
}
```

## 关键配置项

### Models

`models` 数组中的每个条目定义一个模型配置：

- `provider`
- `name`
- `parallel_safe`
- `tool_shim`
  - `use_tool_shim`
  - `tool_shim_model`

### Evals

`evals` 数组中的每个条目定义一个 evaluation：

- `selector`
- `post_process_cmd`
- `parallel_safe`

### 通用选项

- `include_dirs`：额外纳入 benchmark 的目录
- `repeat`：每个 evaluation 重复执行的次数
- `run_id`：本次 benchmark 的可选标识
- `eval_result_filename`：详细评测结果文件名
- `run_summary_filename`：结果汇总文件名
- `env_file`：可选的环境变量文件

#### `include_dirs` 的作用机制

`include_dirs` 会把配置中列出的路径内容暴露给所有 evaluation。具体实现方式是：

- 先把每个 include 的资源复制到每个 model/provider 对应的顶层运行目录
- 真正执行某个 evaluation 时：
  - 只有该 evaluation 明确需要的资源才会被复制到 eval 对应目录
  - 前提是 evaluation 代码主动去读取这些资源
  - 同时该 evaluation 还必须被当前 selector 命中并实际执行

### 自定义评测

你还可以从几个方向自定义 benchmark 运行：

1. 评测完成后执行后处理命令：

```json
{
  "evals": [
    {
      "selector": "core",
      "post_process_cmd": "/path/to/process-script.sh",
      "parallel_safe": true
    }
  ]
}
```

2. 带入额外数据目录：

```json
{
  "include_dirs": [
    "/path/to/custom/eval/data"
  ]
}
```

3. 使用额外环境变量文件：

```json
{
  "env_file": "/path/to/env-file"
}
```

## 输出与结果

benchmark 会生成两类主要结果文件。不同 model/provider 组合的运行结果会分别写入各自目录，例如：

```text
benchmark-${datetime}/
  ${model}-${provider}[-tool-shim[-${shim-model}]]/
    run-${i}/
      ${an-include_dir-asset}
      run-results-summary.json
      core/developer/list_files/
        ${an-include_dir-asset}
        run-results-summary.json
```

- `eval-results.json`：包含每个 evaluation 的详细结果，例如：
  - 单个测试用例结果
  - 模型响应
  - 打分指标
  - 错误日志
- `run-results-summary.json`：汇总所有 eval suite 的整体结果

### Debug 模式

如果你需要更细的日志，可开启 debug：

```bash
RUST_LOG=debug goose bench bench-config.json
```

## 高级用法

### Tool Shimming

如果你想让不支持原生 tool calling 的模型也参与 benchmark，可以配合 [Ollama Tool Shim](/zh-CN/docs/experimental/ollama)。前提是系统中已经安装 Ollama。

## 适合什么时候用

如果你在做这些事情，benchmarking 很有价值：

- 对比不同 provider / model 在同一任务集上的表现
- 评估 tool shim、并行策略或上下文策略的影响
- 为团队建立一套长期可重复运行的评测基线

本质上，它把“我感觉这个模型更好”变成“在统一基准下，我们能量化说明哪个配置更适合当前任务”。
