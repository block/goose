# V1 架构与复用边界

## 核心原则

V1 必须是：

> **Goose 原生能力 + 安全产品装配层**

而不是：

> **新的 agent 平台 + 借用了一点 Goose**

补充：

- `V1a` 是 `macOS-only` 的本地预览版
- `V1a` 默认不引入产品网关
- `V1a` 默认不引入 LiteLLM
- 先直接借 Goose 的 provider 与 model 配置能力

## 复用清单

V1 必须复用 Goose 的这些机制：

- `goosed` 作为服务端主入口
- Goose session / conversation 机制
- Goose provider 机制
- Goose multi-model 机制
- Goose extensions / MCP 加载机制
- Goose skills 自动发现和自动加载
- Goose recipes / subagents / persistent instructions
- Goose memory extension / project memory
- Goose desktop 壳与会话体验

## 明确不做的事

V1 不做：

- 自建 agent scheduler
- 自建任务编排器替代 Goose recipe/subagent
- 自建 memory framework
- 自建 skills runtime
- 自建会话系统
- 自建桌面客户端内核

## 架构图

```text
Security Goose Desktop
  -> goosed
     -> Goose Core
        -> Providers
        -> Extensions / MCP
        -> Skills
        -> Recipes / Subagents
        -> Memory
```

## 为什么要这样分

## Goose Desktop / goosed

这是产品主心骨，负责：

- 用户会话
- agent 执行循环
- 工具调用
- memory
- skill 加载
- recipe / workflow

只要 V1 目标是“快”，这块就不能重造。

## Product Gateway / Remote Control Plane

这不是 V1a 的必选项，而是 `V1.5+` 的增强项。

当需要以下能力时再引入：

- 谁能用
- 能用哪个模型
- 用了多少
- 免费/付费能力差异

它不参与 agent orchestration，只参与能力访问控制和模型路由。

如果当前只是：

- 你自己在本机预览
- 先做一个可用的 Mac 客户端
- 先验证安全 skills / MCP / 模型切换

那就先不要上网关。

## 未来 AGS 缝隙

V1 不要求接入 AGS，但必须预留一条稳定缝隙：

- 本地 / 默认 provider 模式
- 网关代理模式
- 后续远端执行模式

V1 的目标不是把 AGS 全接上，而是保证：

- 日后接 AGS 时，不需要推翻 V1

## 推荐的请求路径

### V1 默认路径

```text
Desktop -> goosed -> Goose provider config -> TokenPlan-compatible endpoint
```

### V1.5 默认路径

```text
Desktop -> goosed -> Product Gateway -> LiteLLM -> TokenPlan
```

### V1 安全扩展路径

```text
Desktop -> goosed -> MCP extension / skill / recipe
```

### V2 远端高风险任务路径

```text
Desktop -> goosed -> gateway task route -> AGS runtime / sandbox
```

## 能力映射

## 安全 Skill

用 Goose `skills` 表达：

- 漏洞研判
- 告警分析
- IOC 研判
- 资产风险汇总
- 报告生成

## 安全调度应用

用 Goose `recipes` 和 `subagents` 表达：

- 漏洞调查流程
- 告警归因流程
- 网页调查流程

不要另起一个“安全调度引擎”。

## 安全能力包

V1a 先用 Goose 原生目录装配：

- `skills/`
- `recipes/`
- `extensions/`
- `prompts/`

如果后续需要对外分发统一包，再考虑 Goose `plugin` 机制。

## 安全工具

用 Goose `MCP extensions` 表达：

- AiseeSec 查询
- 本地安全网关
- 威胁情报查询
- 浏览器辅助

## V1 状态持久化原则

V1 只依赖：

- Goose 原生会话持久化
- Goose 原生 memory
- `V1.5+` 才需要的 usage / quota / auth 数据

不建立新的“产品会话数据库”作为单一事实源。

如果需要云同步，V1 只做：

- 会话元数据镜像
- 用户偏好镜像

不要把完整 Goose 内部状态重新抽到另一套后端模型里。

## V1a 模型接入原则

V1a 优先使用 Goose 已有机制：

- `GOOSE_PROVIDER`
- `GOOSE_MODEL`
- `GOOSE_PROVIDER__HOST`
- `GOOSE_PROVIDER__API_KEY`
- `GOOSE_PREDEFINED_MODELS`
- `init-config.yaml`

只要这些足以完成：

- 连接 TokenPlan 兼容端点
- 展示模型列表
- 切换模型
- 暴露 `Auto + 真实模型名`

就不要先上自定义网关。

V1a 主路径不开放：

- Goose 式完全自由 provider 配置
- 本地模型入口
- 任意自定义 OpenAI-compatible endpoint

这些放到 `V1a+` 或 `V1.5+`。

## UI 改造范围

可改：

- 图标
- 名称
- 中文默认文案
- 安全产品入口
- 模型选择体验
- 少量任务模板入口文案

不建议 V1 深改：

- 全量页面布局
- 会话主体结构
- 核心设置结构
- 内部事件流

## 技术选型

### 客户端

- 继续用 Goose Desktop 原生 Electron + React

### Agent 层

- 继续用 Goose / goosed

### 模型路由

- `V1a` 直接使用 Goose 的 OpenAI-compatible provider 配置
- `V1.5+` 再引入 `LiteLLM`

### 产品网关

- `V1a` 默认无
- `V1.5+` 可使用 `Node.js + TypeScript` 做薄层控制面

### 安全能力

- `skills + recipes + MCP`

## 架构验收标准

- V1 功能成立时，仍然能明确看出 Goose 是主系统
- 产品差异主要来自 distro，而不是重写 core
- 安全场景通过 skills/MCP/recipes 组合出来，而不是另写 runtime
- 后续接 AGS 只需扩任务路由，不需推翻整个架构
- V1a 即使没有网关与 LiteLLM，也能完成本地预览
