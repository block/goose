use indoc::indoc;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, ErrorCode, ErrorData, Implementation, ListResourcesResult,
        PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResult, Resource,
        ServerCapabilities, ServerInfo,
    },
    schemars::JsonSchema,
    service::RequestContext,
    tool, tool_handler, tool_router, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct NmapScanParams {
    /// The target IP address or hostname to scan.
    pub target: String,
    /// Optional arguments to pass to nmap (e.g., "-sS -p-").
    pub options: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WiresharkCaptureParams {
    /// The network interface to capture on (e.g., "eth0", "wlan0").
    pub interface: String,
    /// The duration of the capture in seconds.
    pub duration: u64,
    /// The file path to save the capture (e.g., "/tmp/capture.pcap").
    pub output_file: String,
    /// Optional capture filter (e.g., "tcp port 80").
    pub filter: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MetasploitParams {
    /// The command to execute in msfconsole (e.g., "use exploit/windows/smb/ms08_067_netapi; set RHOSTS 192.168.1.1; check; exit").
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExploitSearchParams {
    /// The search query for Exploit-DB (e.g., "wordpress 5.0").
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ShodanSearchParams {
    /// The search query for Shodan (e.g., "apache").
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum RootkitTool {
    Chkrootkit,
    Rkhunter,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RootkitScanParams {
    /// The rootkit hunting tool to use.
    pub tool: RootkitTool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum SdrTool {
    RtlPower,
    RtlSdr,
    HackrfInfo,
    HackrfSweep,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SdrScanParams {
    /// The SDR tool to use.
    pub tool: SdrTool,
    /// Arguments for the tool (e.g., "-f 88M:108M:125k" for rtl_power).
    pub args: String,
}

#[derive(Clone)]
pub struct SecurityServer {
    tool_router: ToolRouter<Self>,
    instructions: String,
}

impl Default for SecurityServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl SecurityServer {
    pub fn new() -> Self {
        let instructions = indoc! {r#"
            You are a Security Assistant equipped with various network security and SDR tools.
            Use these tools responsibly and only on systems you have permission to test.

            Available tools:
            - nmap_scan: Network exploration and security auditing.
            - wireshark_capture: Network protocol analyzer (uses tshark).
            - metasploit_execute: Penetration testing framework (uses msfconsole).
            - exploit_search: Search for exploits in Exploit-DB (uses searchsploit).
            - shodan_search: Search Engine for the Internet of Things (uses shodan CLI).
            - rootkit_scan: Scan for rootkits (uses chkrootkit or rkhunter).
            - sdr_scan: Software Defined Radio tools (rtl-sdr, hackrf).
        "#}
        .to_string();

        Self {
            tool_router: Self::tool_router(),
            instructions,
        }
    }

    /// Run an Nmap scan against a target.
    #[tool(
        name = "nmap_scan",
        description = "Run an Nmap scan against a target to discover hosts and services."
    )]
    pub async fn nmap_scan(
        &self,
        params: Parameters<NmapScanParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let mut command = Command::new("nmap");
        command.arg(&params.target);

        if let Some(options) = params.options {
            // Split options by space, respecting quotes would be better but simple split for now
            // Security risk: this allows arbitrary arguments, but we assume the agent is trusted.
            // Using shell-words crate for proper parsing if available would be better.
            // But since we don't want to add dependencies right now, we trust the agent or use simple split.
            // Actually, we can use `shell-words` since it is in `Cargo.toml`.
            match shell_words::split(&options) {
                Ok(args) => {
                    command.args(args);
                }
                Err(e) => {
                    return Err(ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!("Failed to parse options: {}", e),
                        None,
                    ));
                }
            }
        }

        let output = command.output().map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to execute nmap: {}", e),
                None,
            )
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let result = format!("Stdout:\n{}\n\nStderr:\n{}", stdout, stderr);
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    /// Capture network traffic using Tshark (Wireshark).
    #[tool(
        name = "wireshark_capture",
        description = "Capture network traffic using Tshark. Requires tshark to be installed."
    )]
    pub async fn wireshark_capture(
        &self,
        params: Parameters<WiresharkCaptureParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let mut command = Command::new("tshark");
        command
            .arg("-i")
            .arg(&params.interface)
            .arg("-a")
            .arg(format!("duration:{}", params.duration))
            .arg("-w")
            .arg(&params.output_file);

        if let Some(filter) = params.filter {
            command.arg("-f").arg(filter);
        }

        let output = command.output().map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to execute tshark: {}", e),
                None,
            )
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let result = format!(
            "Capture saved to {}\n\nStdout:\n{}\n\nStderr:\n{}",
            params.output_file, stdout, stderr
        );
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    /// Execute Metasploit commands using msfconsole.
    #[tool(
        name = "metasploit_execute",
        description = "Execute Metasploit commands using msfconsole. Requires Metasploit Framework."
    )]
    pub async fn metasploit_execute(
        &self,
        params: Parameters<MetasploitParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let output = Command::new("msfconsole")
            .arg("-x")
            .arg(&params.command)
            .output()
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to execute msfconsole: {}", e),
                    None,
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let result = format!("Stdout:\n{}\n\nStderr:\n{}", stdout, stderr);
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    /// Search for exploits in Exploit-DB.
    #[tool(
        name = "exploit_search",
        description = "Search for exploits in Exploit-DB using searchsploit."
    )]
    pub async fn exploit_search(
        &self,
        params: Parameters<ExploitSearchParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let output = Command::new("searchsploit")
            .arg(&params.query)
            .output()
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to execute searchsploit: {}", e),
                    None,
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let result = format!("Stdout:\n{}\n\nStderr:\n{}", stdout, stderr);
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    /// Search Shodan using the CLI.
    #[tool(
        name = "shodan_search",
        description = "Search Shodan using the CLI. Requires shodan CLI to be installed and initialized."
    )]
    pub async fn shodan_search(
        &self,
        params: Parameters<ShodanSearchParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let output = Command::new("shodan")
            .arg("search")
            .arg(&params.query)
            .output()
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to execute shodan: {}", e),
                    None,
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let result = format!("Stdout:\n{}\n\nStderr:\n{}", stdout, stderr);
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    /// Scan for rootkits.
    #[tool(
        name = "rootkit_scan",
        description = "Scan for rootkits using chkrootkit or rkhunter."
    )]
    pub async fn rootkit_scan(
        &self,
        params: Parameters<RootkitScanParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let (cmd, args) = match params.tool {
            RootkitTool::Chkrootkit => ("chkrootkit", vec![]),
            RootkitTool::Rkhunter => ("rkhunter", vec!["--check", "--sk"]),
        };

        let output = Command::new(cmd).args(args).output().map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to execute {}: {}", cmd, e),
                None,
            )
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let result = format!("Stdout:\n{}\n\nStderr:\n{}", stdout, stderr);
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    /// use SDR tools.
    #[tool(
        name = "sdr_scan",
        description = "Use SDR tools like rtl_power, rtl_sdr, etc."
    )]
    pub async fn sdr_scan(
        &self,
        params: Parameters<SdrScanParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let cmd = match params.tool {
            SdrTool::RtlPower => "rtl_power",
            SdrTool::RtlSdr => "rtl_sdr",
            SdrTool::HackrfInfo => "hackrf_info",
            SdrTool::HackrfSweep => "hackrf_sweep",
        };

        let mut command = Command::new(cmd);

        match shell_words::split(&params.args) {
            Ok(args) => {
                command.args(args);
            }
            Err(e) => {
                return Err(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("Failed to parse args: {}", e),
                    None,
                ));
            }
        }

        let output = command.output().map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to execute {}: {}", cmd, e),
                None,
            )
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let result = format!("Stdout:\n{}\n\nStderr:\n{}", stdout, stderr);
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for SecurityServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "goose-security".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                title: Some("Security Tools".to_string()),
                description: Some("Network security and SDR tools".to_string()),
                icons: None,
                website_url: None,
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(self.instructions.clone()),
            ..Default::default()
        }
    }

    async fn list_resources(
        &self,
        _pagination: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        _params: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        Err(ErrorData::new(
            ErrorCode::INVALID_REQUEST,
            "No resources available".to_string(),
            None,
        ))
    }
}
