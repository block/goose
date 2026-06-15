# 安全 Skills、Recipes 与 MCP 设计

## 总原则

V1 的安全能力不要发明新抽象，统一映射到 Goose 已有能力：

- `Skill`：领域知识与工作法
- `MCP extension`：工具访问与外部系统
- `Recipe`：预配置流程
- `Subagent`：复杂任务中的角色拆分

## 不再新增“平台型 capability runtime”

V1 不做：

- 新的 capability bus
- 新的 skill runner
- 新的 recipe engine

## 能力映射表

| 需求 | Goose 对应能力 | 说明 |
| --- | --- | --- |
| 漏洞研判方法论 | Skill | 指令、步骤、模板 |
| AiseeSec 查询能力 | MCP | 结构化工具调用 |
| 一键跑某类安全工作流 | Recipe | 固定流程入口 |
| 多角色协作研判 | Subagent | 例如“分析员/审阅员” |

## V1 推荐能力包

## Skills

首批建议内置 5 个：

1. `vuln-triage`
2. `alert-triage`
3. `ioc-analysis`
4. `asset-risk-summary`
5. `report-writing`

每个 skill 必须包含：

- 明确使用场景
- 输入要求
- 分步流程
- 风险提示
- 输出模板
- 验证清单

## Recipes

首批建议 3 个：

1. `security-vuln-triage`
2. `alert-investigation`
3. `web-investigation`

Recipe 负责：

- 组合默认 skills
- 启用对应扩展
- 设置推荐模型
- 设定系统指令

## MCP Extensions

首批建议 4 类：

1. `aiseesec-mcp`
2. `local-security-gateway-mcp`
3. `threat-intel-mcp`
4. `browser-assist-mcp`

### `aiseesec-mcp`

负责：

- 查询 AiseeSec 精选能力
- 拉取安全知识片段
- 执行查询类/检索类能力

V1 不要求把 AiseeSec 变成“在线技能市场平台”，只需要做精选能力接入。

### `local-security-gateway-mcp`

负责：

- 查询本地安全网关能力
- 调用内部查询接口
- 统一对接一些查询型安全系统

### `threat-intel-mcp`

负责：

- IOC 查询
- 情报摘要
- 域名/IP/Hash 快速调查

Goal 8 最小落地：

- 先做 Goose-native 本机预览版
- 不依赖外部厂商 API
- 通过本地 IOC 提取、启发式分析与 DNS 富化打通真实预览链路

### `browser-assist-mcp`

负责：

- 网页调查辅助
- 安全情报抓取辅助

Goal 8 最小落地：

- 先做 Goose-native 本机预览版
- 通过静态网页抓取或内联 HTML 分析提供页面摘要、表单/链接/脚本信号与 observable 提取
- 不执行页面脚本，不伪装成完整浏览器自动化平台

## AiseeSec 接入策略

V1 不做完整在线 marketplace，同步策略如下：

### 路线

- 从 AiseeSec 选出一组“精选能力”
- 映射为：
  - Skill
  - MCP
  - Recipe

### 原则

- 优先精选
- 不追求平台全量同步
- 不追求动态安装市场

这样最符合“效果优先”。

## 能力包目录建议

```text
distro/security-cn/
├── skills/
│   ├── vuln-triage/
│   ├── alert-triage/
│   ├── ioc-analysis/
│   ├── asset-risk-summary/
│   └── report-writing/
├── recipes/
│   ├── security-vuln-triage.yaml
│   ├── alert-investigation.yaml
│   └── web-investigation.yaml
├── extensions/
│   ├── bundled-extensions.security.json
│   ├── aiseesec-mcp/
│   ├── local-security-gateway-mcp/
│   ├── threat-intel-mcp/
│   └── browser-assist-mcp/
```

## Skill 编写规范

每个安全 skill 的 `SKILL.md` 应包含：

- `name`
- `description`
- 适用场景
- 输入定义
- 步骤
- 输出模板
- 风险与边界
- 验证步骤

安全 skill 必须偏“事务结果”，不要写成泛泛 prompt。

## 输出模板建议

例如 `vuln-triage`：

- 漏洞结论
- 利用条件
- 风险等级
- 关键证据
- 不确定项
- 建议下一步

这样更符合中国安全人员的工作习惯。

## 中国安全用户的能力入口建议

V1 默认入口不要写成很技术化的 Goose 术语，而要尽量面向任务：

- `漏洞研判`
- `告警分析`
- `IOC 研判`
- `网页调查`
- `报告生成`

把：

- skill
- MCP
- recipe

这些底层名词放到设置页或高级页，不要放到用户第一层入口。

## V1 不做的能力市场

不做：

- 大而全在线市场
- 用户自助上传任意插件
- 远端下载执行第三方未审查扩展
- 复杂安装审批系统

## V1 验收标准

- 安全场景能通过 Goose 原生能力拼出来
- AiseeSec 精选能力能通过 skill/MCP/recipe 方式落地
- 用户第一视角看到的是“任务入口”，不是一堆底层技术名词
- 不需要新建 capability runtime
