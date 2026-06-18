# V1a Curated Built-ins Catalog

## 目标

V1a 不做开放 marketplace，改为做一份可控的内置能力目录。

目标是：

- 首发就有一批安全工程师真正会用的能力
- 能力来源清楚、风险可控、升级路径明确
- 继续复用 Goose 的原生机制，不新造平台型抽象

这份目录覆盖六类内置对象：

- `skills`
- `prompts`
- `recipes`
- `agent presets`
- `MCP extensions`
- `plugin bundles`

其中 `agent presets` 在 V1a 里不是新 runtime，而是：

- 默认 system prompt
- 默认 recipes
- 默认 extensions
- 推荐模型

的产品打包视图。

## 选择原则

任何候选能力要进入 V1a 内置目录，至少满足：

1. 来源可信
2. 文档可得
3. 接口或行为稳定
4. 首版可以优先只读或低风险执行
5. 有明确安全工作场景，不是泛聊天玩法
6. 可以映射到 Goose 现有机制

默认拒绝：

- 无稳定维护方的匿名社区条目
- 首版就要求高权限写操作的插件
- 需要大规模自建后台才能运行的能力
- 不清楚许可、协议、数据来源的条目

## 候选来源分层

### S0：Goose 原生能力

优先直接复用 Goose 已有机制：

- built-in extensions
- recipes
- skills
- memory
- MCP extension loading

这类能力默认可信度最高，接入成本最低。

参考：

- Goose 首页与能力概览：[goose-docs.ai](https://goose-docs.ai/)
- Extensions 文档：[Using Extensions](https://goose-docs.ai/docs/getting-started/using-extensions/)
- Recipes 文档：[Recipes](https://goose-docs.ai/docs/guides/recipes/)

### S1：官方安全数据与工作流系统

这类来源适合做 V1a 的第一批标准工具。

建议优先：

- VirusTotal
- Shodan
- Censys
- urlscan.io
- NVD / CVE
- MITRE ATT&CK
- GitHub
- Jira

参考：

- VirusTotal API：[docs.virustotal.com/reference/overview](https://docs.virustotal.com/reference/overview)
- Shodan API：[developer.shodan.io/api](https://developer.shodan.io/api)
- Censys API：[docs.censys.com/reference/get-started](https://docs.censys.com/reference/get-started)
- urlscan API：[urlscan.io/docs/api](https://urlscan.io/docs/api/)
- NVD Developers：[nvd.nist.gov/developers](https://nvd.nist.gov/developers)
- MITRE ATT&CK Data & Tools：[attack.mitre.org/resources/attack-data-and-tools](https://attack.mitre.org/resources/attack-data-and-tools/)
- GitHub REST API：[docs.github.com/en/rest](https://docs.github.com/en/rest)
- Jira REST API：[developer.atlassian.com/cloud/jira/platform/rest/v3/intro/](https://developer.atlassian.com/cloud/jira/platform/rest/v3/intro/)

### S2：企业自有与精选安全来源

这类来源是产品差异化的核心。

首批建议：

- `aiseesec`
- `local-security-gateway`
- 企业内自有知识库 / 工单 / 风险平台

原则：

- 只挑高价值、稳定、可解释的能力
- 优先做查询与研判辅助
- 把真实执行授权留在本地受控配置层

### S3：社区与第三方能力广场

这类来源可以作为候选池，但不能自动进入 V1a 内置列表。

当前建议纳入观察的来源：

- Digpool 蛙池中心
- Goose Skills Marketplace
- MCP 社区目录

这类来源进入内置目录前，必须经过：

- 热度筛选
- 维护状态筛选
- 风险审查
- 协议与许可检查
- 本地 smoke 验证

## Digpool 纳入规则

`Digpool` 可以作为候选来源，但当前不应直接等同于“可默认内置”。

原因：

- 公开中心页是动态渲染页面
- 我在 `2026-06-14` 访问公开地址 [www.digpool.cn/center](https://www.digpool.cn/center) 时，未登录视图只显示 `0` 和 `暂无数据`
- 因此当前无法仅基于公开页面稳定提取“高下载量 skill 清单”

V1a 对 Digpool 的采用方式应是：

1. 你提供后台可见的高下载量能力列表，或导出的能力清单
2. 我们按统一规则做二次筛选
3. 入选项再映射到 Goose 的 `skill / prompt / recipe / MCP` 结构

### Digpool 入选门槛

建议至少满足：

- 下载量高
- 最近仍有维护
- 场景属于安全事务主路径
- 不依赖不透明远端执行
- 不默认申请危险权限
- 可被中文安全用户稳定复用

优先纳入的 Digpool 类能力：

- 漏洞研判
- 资产测绘与风险汇总
- IOC 富化
- 告警初筛
- 报告整理

后置纳入的 Digpool 类能力：

- 自动化攻击模拟
- 高权限变更执行
- 需要远端账号托管的复杂工作流

## V1a 建议内置对象

## 1. Skills

首批固定内置：

- `vuln-triage`
- `alert-triage`
- `ioc-analysis`
- `asset-risk-summary`
- `report-writing`

第二批候选：

- `web-investigation`
- `code-security-review`
- `threat-hunting-summary`

## 2. Prompts

首批固定内置：

- `security-analyst-default`
- `vuln-triage-prompt`
- `alert-investigation-prompt`
- `ioc-enrichment-prompt`
- `security-report-prompt`

要求：

- 每个 prompt 都要有清晰角色边界
- 不直接暴露内部实现名给最终用户
- 中文为默认主文案

## 3. Recipes

首批固定内置：

- `security-vuln-triage`
- `alert-investigation`
- `web-investigation`
- `repo-security-review`

要求：

- recipe 必须绑定明确任务目标
- 可以指定推荐模型
- 可以指定推荐 extensions

## 4. Agent Presets

V1a 不单独造 Agent 系统，先做 4 个预置工作台角色：

- `漏洞研判助手`
- `威胁情报助手`
- `网页调查助手`
- `安全报告助手`

每个 preset 对应：

- 一个默认 system prompt
- 一个默认 recipe 集
- 一组默认 MCP extensions
- 一个推荐模型

## 5. MCP Extensions

### 企业与精选来源

- `aiseesec-mcp`
- `local-security-gateway-mcp`

### 官方与行业通用来源

- `virustotal-mcp`
- `shodan-mcp`
- `censys-mcp`
- `urlscan-mcp`
- `nvd-mcp`
- `attack-mcp`
- `github-mcp`
- `jira-mcp`

V1a 优先级：

1. `aiseesec-mcp`
2. `virustotal-mcp`
3. `nvd-mcp`
4. `attack-mcp`
5. `shodan-mcp`
6. `censys-mcp`
7. `urlscan-mcp`
8. `github-mcp`
9. `jira-mcp`

## 6. Plugin Bundles

V1a 不做开放插件市场，但可以保留内部 bundle 概念，便于后续扩展。

建议只做产品内部分组，不暴露为开放安装系统：

- `threat-intel-pack`
- `vuln-response-pack`
- `security-report-pack`

每个 bundle 只是一个分发组织方式，底层仍映射到：

- skills
- prompts
- recipes
- extensions

## 内置能力筛选矩阵

| 类型 | 首发是否建议 | 默认权限 | 来源优先级 | 备注 |
| --- | --- | --- | --- | --- |
| Skill | 是 | 低 | S0 / S2 / S3 | 先做事务型技能 |
| Prompt | 是 | 低 | S0 / S2 / S3 | 用于角色和文案统一 |
| Recipe | 是 | 低 | S0 / S2 | 适合任务模板化 |
| Agent Preset | 是 | 低 | S0 / S2 | 只是产品预设，不是新 runtime |
| MCP Extension | 是 | 中 | S1 / S2 / S3 | 优先查询型能力 |
| Plugin Bundle | 可选 | 低 | S0 / S2 | 只做内部组织，不做开放市场 |

## 首批落地顺序

建议按这个顺序开发：

1. `security-analyst-default` prompt
2. `vuln-triage` skill
3. `security-vuln-triage` recipe
4. `aiseesec-mcp`
5. `virustotal-mcp`
6. `nvd-mcp`
7. `attack-mcp`
8. `漏洞研判助手` preset

第二批再补：

- `alert-triage`
- `ioc-analysis`
- `shodan-mcp`
- `censys-mcp`
- `web-investigation`

## 后续需要你提供的数据

如果你要把 Digpool 高下载量能力真正并入内置基线，后续最好补一份清单，字段至少包括：

- 名称
- 类型
- 下载量
- 更新时间
- 作者/维护方
- 场景描述
- 是否需要登录
- 是否需要远端执行
- 是否有公开文档

只要你给出这份表，我就可以直接帮你做：

- 入选评级
- V1a / V1.5 分层
- Goose 映射方案
- 首批内置建议名单
