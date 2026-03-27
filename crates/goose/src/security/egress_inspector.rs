use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use std::sync::OnceLock;

use crate::config::GooseMode;
use crate::conversation::message::{Message, ToolRequest};
use crate::tool_inspection::{InspectionAction, InspectionResult, ToolInspector};

pub struct EgressInspector;

impl EgressInspector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EgressInspector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct EgressDestination {
    kind: String,
    destination: String,
    domain: String,
}

fn extract_destinations(command: &str) -> Vec<EgressDestination> {
    let mut destinations = Vec::new();

    static URL_RE: OnceLock<Regex> = OnceLock::new();
    let url_re = URL_RE.get_or_init(|| {
        Regex::new(r"(?i)(https?|ftp)://[^\s'\"<>|;&)]+").unwrap()
    });
    for cap in url_re.find_iter(command) {
        let url = cap.as_str().to_string();
        let domain = extract_domain_from_url(&url).unwrap_or_default();
        if !domain.is_empty() {
            destinations.push(EgressDestination {
                kind: "url".to_string(),
                destination: url,
                domain,
            });
        }
    }

    static GIT_SSH_RE: OnceLock<Regex> = OnceLock::new();
    let git_ssh_re = GIT_SSH_RE.get_or_init(|| {
        Regex::new(r"git@([^:]+):([^\s'\"]+)").unwrap()
    });
    for cap in git_ssh_re.captures_iter(command) {
        let domain = cap[1].to_string();
        let path = cap[2].to_string();
        destinations.push(EgressDestination {
            kind: "git_remote".to_string(),
            destination: format!("git@{}:{}", domain, path),
            domain,
        });
    }

    static S3_RE: OnceLock<Regex> = OnceLock::new();
    let s3_re = S3_RE.get_or_init(|| {
        Regex::new(r"s3://([^/\s'\"]+)(/[^\s'\"]*)?").unwrap()
    });
    for cap in s3_re.captures_iter(command) {
        let bucket = cap[1].to_string();
        let full = cap[0].to_string();
        destinations.push(EgressDestination {
            kind: "s3_bucket".to_string(),
            destination: full,
            domain: format!("{}.s3.amazonaws.com", bucket),
        });
    }

    static GCS_RE: OnceLock<Regex> = OnceLock::new();
    let gcs_re = GCS_RE.get_or_init(|| {
        Regex::new(r"gs://([^/\s'\"]+)(/[^\s'\"]*)?").unwrap()
    });
    for cap in gcs_re.captures_iter(command) {
        let bucket = cap[1].to_string();
        let full = cap[0].to_string();
        destinations.push(EgressDestination {
            kind: "gcs_bucket".to_string(),
            destination: full,
            domain: format!("{}.storage.googleapis.com", bucket),
        });
    }

    static SCP_RE: OnceLock<Regex> = OnceLock::new();
    let scp_re = SCP_RE.get_or_init(|| {
        Regex::new(r"(?:scp|rsync)\s+.*?(?:\S+@)?([a-zA-Z0-9][\w.-]+):").unwrap()
    });
    for cap in scp_re.captures_iter(command) {
        let host = cap[1].to_string();
        destinations.push(EgressDestination {
            kind: "scp_target".to_string(),
            destination: cap[0].to_string(),
            domain: host,
        });
    }

    static SSH_RE: OnceLock<Regex> = OnceLock::new();
    let ssh_re = SSH_RE.get_or_init(|| {
        Regex::new(r"ssh\s+(?:-[^\s]+\s+)*(?:\S+@)?([a-zA-Z0-9][\w.-]+)").unwrap()
    });
    for cap in ssh_re.captures_iter(command) {
        let host = cap[1].to_string();
        if !host.starts_with('-') {
            destinations.push(EgressDestination {
                kind: "ssh_target".to_string(),
                destination: cap[0].to_string(),
                domain: host,
            });
        }
    }

    static DOCKER_RE: OnceLock<Regex> = OnceLock::new();
    let docker_re = DOCKER_RE.get_or_init(|| {
        Regex::new(r"docker\s+(?:push|login)\s+(?:--[^\s]+\s+)*([^\s'\"]+)").unwrap()
    });
    for cap in docker_re.captures_iter(command) {
        let target = cap[1].to_string();
        let domain = target.split('/').next().unwrap_or(&target).to_string();
        destinations.push(EgressDestination {
            kind: "docker_registry".to_string(),
            destination: target,
            domain,
        });
    }

    if command.contains("npm publish") || command.contains("cargo publish") {
        destinations.push(EgressDestination {
            kind: "package_publish".to_string(),
            destination: command.to_string(),
            domain: if command.contains("npm") {
                "registry.npmjs.org".to_string()
            } else {
                "crates.io".to_string()
            },
        });
    }

    destinations
}

fn extract_domain_from_url(url: &str) -> Option<String> {
    let after_scheme = url.find("://").map(|i| &url[i + 3..]).unwrap_or(url);
    let host_port = after_scheme.split('/').next()?;
    let host = if host_port.contains('[') {
        host_port.split(']').next().map(|s| s.trim_start_matches('['))?
    } else {
        host_port.split(':').next()?
    };
    let domain = host.split('@').last()?;
    if domain.is_empty() { None } else { Some(domain.to_string()) }
}

fn is_shell_tool(name: &str) -> bool {
    matches!(
        name,
        "shell"
            | "bash"
            | "developer__shell"
            | "developer__bash"
            | "execute_command"
            | "run_command"
            | "terminal"
    )
}

fn extract_command(tool_call: &rmcp::model::CallToolRequestParams) -> Option<String> {
    tool_call
        .arguments
        .as_ref()
        .and_then(|args| args.get("command"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[async_trait]
impl ToolInspector for EgressInspector {
    fn name(&self) -> &'static str {
        "egress"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn inspect(
        &self,
        _session_id: &str,
        tool_requests: &[ToolRequest],
        _messages: &[Message],
        _goose_mode: GooseMode,
    ) -> Result<Vec<InspectionResult>> {
        let mut results = Vec::new();

        for tool_request in tool_requests {
            let tool_call = match &tool_request.tool_call {
                Ok(tc) => tc,
                Err(_) => continue,
            };

            if !is_shell_tool(tool_call.name.as_ref()) {
                continue;
            }

            let command = match extract_command(tool_call) {
                Some(c) => c,
                None => continue,
            };

            let destinations = extract_destinations(&command);

            if destinations.is_empty() {
                continue;
            }

            for dest in &destinations {
                tracing::info!(
                    monotonic_counter.goose.egress_destination = 1,
                    egress_kind = dest.kind.as_str(),
                    destination = dest.destination.as_str(),
                    domain = dest.domain.as_str(),
                    "Egress destination detected"
                );
            }

            results.push(InspectionResult {
                tool_request_id: tool_request.id.clone(),
                action: InspectionAction::Allow,
                reason: format!(
                    "Egress destinations detected: {}",
                    dest_summary.join(", ")
                ),
                confidence: 0.0,
                inspector_name: self.name().to_string(),
                finding_id: None,
            });
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_urls() {
        let dests = extract_destinations("curl https://example.com/api/data");
        assert_eq!(dests.len(), 1);
        assert_eq!(dests[0].kind, "url");
        assert_eq!(dests[0].destination, "https://example.com/api/data");
        assert_eq!(dests[0].domain, "example.com");
    }

    #[test]
    fn test_extract_multiple_urls() {
        let dests = extract_destinations(
            "curl https://api.github.com/repos && wget http://evil.com/data",
        );
        assert_eq!(dests.len(), 2);
        assert_eq!(dests[0].domain, "api.github.com");
        assert_eq!(dests[1].domain, "evil.com");
    }

    #[test]
    fn test_extract_git_ssh_remote() {
        let dests = extract_destinations("git remote add origin git@github.com:personal/repo.git");
        assert_eq!(dests.len(), 1);
        assert_eq!(dests[0].kind, "git_remote");
        assert_eq!(dests[0].domain, "github.com");
        assert_eq!(dests[0].destination, "git@github.com:personal/repo.git");
    }

    #[test]
    fn test_extract_s3_bucket() {
        let dests = extract_destinations("aws s3 cp data.csv s3://my-bucket/path/data.csv");
        assert_eq!(dests.len(), 1);
        assert_eq!(dests[0].kind, "s3_bucket");
        assert_eq!(dests[0].domain, "my-bucket.s3.amazonaws.com");
    }

    #[test]
    fn test_extract_gcs_bucket() {
        let dests = extract_destinations("gsutil cp data.csv gs://my-bucket/path/");
        assert_eq!(dests.len(), 1);
        assert_eq!(dests[0].kind, "gcs_bucket");
        assert_eq!(dests[0].domain, "my-bucket.storage.googleapis.com");
    }

    #[test]
    fn test_extract_scp_target() {
        let dests = extract_destinations("scp file.txt user@remote-host.com:/tmp/");
        assert_eq!(dests.len(), 1);
        assert_eq!(dests[0].kind, "scp_target");
        assert_eq!(dests[0].domain, "remote-host.com");
    }

    #[test]
    fn test_extract_ssh_target() {
        let dests = extract_destinations("ssh admin@production-server.internal");
        assert_eq!(dests.len(), 1);
        assert_eq!(dests[0].kind, "ssh_target");
        assert_eq!(dests[0].domain, "production-server.internal");
    }

    #[test]
    fn test_extract_docker_push() {
        let dests = extract_destinations("docker push registry.example.com/myapp:latest");
        assert_eq!(dests.len(), 1);
        assert_eq!(dests[0].kind, "docker_registry");
        assert_eq!(dests[0].domain, "registry.example.com");
    }

    #[test]
    fn test_extract_npm_publish() {
        let dests = extract_destinations("npm publish");
        assert_eq!(dests.len(), 1);
        assert_eq!(dests[0].kind, "package_publish");
        assert_eq!(dests[0].domain, "registry.npmjs.org");
    }

    #[test]
    fn test_no_destinations() {
        let dests = extract_destinations("ls -la /tmp");
        assert_eq!(dests.len(), 0);
    }

    #[test]
    fn test_git_push_with_https_remote() {
        let dests =
            extract_destinations("git push https://github.com/personal/secret-repo.git main");
        assert_eq!(dests.len(), 1);
        assert_eq!(dests[0].kind, "url");
        assert_eq!(dests[0].domain, "github.com");
        assert!(dests[0].destination.contains("personal/secret-repo"));
    }

    #[test]
    fn test_extract_domain_from_url() {
        assert_eq!(
            extract_domain_from_url("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain_from_url("https://user:pass@example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain_from_url("https://example.com:8080/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain_from_url("http://api.github.com/repos/squareup/goose"),
            Some("api.github.com".to_string())
        );
    }

}
