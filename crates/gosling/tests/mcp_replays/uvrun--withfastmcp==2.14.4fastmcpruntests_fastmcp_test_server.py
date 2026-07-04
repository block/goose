STDIN: {"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{"extensions":{"io.modelcontextprotocol/ui":{"mimeTypes":["text/html;profile=mcp-app"]}},"roots":{},"sampling":{},"elicitation":{}},"clientInfo":{"name":"gosling-desktop","version":"0.0.0"}}}
STDERR: Downloading pygments (1.2MiB)
STDERR: Downloading pydantic-core (1.9MiB)
STDERR: Downloading lupa (1.2MiB)
STDERR: Downloading beartype (1.3MiB)
STDERR:  Downloaded lupa
STDERR:  Downloaded pygments
STDERR:  Downloaded beartype
STDERR:  Downloaded pydantic-core
STDERR: Installed 79 packages in 81ms
STDERR: /Users/eric/.cache/uv/archive-v0/NlBLSuvSpsOvRahr/lib/python3.14/site-packages/fastmcp/server/auth/providers/jwt.py:10: AuthlibDeprecationWarning: authlib.jose module is deprecated, please use joserfc instead.
STDERR: It will be compatible before version 2.0.0.
STDERR:   from authlib.jose import JsonWebKey, JsonWebToken
STDERR: 
STDERR: 
STDERR: ╭──────────────────────────────────────────────────────────────────────────────╮
STDERR: │                                                                              │
STDERR: │                                                                              │
STDERR: │                         ▄▀▀ ▄▀█ █▀▀ ▀█▀ █▀▄▀█ █▀▀ █▀█                        │
STDERR: │                         █▀  █▀█ ▄▄█  █  █ ▀ █ █▄▄ █▀▀                        │
STDERR: │                                                                              │
STDERR: │                                                                              │
STDERR: │                                                                              │
STDERR: │                                FastMCP 2.14.4                                │
STDERR: │                            https://gofastmcp.com                             │
STDERR: │                                                                              │
STDERR: │                    🖥  Server:      mymcp                                     │
STDERR: │                    🚀 Deploy free: https://fastmcp.cloud                     │
STDERR: │                                                                              │
STDERR: ╰──────────────────────────────────────────────────────────────────────────────╯
STDERR: ╭──────────────────────────────────────────────────────────────────────────────╮
STDERR: │                          ✨ FastMCP 3.0 is coming!                           │
STDERR: │       Pin `fastmcp < 3` in production, then upgrade when you're ready.       │
STDERR: ╰──────────────────────────────────────────────────────────────────────────────╯
STDERR: ╭──────────────────────────────────────────────────────────────────────────────╮
STDERR: │                          🎉 Update available: 3.4.2                          │
STDERR: │                      Run: pip install --upgrade fastmcp                      │
STDERR: ╰──────────────────────────────────────────────────────────────────────────────╯
STDERR: 
STDERR: 
STDERR: [07/04/26 17:28:41] INFO     Starting MCP server 'mymcp' with     server.py:2506
STDERR:                              transport 'stdio'                                  
STDERR: /Users/eric/.cache/uv/archive-v0/NlBLSuvSpsOvRahr/lib/python3.14/site-packages/redis/asyncio/connection.py:1628: DeprecationWarning: FakeConnection is deprecated. Use FakeAsyncRedisConnection instead
STDERR:   return self.connection_class(**self.connection_kwargs)
STDOUT: {"jsonrpc":"2.0","id":0,"result":{"protocolVersion":"2025-03-26","capabilities":{"experimental":{},"prompts":{"listChanged":false},"resources":{"subscribe":false,"listChanged":false},"tools":{"listChanged":true},"tasks":{"list":{},"cancel":{},"requests":{"tools":{"call":{}},"prompts":{"get":{}},"resources":{"read":{}}}}},"serverInfo":{"name":"mymcp","version":"2.14.4"}}}
STDIN: {"jsonrpc":"2.0","method":"notifications/initialized"}
STDIN: {"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{"agent-session-id":"test-session-id","progressToken":0}}}
STDOUT: {"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"divide","description":"Divide two numbers and return the result.","inputSchema":{"properties":{"dividend":{"description":"Dividend/numerator of the division.","type":"number"},"divisor":{"description":"Divisor/denominator of the division.","type":"number"}},"required":["dividend","divisor"],"type":"object"},"outputSchema":{"description":"Generic wrapper for non-object return types.","properties":{"result":{"type":"number"}},"required":["result"],"type":"object","x-fastmcp-wrap-result":true},"_meta":{"_fastmcp":{"tags":[]}}}]}}
STDIN: {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"_meta":{"agent-session-id":"test-session-id","agent-tool-call-request-id":"test-id","progressToken":1},"name":"divide","arguments":{"dividend":10,"divisor":2}}}
STDOUT: {"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"5.0"}],"structuredContent":{"result":5.0},"isError":false}}
