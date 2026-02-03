STDIN: {"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{"sampling":{},"elicitation":{}},"clientInfo":{"name":"goose-cli","version":"0.0.0"}}}
STDERR: 
STDERR: 
STDERR: ╭──────────────────────────────────────────────────────────────────────────────╮
STDERR: │                                                                              │
STDERR: │                                                                              │
STDERR: │                         ▄▀▀ ▄▀█ █▀▀ ▀█▀ █▀▄▀█ █▀▀ █▀█                        │
STDERR: │                         █▀  █▀█ ▄▄█  █  █ ▀ █ █▄▄ █▀▀                        │
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
STDERR: 
STDERR: 
STDERR: [01/23/26 15:56:13] INFO     Starting MCP server 'mymcp' with     server.py:2506
STDERR:                              transport 'stdio'                                  
STDOUT: {"jsonrpc":"2.0","id":0,"result":{"protocolVersion":"2025-03-26","capabilities":{"experimental":{},"prompts":{"listChanged":false},"resources":{"subscribe":false,"listChanged":false},"tools":{"listChanged":true},"tasks":{"list":{},"cancel":{},"requests":{"tools":{"call":{}},"prompts":{"get":{}},"resources":{"read":{}}}}},"serverInfo":{"name":"mymcp","version":"2.14.4"}}}
STDIN: {"jsonrpc":"2.0","method":"notifications/initialized"}
STDIN: {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"_meta":{"agent-session-id":"test-session-id","progressToken":0},"name":"divide","arguments":{"dividend":10,"divisor":2}}}
STDOUT: {"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"5.0"}],"structuredContent":{"result":5.0},"isError":false}}
