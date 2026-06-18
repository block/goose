# 从 CSO 迁移到 Goose Fork 的执行清单

## 目标

把当前 `CSO` 仓库从“产品主线”彻底降级为：

- 参考资料库
- spec 仓库
- 能力候选池

把真正的产品开发主线切到新的 Goose fork 仓库中。

目标结果：

- `CSO` 继续沉淀文档与方案
- `Goose fork` 承担真实代码开发
- 分支、职责、节奏全部清晰分离

## 仓库角色划分

## 1. CSO 仓库

继续保留这些内容：

- 产品 spec
- 架构分析
- 内置能力目录
- Digpool / AiseeSec / TokenPlan 候选资料
- 历史探索和设计决策

不要继续在这里做：

- Goose Desktop 产品代码
- goosed 行为改造
- 真实 MCP 实现
- 打包和发布流程

## 2. Goose Fork 仓库

这里才是新的开发主线。

它负责：

- Goose Desktop branding
- 中文默认文案
- TokenPlan 模型配置
- 安全 skills / prompts / recipes
- MCP wrappers
- macOS 打包与 smoke test

## 迁移原则

1. 不复制当前 CSO 里的旧前后端实现思路到 Goose
2. 只迁移“规格、约束、能力目录、命名规则”
3. 优先用 Goose 原生机制落地
4. 先做 V1a 最小闭环，再谈网关、LiteLLM、AGS

## 要迁移过去的内容

从当前 `CSO` 仓库迁移到 Goose fork 的，应该是这些：

- `docs/goose-mainline-v1/README.md`
- `docs/goose-mainline-v1/01-repo-structure.md`
- `docs/goose-mainline-v1/02-v1-architecture.md`
- `docs/goose-mainline-v1/03-gateway-model-routing.md`
- `docs/goose-mainline-v1/04-security-skills-mcp.md`
- `docs/goose-mainline-v1/05-design-localization.md`
- `docs/goose-mainline-v1/06-development-sequence.md`
- `docs/goose-mainline-v1/07-testing-ci-cd.md`
- `docs/goose-mainline-v1/08-vibe-coding-playbook.md`
- `docs/goose-mainline-v1/09-curated-builtins-catalog.md`
- `docs/goose-mainline-v1/examples/*`

这些是“迁移源文档”，不是最终目录结构本身。

## 不要迁移过去的内容

这些不要直接复制进 Goose fork：

- 当前 CSO 的旧 app-server 设计
- 旧 runtime contract
- 旧 capability marketplace 实现
- Accio-parity 的 CSO Web 壳代码
- 任何与 Goose 原生结构冲突的中台抽象

如果未来有价值，也只能作为参考，不作为直接代码基础。

## Goose Fork 初始化步骤

## Step 1：创建新仓库

建议：

- fork `aaif-goose/goose`
- 新仓库名先用 `security-goose`

完成标准：

- 能 clone 到本地
- 能正常拉取 upstream

## Step 2：建立远程关系

建议保留两个 remote：

- `origin`：你的 fork
- `upstream`：`aaif-goose/goose`

完成标准：

- 可以 `fetch upstream`
- 可以看见 upstream 分支和 tag

## Step 3：创建主线开发分支

第一条分支固定建议：

- `codex/goose-v1a-bootstrap`

后续功能分支建议：

- `codex/v1a-branding`
- `codex/v1a-model-config`
- `codex/v1a-security-skills`
- `codex/v1a-security-mcp`
- `codex/v1a-mac-release`

## Step 4：导入 spec

把当前 `CSO` 中的 `docs/goose-mainline-v1/` 整包迁移到 Goose fork 的文档目录。

建议在 Goose fork 中落成：

- `docs/v1a/`

至少包含：

- `README.md`
- `architecture.md`
- `development-sequence.md`
- `testing-release.md`
- `curated-builtins-catalog.md`

## Step 5：创建发行版目录

在 Goose fork 里优先创建：

- `distro/security-cn/branding/`
- `distro/security-cn/config/`
- `distro/security-cn/locales/`
- `distro/security-cn/prompts/`
- `distro/security-cn/skills/`
- `distro/security-cn/recipes/`
- `distro/security-cn/extensions/`

V1a 先不要创建：

- `services/gateway/`
- 开放 marketplace backend
- 自定义 agent runtime

## Step 6：跑通原生 Goose

先验证上游 Goose 在你的机器上可运行：

- desktop 能启动
- goosed 能启动
- 设置页可打开
- provider 配置可生效

这一步没过，不要开始做产品定制。

## 首批开发顺序

迁移完成后，严格按这个顺序做：

1. 仓库 bootstrap
2. branding + 中文默认
3. TokenPlan 模型预置
4. 首批 skills
5. 首批 recipes
6. 首批 MCP
7. macOS 打包与 smoke

不要先做：

- gateway
- LiteLLM
- AGS
- 在线 marketplace
- 大改 Goose UI

## Goal 模式建议

如果你要继续用 goal 模式推进，建议直接拆成下面 6 个 goal。

## Goal 1：Goose Fork Bootstrap

验收：

- fork 完成
- 本地 clone 完成
- upstream remote 配好
- `codex/goose-v1a-bootstrap` 分支建好
- 原生 Goose 可启动

## Goal 2：Spec 导入与目录骨架

验收：

- `docs/v1a/` 建好
- `distro/security-cn/` 建好
- 示例配置文件有落点

## Goal 3：Branding 与中文默认

验收：

- app name 替换
- icon / splash 替换
- 默认语言变成 `zh-CN`
- 默认安全助手文案生效

## Goal 4：TokenPlan 模型配置

验收：

- Goose 直连 TokenPlan 兼容端点
- 模型列表显示 `Auto + 真实模型名`
- 模型切换可用

## Goal 5：首批安全内置能力

验收：

- 5 个 skills 落地
- 3-4 个 recipes 落地
- 4 个 agent presets 落地

## Goal 6：首批 MCP 与 Mac 预览

验收：

- `aiseesec-mcp` 可用
- 至少 1-2 个通用安全 MCP 可用
- macOS 打包成功
- smoke checklist 可通过

## 文件迁移建议

如果你想保持迁移简单，建议这样映射：

| CSO 当前文件 | Goose fork 建议位置 |
| --- | --- |
| `docs/goose-mainline-v1/README.md` | `docs/v1a/README.md` |
| `docs/goose-mainline-v1/02-v1-architecture.md` | `docs/v1a/architecture.md` |
| `docs/goose-mainline-v1/06-development-sequence.md` | `docs/v1a/development-sequence.md` |
| `docs/goose-mainline-v1/07-testing-ci-cd.md` | `docs/v1a/testing-release.md` |
| `docs/goose-mainline-v1/09-curated-builtins-catalog.md` | `docs/v1a/curated-builtins-catalog.md` |
| `docs/goose-mainline-v1/examples/*` | `distro/security-cn/config/` 或 `docs/v1a/examples/` |

## 切换时的检查项

真正开始 Goose fork 开发前，确认这几个问题已经锁死：

- V1a 是不是只做 `macOS-only`
- V1a 是不是只做本机预览
- 模型是不是先固定 TokenPlan
- 主路径是不是只开放 `Auto + 真实模型名`
- 首发是不是不做 gateway / LiteLLM / AGS
- 首批内置能力名单是不是已经确定

如果这些还没锁死，不要急着进代码实现。

## 完成定义

迁移工作完成，必须同时满足：

- `CSO` 不再承担产品代码主线
- Goose fork 已成为唯一实现主线
- spec 已在 Goose fork 有副本
- 分支策略明确
- 首批 V1a goals 明确
- 开发顺序明确

## 下一步建议

迁移清单完成后，下一步最合理的是：

1. 真正创建 Goose fork 本地目录
2. 导入 `docs/v1a/`
3. 建 `codex/goose-v1a-bootstrap`
4. 进入 Goal 1 和 Goal 2

如果你确认要开始，我下一轮可以直接按这个清单，切到 Goose fork 视角，给你出第一条可执行开发 goal。
