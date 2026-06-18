# Codex / Vibe Coding 开发手册

## 目标

这份手册的作用不是“讲开发哲学”，而是让你能直接按 spec 驱动 Codex 做实现。

## 核心工作方式

### 先做发行版，不先做平台

所有开发任务先问一句：

> 这件事能不能作为 Goose 发行版定制完成？

如果能，就不要上升为平台开发。

### 先做装配，不先做深改

优先顺序：

1. config
2. branding
3. skills
4. recipes
5. MCP extensions
6. 少量 UI
7. gateway
8. 最后才是 core patch

## Codex 任务拆分模板

每次给 Codex 的任务，都尽量写成下面这种粒度：

### 模板

```text
目标：
在 Goose fork 中完成 [一个明确切片]。

约束：
- 不改 Goose core，除非证明配置/recipe/skill/extension 无法完成
- 优先复用 upstream 目录结构
- 只修改 [明确目录]
- 补测试
- 补文档

验收：
- [具体用户可见结果]
- [具体命令或测试]
```

## 推荐的 Codex 实现顺序

### Prompt 1：主线仓库初始化

让 Codex 做：

- 新建 `distro/security-cn/`
- 新建 docs 结构
- 补 README

### Prompt 2：branding

让 Codex 做：

- 改 app name
- 改 icon
- 改启动文案
- 改默认 system prompt 中的品牌名

### Prompt 3：模型切换

让 Codex 做：

- 建 `model-catalog.json`
- 用 `init-config.yaml` 预置 TokenPlan endpoint
- 配置 `GOOSE_PREDEFINED_MODELS`
- 客户端接 `Auto + 真实模型名` 入口

### Prompt 4：安全 skills

让 Codex 做：

- 生成首批 5 个 skill 目录
- 按统一模板写 `SKILL.md`
- 补 supporting templates
- 加校验脚本

### Prompt 5：安全 recipes

让 Codex 做：

- 3 个 recipe
- 每个 recipe 对应 skill 与扩展配置
- 映射到 Goose 原生任务模板入口

### Prompt 6：MCP

让 Codex 做：

- bundled extension config
- AiseeSec MCP wrapper
- 本地安全网关 MCP wrapper
- 威胁情报 MCP wrapper

### Prompt 7：CI/CD

让 Codex 做：

- `.github/workflows/ci.yml`
- `.github/workflows/release-desktop.yml`
- `scripts/validate-skills.mjs`
- `scripts/validate-locales.mjs`

## 文档先行策略

主线仓库里，至少要先有这些文档：

- `README.md`
- `PRODUCT.md`
- `ARCHITECTURE.md`
- `DEVELOPMENT.md`
- `TESTING.md`
- `RELEASE.md`
- `LOCALIZATION.md`
- `SKILLS.md`
- `MCP.md`

Codex 每做一块，就同步更新对应文档。

## 分支建议

推荐：

- `main`
- `codex/goose-v1a-bootstrap`
- `codex/v1a-branding`
- `codex/v1a-model-config`
- `codex/v1a-security-skills`
- `codex/v1a-security-mcp`
- `codex/v1a-mac-release`

这样每次工作都足够小，便于评审和回滚。

## 单次提交的最佳粒度

一个分支只做一类事：

- 只做 branding
- 只做 model config
- 只做 skill pack
- 只做 recipe pack
- 只做 CI/CD

不要混着做。

## 任务拆分原则

### 好任务

- “新增 `vuln-triage` skill，并补模板与校验”
- “把 TokenPlan endpoint 与预置模型列表接进 Goose 默认配置”
- “把 desktop 默认语言切到 zh-CN，并补中英 locale 文件”

### 坏任务

- “把整个安全工作台做完”
- “把 Goose 改造成我们的产品”
- “把前后端都重构一下”

## 评审标准

每个任务评审时只看四件事：

1. 有没有复用 Goose 机制
2. 有没有引入不必要新轮子
3. 有没有补测试
4. 有没有补文档

## 文档开发计划

建议顺序：

1. 先写产品总览
2. 再写架构
3. 再写 skills / MCP
4. 再写 testing / release
5. 网关文档放到 `V1.5+` 再补

这与实现顺序保持一致。

## 与当前 CSO 仓库的协作方式

当前仓库只做三件事：

- 查原型与历史思路
- 抽安全场景素材
- 抽已有 TokenPlan / AiseeSec 思路

不要再在当前仓库继续叠 V1 功能。

## 完成定义

一个 V1 切片完成，必须同时满足：

- 代码完成
- 测试完成
- 文档完成
- 配置样例完成

少一项都不算完整。
