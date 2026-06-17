# V1 开发顺序 Spec

## 开发目标

在最短路径上做出：

- 一个 branded Goose Desktop
- 先只支持 macOS 本地预览
- 默认中文
- 支持模型切换
- 内置安全 skills / recipes / MCP
- 能用来完成真实安全任务

## 工作顺序原则

1. 先做“可启动、可对话、可切模型”
2. 再做“可用的安全技能包”
3. 再做“精选 MCP / 扩展”
4. 再做“可选的网关与套餐能力”
5. 最后做“包装、测试、CI/CD、发布”

不要先做：

- 大而全后台
- 复杂远端调度
- 深度 UI 美化
- 自建平台层

## 开发阶段

## Phase 0：仓库初始化

目标：

- fork Goose
- 跑通开发环境
- 建立 distro 目录与文档骨架

任务：

- 创建 `distro/security-cn/`
- 建立 `docs/`
- 跑通 `ui/desktop` 本地启动
- 跑通 `goosed`

验收：

- 新仓库可本地启动 Goose Desktop
- 自定义目录结构已建立

## Phase 1：品牌与基础发行版

目标：

- 应用名称、图标、文案替换
- 默认语言切到中文
- 默认 provider / model 配置完成

任务：

- 替换图标、名称、打包元数据
- 注入 `init-config.yaml`
- 默认关闭上游不需要的噪音入口
- 修改系统提示词中的品牌名与角色定义

验收：

- 打开就是你的品牌客户端
- 首次启动默认中文
- 默认模型配置生效

## Phase 2：模型切换与 TokenPlan 预置

目标：

- 客户端能切 `Auto + 已配置开放的具体模型`
- 先通过 Goose 原生配置完成模型切换

任务：

- 创建 `model-catalog.json`
- 配置 `OPENAI_BASE_URL`
- 配置 `OPENAI_API_KEY`
- 配置 `GOOSE_PREDEFINED_MODELS`
- 主入口模型列表接成 `Auto + 真实模型名`

验收：

- Goose 客户端能稳定切换预置的模型列表
- 切换不需要改 Goose core

## Phase 3：安全技能包

目标：

- 至少 5 个高价值安全 skills 能用

任务：

- `vuln-triage`
- `alert-triage`
- `ioc-analysis`
- `asset-risk-summary`
- `report-writing`

每个 skill 都要带：

- 标准输出模板
- 风险边界
- 验证步骤

验收：

- 用户可直接在中文任务语义下调用这些能力
- 技能输出结果更像安全工作产出，而不是泛聊天

## Phase 4：Recipe 与任务模板入口

目标：

- 用户不只会“聊天”，还会“一键进入任务模板”

任务：

- `security-vuln-triage`
- `alert-investigation`
- `web-investigation`

同时做：

- 角色默认入口
- Goose 原生任务模板入口的中文化与命名收敛

验收：

- 用户不用知道 Goose 底层术语，也能发起安全任务

## Phase 5：精选 MCP / 扩展

目标：

- 接入 AiseeSec 与本地安全工具能力

任务：

- `aiseesec-mcp`
- `local-security-gateway-mcp`
- `threat-intel-mcp`
- `browser-assist-mcp`

验收：

- 至少 2-3 个真实工具查询链路可用
- 能与 skills / recipes 组合使用

## Phase 6：产品网关增强（V1.5+）

目标：

- 有基础套餐与配额控制

任务：

- auth middleware
- plan policy
- quota policy
- usage events

验收：

- 免费/付费模型能力可区分
- 关键 usage 可记录

## Phase 7：测试、打包、发布

目标：

- 可通过 GitHub Actions 构建、测试、打包

任务：

- Rust tests
- desktop lint/test/build
- skill/locale/config validation
- desktop release workflow

验收：

- PR 有自动 CI
- tag 可生成桌面构建产物

## 周级建议

## 第 1 周

- Phase 0
- Phase 1

## 第 2 周

- Phase 2
- Phase 3

## 第 3 周

- Phase 4
- Phase 5

## 第 4 周

- Phase 7
- 视需要预研 Phase 6

这是最理想的 4 周节奏。若资源少，可拉成 6 周，但顺序不要乱。

## 优先级表

### P0

- fork 可启动
- branding
- 默认中文
- 模型切换
- 5 个安全 skills
- 2-3 个真实 MCP 能力

### P1

- recipe 任务模板
- 打包发布
- 基础 smoke checklist

### P2

- 更完整双语
- 更深 UI 定制
- 网关套餐逻辑
- 使用量记录
- 后续 AGS 预接缝

## V1 最终验收标准

- 用户安装后可直接使用
- 能选模型
- 能发起至少 3 类安全任务
- 有真实安全技能输出
- 有至少 2 类真实工具接入
- 具备最小测试与发布流水线
