import subprocess
import socket
import asyncio
import sys
import shlex

try:
    from mcp.server.lowlevel import Server
    from mcp.types import Tool, TextContent, ImageContent, EmbeddedResource
except ImportError:
    # Fallback or try different import structure if mcp version varies
    try:
        from mcp.server import Server
        from mcp.types import Tool, TextContent
    except ImportError:
        print("Error: 'mcp' library not found. Please install it via pip.", file=sys.stderr)
        sys.exit(1)

# Initialize the MCP server
server = Server("redteam-tools")

@server.list_tools()
async def list_tools() -> list[Tool]:
    return [
        Tool(
            name="nmap_scan",
            description="Run an Nmap scan on the target.",
            inputSchema={
                "type": "object",
                "properties": {
                    "target": {"type": "string", "description": "The target IP or hostname."},
                    "options": {"type": "string", "description": "Nmap options (default: -sV)."}
                },
                "required": ["target"]
            }
        ),
        Tool(
            name="search_exploits",
            description="Search for exploits using searchsploit (exploit-db).",
            inputSchema={
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "The search query."}
                },
                "required": ["query"]
            }
        ),
        Tool(
            name="capture_traffic",
            description="Capture network traffic using tshark.",
            inputSchema={
                "type": "object",
                "properties": {
                    "interface": {"type": "string", "description": "Network interface (default: any)."},
                    "duration": {"type": "integer", "description": "Duration in seconds (default: 10)."},
                    "filter": {"type": "string", "description": "Capture filter."}
                },
                "required": []
            }
        ),
        Tool(
            name="simple_port_scan",
            description="Perform a simple TCP connect scan (Python implementation).",
            inputSchema={
                "type": "object",
                "properties": {
                    "target": {"type": "string", "description": "Target IP or hostname."},
                    "start_port": {"type": "integer", "description": "Start port (default: 1)."},
                    "end_port": {"type": "integer", "description": "End port (default: 1024)."}
                },
                "required": ["target"]
            }
        ),
        Tool(
            name="metasploit_command",
            description="Run a Metasploit console command non-interactively.",
            inputSchema={
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "The msfconsole command to run."}
                },
                "required": ["command"]
            }
        ),
        Tool(
            name="check_burpsuite",
            description="Check if Burp Suite is installed.",
            inputSchema={
                "type": "object",
                "properties": {},
                "required": []
            }
        )
    ]

@server.call_tool()
async def call_tool(name: str, arguments: dict) -> list[TextContent]:
    if name == "nmap_scan":
        target = arguments.get("target")
        options = arguments.get("options", "-sV")
        # Split options to handle multiple flags correctly
        cmd = ["nmap"] + shlex.split(options) + [target]
        return await run_process(cmd, timeout=600)

    elif name == "search_exploits":
        query = arguments.get("query")
        # searchsploit might need shell splitting if query is complex, but usually it's just words
        # safest is to treat query as single arg if possible, or split if multiple terms
        cmd = ["searchsploit"] + shlex.split(query)
        return await run_process(cmd)

    elif name == "capture_traffic":
        interface = arguments.get("interface", "any")
        duration = int(arguments.get("duration", 10))
        filter_str = arguments.get("filter", "")
        cmd = ["tshark", "-i", interface, "-a", f"duration:{duration}"]
        if filter_str:
            cmd.extend(["-f", filter_str])
        return await run_process(cmd, timeout=duration + 10)

    elif name == "simple_port_scan":
        target = arguments.get("target")
        start_port = int(arguments.get("start_port", 1))
        end_port = int(arguments.get("end_port", 1024))
        return await simple_port_scan_impl(target, start_port, end_port)

    elif name == "metasploit_command":
        command = arguments.get("command")
        # msfconsole -q -x "command; exit"
        cmd = ["msfconsole", "-q", "-x", f"{command}; exit"]
        return await run_process(cmd, timeout=300)

    elif name == "check_burpsuite":
        return await check_burpsuite_impl()

    else:
        return [TextContent(type="text", text=f"Unknown tool: {name}")]

async def run_process(cmd: list[str], timeout: int = 60) -> list[TextContent]:
    try:
        # cmd is a list of strings. subprocess.exec expects this.
        process = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE
        )

        try:
            stdout, stderr = await asyncio.wait_for(process.communicate(), timeout=timeout)
            output = stdout.decode(errors='replace') + stderr.decode(errors='replace')
            return [TextContent(type="text", text=output)]
        except asyncio.TimeoutError:
            try:
                process.kill()
            except ProcessLookupError:
                pass
            return [TextContent(type="text", text=f"Command timed out after {timeout} seconds.")]

    except Exception as e:
        return [TextContent(type="text", text=f"Error running command: {str(e)}")]

async def simple_port_scan_impl(target: str, start_port: int, end_port: int) -> list[TextContent]:
    open_ports = []
    try:
        # Resolve target first to avoid repeated lookups
        target_ip = socket.gethostbyname(target)
    except socket.gaierror:
        return [TextContent(type="text", text=f"Could not resolve hostname: {target}")]

    # Limit concurrency to avoid hitting file descriptor limits
    sem = asyncio.Semaphore(100)

    async def check_port(ip, port):
        async with sem:
            try:
                conn = asyncio.open_connection(ip, port)
                reader, writer = await asyncio.wait_for(conn, timeout=1.0)
                writer.close()
                await writer.wait_closed()
                return port
            except:
                return None

    tasks = [check_port(target_ip, p) for p in range(start_port, end_port + 1)]
    results = await asyncio.gather(*tasks)

    open_ports = [p for p in results if p is not None]

    if open_ports:
        return [TextContent(type="text", text=f"Open ports on {target} ({target_ip}): {', '.join(map(str, open_ports))}")]
    else:
        return [TextContent(type="text", text=f"No open ports found on {target} ({target_ip}) in range {start_port}-{end_port}.")]

async def check_burpsuite_impl() -> list[TextContent]:
    try:
        process = await asyncio.create_subprocess_exec(
            "which", "burpsuite",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE
        )
        await process.communicate()
        if process.returncode == 0:
            return [TextContent(type="text", text="Burp Suite is installed. You can launch it using 'burpsuite' (requires display).")]
        else:
            return [TextContent(type="text", text="Burp Suite is not in the PATH.")]
    except Exception as e:
        return [TextContent(type="text", text=f"Error checking Burp Suite: {str(e)}")]

if __name__ == "__main__":
    try:
        from mcp.server.stdio import stdio_server

        async def main():
            async with stdio_server() as (read_stream, write_stream):
                await server.run(read_stream, write_stream, server.create_initialization_options())

        asyncio.run(main())
    except ImportError:
        print("Error: mcp library (specifically mcp.server.stdio) not found. Please install it.", file=sys.stderr)
