use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use mcp_core::Tool;
use serde_json::{json, Value};
use std::io::{self, BufRead, BufWriter, Write};

#[derive(Clone)]
struct MockMcpServer {
    tools: Vec<Tool>,
}

impl MockMcpServer {
    fn new(tools: Vec<Tool>) -> Self {
        Self { tools }
    }

    fn handle_request(&self, request: Value) -> Value {
        let method = request["method"].as_str().unwrap_or("");
        let id = request.get("id").cloned();

        match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "mock-mcp-server",
                        "version": "1.0.0"
                    }
                }
            }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": self.tools
                }
            }),
            "tools/call" => {
                let tool_name = request["params"]["name"].as_str().unwrap_or("unknown");
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": format!("[MOCK] Tool {} executed with mock result", tool_name)
                        }]
                    }
                })
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": "Method not found"
                }
            }),
        }
    }
}

fn create_fallback_tools(extension_name: &str) -> Vec<Tool> {
    vec![
        Tool::new(
            format!("{}_tool_1", extension_name),
            format!("Mock tool 1 for {}", extension_name),
            json!({"type": "object", "properties": {}}),
            None,
        ),
        Tool::new(
            format!("{}_tool_2", extension_name),
            format!("Mock tool 2 for {}", extension_name),
            json!({"type": "object", "properties": {}}),
            None,
        ),
    ]
}

fn main() -> io::Result<()> {
    // Get extension name from environment variable
    let extension_name =
        std::env::var("EXTENSION_NAME").unwrap_or_else(|_| "mock_extension".to_string());

    // Get actual tools from environment variable or create mock tools
    let tools = if let Ok(tools_base64) = std::env::var("EXTENSION_TOOLS") {
        // Decode and deserialize the actual tools from the dataset
        match BASE64_STANDARD.decode(tools_base64) {
            Ok(tools_json_bytes) => match std::str::from_utf8(&tools_json_bytes) {
                Ok(tools_json) => match serde_json::from_str::<Vec<Tool>>(tools_json) {
                    Ok(tools) => tools,
                    Err(_) => create_fallback_tools(&extension_name),
                },
                Err(_) => create_fallback_tools(&extension_name),
            },
            Err(_) => create_fallback_tools(&extension_name),
        }
    } else {
        create_fallback_tools(&extension_name)
    };

    let server = MockMcpServer::new(tools);
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout);

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(request) = serde_json::from_str::<Value>(&line) {
            let response = server.handle_request(request);
            let response_str = serde_json::to_string(&response).unwrap();
            writeln!(writer, "{}", response_str)?;
            writer.flush()?;
        }
    }

    Ok(())
}
