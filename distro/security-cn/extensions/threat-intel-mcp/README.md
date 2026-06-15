# threat-intel-mcp

Goal 8 已升级为真实本机可预览链路：

- `server.mjs` 是可运行的 Goose-native stdio MCP server
- 默认 `enabled: false`
- 提供 IOC 提取、启发式分析和本地 DNS 富化
- 不依赖外部 API key
- 不伪装成供应商情报源，只是 Goose-first 的本机预览实现
