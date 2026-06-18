# Extensions source directory

这里保存安全 MCP wrapper 与 bundled extension catalog 的发行源文件。

Goose 当前的桌面内置扩展目录在：

- `ui/desktop/src/built-in-extensions.json`
- `ui/desktop/src/components/settings/extensions/bundled-extensions.json`

Goal 5 的最小同步策略：

- `bundled-extensions.security.json.example` 继续作为发行源文件
- 只同步到 `ui/desktop/src/components/settings/extensions/bundled-extensions.json`
- 默认全部 `enabled: false`
- 真实可预览链路与 stub/blocker 在这里并存，由 catalog + README 明确区分

不修改 Goose core，不提前落真实网关或高权限写操作。

约定：

- `name` 使用稳定的机器名，例如 `aiseesec-mcp`
- `display_name` 使用用户可见名称

## Goal 8 当前状态

- `browser-assist-mcp`
  - 已是 Goose-native 本机可预览链路
  - 通过本地 stdio MCP server 提供静态网页抓取、页面摘要、可疑特征与 observable 提取
  - 只读，不执行页面脚本，不登录，不写目标系统
- `threat-intel-mcp`
  - 已是 Goose-native 本机可预览链路
  - 通过本地 stdio MCP server 提供 IOC 提取、启发式分析与 DNS 富化
  - 不伪装成供应商情报源，不依赖外部密钥
- `aiseesec-mcp`
  - 仍为 disabled stub
  - blocker：需要外部专有 API / 账号 / 服务协议，当前仓库不能假装为本机可用
- `local-security-gateway-mcp`
  - 仍为 disabled stub
  - blocker：Goal 8 明确不实现真实 gateway

## 运行时说明

- 发行源 catalog 继续保留相对路径，如 `distro/security-cn/extensions/.../server.mjs`
- desktop 在同步 bundled extension 到 Goose config 时，会把这几条 security stdio 脚本路径解析为绝对路径
  原因：Goose 启动 extension 子进程时 `cwd` 跟随 session 工作目录，相对路径不能稳定工作
- 不新增 loader，不改 Goose extension schema，只复用现有 stdio MCP 入口
