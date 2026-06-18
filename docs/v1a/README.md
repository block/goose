# Goose 新主线 V1 Spec 包

## 目标

这套文档用于指导一个**全新的 Goose 主线仓库**开发，不再以当前 `CSO` 代码作为产品主线。

目标产品：

- 基于 `aaif-goose/goose` fork
- 复用 Goose 原生的：
  - Agent 调度
  - Session / 加载
  - Memory
  - Extensions / MCP
  - Recipes / Subagents
  - Provider / Multi-model
- 只增加安全场景的：
  - 品牌与桌面壳
  - 中国安全人员习惯的 UI 文案与默认工作流
  - 安全 skills / recipes / MCP 扩展
  - 后续可选的模型网关与套餐能力
  - 后续接入腾讯云 AGS 的远端任务路由缝隙

V1 的定义：

- **能用优先**
- **效果优先**
- **不考虑持续任务时长优化**
- **不新建并行 Agent runtime / memory / scheduler 轮子**

补充约束：

- `V1a` 允许先做 `macOS-only` 客户端预览版
- `V1a` 优先借 Goose 原生 provider / model / config 机制
- `V1a` 不强制要求产品网关
- `V1a` 不强制要求 LiteLLM

## 核心假设

- 新仓库暂定代号：`security-goose`
- 上游基线：`aaif-goose/goose`
- 发行形态：桌面客户端为主
- 默认语言：`zh-CN`
- 次语言：`en-US`
- `V1a` 模型接入优先通过 Goose 原生配置直连 `TokenPlan`
- AiseeSec 以精选 `skills / MCP / recipes` 方式接入

如果后续品牌名、仓库名、组织名变化，这套 spec 保持成立。

## 文档地图

- [01-repo-structure.md](./01-repo-structure.md)
  新主线仓库目录结构、模块职责、哪些目录允许定制

- [02-v1-architecture.md](./02-v1-architecture.md)
  V1 架构边界、必须复用的 Goose 机制、禁止自建的部分

- [03-gateway-model-routing.md](./03-gateway-model-routing.md)
  V1a 直连 TokenPlan 的模型切换方式，以及 `V1.5+` 网关升级口

- [04-security-skills-mcp.md](./04-security-skills-mcp.md)
  安全 skills、MCP、recipes、AiseeSec 精选能力打包方式

- [05-design-localization.md](./05-design-localization.md)
  面向中国安全人员的产品语义、UI 风格、双语与默认文案规范

- [06-development-sequence.md](./06-development-sequence.md)
  V1 开发顺序、每个切片的范围、验收标准

- [07-testing-ci-cd.md](./07-testing-ci-cd.md)
  测试矩阵、GitHub Actions CI/CD、发布流与质量门禁

- [08-vibe-coding-playbook.md](./08-vibe-coding-playbook.md)
  基于 Codex 的开发方式、任务拆分、文档计划、分支策略、协作方式

- [09-curated-builtins-catalog.md](./09-curated-builtins-catalog.md)
  V1a 内置 skills、prompts、recipes、agent presets、MCP、Digpool 候选来源的筛选目录

- [10-migration-to-goose-fork-checklist.md](./10-migration-to-goose-fork-checklist.md)
  从当前 CSO spec 仓库切换到 Goose fork 主线开发的迁移执行清单

- [11-bootstrap-audit.md](./11-bootstrap-audit.md)
  Goose 当前入口、`distro/security-cn` 骨架、以及 spec 与实现冲突的最小修正说明

- 示例配置：
  - [init-config.yaml.example](./examples/init-config.yaml.example)
  - [desktop-env.example](./examples/desktop-env.example)
  - [gateway.env.example](./examples/gateway.env.example) `V1.5+`
  - [bundled-extensions.security.json.example](./examples/bundled-extensions.security.json.example)
  - [security-vuln-triage.recipe.yaml.example](./examples/security-vuln-triage.recipe.yaml.example)
  - [ci.yml.example](./examples/ci.yml.example)
  - [release-desktop.yml.example](./examples/release-desktop.yml.example)

## 绝对约束

1. V1 不新建并行的 Agent runtime。
2. V1 不新建并行的 memory system。
3. V1 不新建并行的 tool scheduler。
4. V1 不复刻 Goose 的 session store、message store、extension loader。
5. 自定义尽量收敛在：
   - distro 配置
   - branding
   - skills / recipes
   - bundled MCP
   - 少量桌面 UI 文案与能力入口
6. 任何新能力，优先判断能否通过：
   - `config`
   - `recipe`
   - `skill`
   - `MCP extension`
   实现，不能先写 core patch。
7. 如果一个能力可以直接通过 Goose 的：
   - `GOOSE_PROVIDER`
   - `GOOSE_MODEL`
   - `OPENAI_BASE_URL`
   - `OPENAI_API_KEY`
   - `GOOSE_PREDEFINED_MODELS`
   - `init-config.yaml`
   完成，则 V1a 不应先引入自定义网关或 LiteLLM。

## V1 非目标

- 自建完整多租户平台
- 自建远端 Agent runtime
- 自建复杂任务调度器
- 自建长期记忆平台
- 自建技能市场平台后端
- 首发即做统一网关控制面
- 首发即做完整模型计费与配额系统
- 首发即深度接入腾讯云 AGS
- 首发即做企业级组织治理后台

## 验收标准

这套 spec 被视为完成，需要满足：

- 能指导新 Goose 主线仓库的初始化
- 能指导 V1 开发顺序
- 明确仓库目录结构
- 明确哪些 Goose 机制复用、哪些不能重造
- 明确模型切换路径，以及 `V1.5+` 网关升级口
- 明确安全 skills / MCP / recipe 的打包路线
- 明确中国安全用户习惯与双语策略
- 明确测试、CI、CD、发布方案
- 明确文档与 vibe coding 协作方式

## 与当前仓库的关系

当前 `CSO` 仓库不再承担 V1 主线开发，只保留为：

- 产品与术语参考
- 安全场景素材库
- AiseeSec 方向参考
- TokenPlan 思路参考
- 未来服务端设计参考
