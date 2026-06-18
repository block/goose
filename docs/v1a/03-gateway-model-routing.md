# 网关与模型路由设计

## 目标

V1a 的模型层必须满足：

- 支持切换不同模型
- 当前可直接连接 `TokenPlan` 兼容端点
- 用户能看到并切换已配置开放的具体模型
- `Auto` 只是一个默认选项，不是唯一抽象模式
- 不改 Goose core provider 逻辑
- 后续可升级到免费/付费套餐区分
- 后续可平滑接更多供应商

## 核心结论

如果第一版只是：

- `macOS-only`
- 本地预览
- 先验证 Goose 定制客户端是否可用

那么：

- **不需要先做产品网关**
- **不需要先做 LiteLLM**

最稳妥、最快的方式是：

> **Goose 直接使用现有 provider/config 机制**  
> **连 TokenPlan 兼容端点**  
> **通过 `init-config.yaml` 与 `GOOSE_PREDEFINED_MODELS` 控制默认模型列表**

## V1a 分层图

```text
Security Goose Desktop
  -> goosed provider config
  -> TokenPlan-compatible endpoint
```

## 什么时候不需要网关

以下场景不需要：

- 你本人在本机开发和预览
- 首发只做 Mac 版体验验证
- 先验证 security skills / MCP / recipes
- 当前不要求隐藏服务端密钥给第三方用户

## 为什么这时可以直接借 Goose 机制

Goose 官方已经支持：

- 基础 provider 变量：`GOOSE_PROVIDER`、`GOOSE_MODEL`
- OpenAI-compatible endpoint：`OPENAI_BASE_URL`
- OpenAI-compatible API key：`OPENAI_API_KEY`
- 自定义模型清单：`GOOSE_PREDEFINED_MODELS`
- 发行版初始化：`init-config.yaml`

这已经足够完成：

- 指向 TokenPlan 兼容端点
- 配置默认模型
- 暴露一组可选模型
- 支持 `Auto + 真实模型名`

## 为什么后续可能仍要网关

当你把客户端发给其他用户时，如果仍想使用你自己购买的 TokenPlan 配额，就会遇到：

- 不能把你的服务端 key 直接放到客户端
- 套餐能力难控制
- 使用量统计分散
- 未来付费模型难统一收口

这时再引入产品网关。

## 什么时候再引入 LiteLLM

以下场景值得加 LiteLLM：

- 不止一个上游供应商
- 需要统一模型别名
- 需要供应商切换与灰度
- 需要更稳定的代理与重试
- 需要上下文限制覆写

Goose 文档也明确提到，`GOOSE_CONTEXT_LIMIT` 对 LiteLLM proxy 很有用。

所以结论是：

- `V1a`: 不需要 LiteLLM
- `V1.5+`: 再考虑 LiteLLM

## 产品网关职责

产品网关是 `V1.5+` 项，不是 `V1a` 强制项。

它只做五件事：

1. 身份校验
2. 套餐与配额判断
3. 暴露 `model catalog`
4. 把模型请求代理给 LiteLLM
5. 记录 usage event

产品网关不做：

- 聊天 orchestration
- skill 调度
- memory 存储
- tool 调用
- 会话生命周期

## V1a 推荐配置

### 直接使用 Goose 配置

- `GOOSE_PROVIDER`
- `GOOSE_MODEL`
- `GOOSE_PROVIDER__TYPE`
- `GOOSE_PROVIDER__HOST`
- `GOOSE_PROVIDER__API_KEY`
- `GOOSE_PREDEFINED_MODELS`
- `GOOSE_FAST_MODEL`
- `GOOSE_PLANNER_MODEL`

### 推荐做法

- 用 `init-config.yaml` 预置默认 provider
- 用 `GOOSE_PREDEFINED_MODELS` 预置一组模型
- 只开放 `Auto + 你当前真实提供的 TokenPlan 模型`
- 用 Goose 原生模型切换入口展示模型列表

## `GET /api/model-catalog`

这是 `V1.5+` 的增强接口，不是 `V1a` 必需。

用于客户端从后端拉模型列表和可见性信息。

返回建议字段：

```json
{
  "defaultModel": "deepseek-v4-flash",
  "models": [
    {
      "id": "auto",
      "labelZh": "自动",
      "labelEn": "Auto",
      "tier": "free",
      "kind": "virtual",
      "reasoning": "balanced",
      "provider": "tokenplan",
      "upstreamModel": "auto"
    },
    {
      "id": "deepseek-v4-flash",
      "labelZh": "DeepSeek V4 Flash",
      "labelEn": "DeepSeek V4 Flash",
      "tier": "free",
      "kind": "hosted",
      "reasoning": "low",
      "provider": "tokenplan",
      "upstreamModel": "deepseek-v4-flash"
    },
    {
      "id": "deepseek-v4-pro",
      "labelZh": "DeepSeek V4 Pro",
      "labelEn": "DeepSeek V4 Pro",
      "tier": "pro",
      "kind": "hosted",
      "reasoning": "high",
      "provider": "tokenplan",
      "upstreamModel": "deepseek-v4-pro"
    },
    {
      "id": "kimi-k2.6",
      "labelZh": "Kimi K2.6",
      "labelEn": "Kimi K2.6",
      "tier": "pro",
      "kind": "hosted",
      "reasoning": "high",
      "provider": "tokenplan",
      "upstreamModel": "kimi-k2.6"
    },
    {
      "id": "glm-5.1",
      "labelZh": "GLM 5.1",
      "labelEn": "GLM 5.1",
      "tier": "pro",
      "kind": "hosted",
      "reasoning": "high",
      "provider": "tokenplan",
      "upstreamModel": "glm-5.1"
    }
  ]
}
```

建议再补充可选字段：

- `visible`: 是否在主模型列表显示
- `enabled`: 当前套餐是否可用
- `source`: `hosted | local | custom`
- `group`: `recommended | hosted | local | custom`
- `badgeZh` / `badgeEn`: 如 `推荐`、`付费`

## `POST /v1/chat/completions`

这是 `V1.5+` 的增强接口。

这一层尽量保持 OpenAI-compatible，让 Goose 用现成 provider 即可。

行为：

- 验证产品 token
- 验证套餐
- 校验模型别名
- 透传到 LiteLLM

## `POST /api/usage/events`

这是 `V1.5+` 的增强接口。

记录：

- user id
- model id
- session id
- token input/output
- request latency
- source client version

V1 可异步写入，不阻塞主对话路径。

## 模型目录设计

## V1 主入口应展示“后台已开放的真实模型”

你的用户是安全工程师，不是泛 C 端聊天用户。

因此 V1 不应该把模型抽象成：

- `快速`
- `深度`
- `研判`

这种过于产品经理化的模式名。

V1 主入口建议直接展示：

- `Auto`
- `DeepSeek V4 Flash`
- `DeepSeek V4 Pro`
- `Kimi K2.6`
- `GLM 5.1`

这些名称在 `V1a` 可以直接来自本地配置，在 `V1.5+` 再改为后台下发。

客户端行为应是：

- `Auto` 放在第一位，作为推荐默认项
- 其余模型按后台配置顺序展示
- 不可用模型直接不下发，或下发为禁用态

这样做的好处：

- 符合安全工程师的使用习惯
- 用户知道自己在用哪个模型
- `V1a` 由发行版配置控制可见范围
- `V1.5+` 再由后台配置控制可见范围

## 本地 / 自定义模型

这条路不属于 V1a 主路径。

如果后续要开，建议只放在：

- 设置页
- 高级模型设置
- 实验功能

并且后置到 `V1a+` 或 `V1.5+`，不要首发就暴露给所有用户。

## Goose 侧接法

优先使用：

- Goose 现有 provider 配置
- OpenAI-compatible base URL
- 默认 model alias

不要 V1a 就去做自定义 provider trait，除非：

- LiteLLM / 网关无法满足认证或接口需求

## LiteLLM 建议职责

这是 `V1.5+` 的设计，不是 `V1a` 的前置要求。

LiteLLM 负责：

- 上游 API key 管理
- 模型路由
- 别名映射
- 供应商切换

建议模型 id：

- `auto`
- `deepseek-v4-flash`
- `deepseek-v4-pro`
- `kimi-k2.6`
- `glm-5.1`
- `claude-3-7-sonnet`
- `gpt-4.1`

V1 按后台配置开放其中一部分即可。

## TokenPlan 接法

### V1a 推荐

- Goose -> TokenPlan-compatible endpoint

### V1.5+ 推荐

- 产品网关 -> LiteLLM -> TokenPlan-compatible endpoint

好处：

- `V1a` 上线最快
- `V1.5+` 更容易切流和做套餐

## 套餐策略

这是 `V1.5+` 设计，`V1a` 可暂不实现。

## 免费版

- `auto`
- 部分 hosted 模型
- 日额度限制
- 并发限制

## 付费版

- 更多 hosted 模型
- 更高日额度
- 更高上下文预算
- 允许更多 MCP/recipe 能力
- 开启本地/自定义模型高级配置

## V1a 不做的网关能力

- 复杂账单引擎
- 组织级 RBAC
- 多区域路由
- 细粒度成本优化器
- 自研模型调度器

## V1.5+ 实现建议

### `services/gateway/src/routes/`

- `auth.ts`
- `catalog.ts`
- `chat-proxy.ts`
- `usage.ts`
- `health.ts`

### `services/gateway/src/lib/`

- `model-catalog.ts`
- `plan-policy.ts`
- `litellm-client.ts`
- `token-counter.ts`
- `request-audit.ts`

### `services/gateway/src/config/`

- `models.json`
- `plans.json`
- `env.ts`

## 配置优先

模型策略优先放配置，不写死在代码里：

- `models.json`
- `plans.json`
- `feature-flags.json`

## V1a 验收标准

- Goose 客户端可以切模型
- 主入口能看到已配置开放的真实模型名
- `Auto` 只是默认项，不遮蔽真实模型选择
- 不需要改 Goose core 才能完成模型切换
- TokenPlan-compatible endpoint 可以直接接入
- 主入口只显示 `Auto + 已配置开放的真实模型`
