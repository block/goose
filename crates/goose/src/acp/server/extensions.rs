use super::*;
use crate::agents::extension::Envs;
use crate::config::extensions::ExtensionEntry;
use agent_client_protocol::schema::{HttpHeader, McpServer, McpServerHttp, McpServerStdio};

impl GooseAcpAgent {
    pub(super) async fn on_add_extension(
        &self,
        req: AddExtensionRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        let session_id = &req.session_id;
        let config: ExtensionConfig = serde_json::from_value(req.config).map_err(|e| {
            agent_client_protocol::Error::invalid_params().data(format!("bad config: {e}"))
        })?;
        let agent = self.get_session_agent(&req.session_id, None).await?;
        agent
            .add_extension(config, session_id)
            .await
            .internal_err()?;
        Ok(EmptyResponse {})
    }

    pub(super) async fn on_remove_extension(
        &self,
        req: RemoveExtensionRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        let session_id = &req.session_id;
        let agent = self.get_session_agent(&req.session_id, None).await?;
        agent
            .remove_extension(&req.name, session_id)
            .await
            .internal_err()?;
        Ok(EmptyResponse {})
    }

    pub(super) async fn on_get_config_extensions(
        &self,
    ) -> Result<GetConfigExtensionsResponse, agent_client_protocol::Error> {
        let extensions = crate::config::extensions::get_all_extensions()
            .into_iter()
            .filter(|ext| {
                !crate::agents::extension_manager::is_hidden_extension(&ext.config.name())
            })
            .collect::<Vec<_>>();
        let warnings = crate::config::extensions::get_warnings();
        let extensions = extensions
            .into_iter()
            .map(config_entry_to_goose_entry)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        Ok(GetConfigExtensionsResponse {
            extensions,
            warnings,
        })
    }

    pub(super) async fn on_get_available_extensions(
        &self,
    ) -> Result<GetAvailableExtensionsResponse, agent_client_protocol::Error> {
        let extensions = crate::config::get_available_extensions()
            .into_iter()
            .map(|config| config_to_goose_extension(&config))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        Ok(GetAvailableExtensionsResponse { extensions })
    }

    pub(super) async fn on_add_config_extension(
        &self,
        req: AddConfigExtensionRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        let conversion = goose_extension_to_config(req.extension)?;

        Config::global()
            .set_secret_values(&conversion.secret_updates)
            .internal_err_ctx("Failed to save extension env secrets")?;

        crate::config::extensions::set_extension(ExtensionEntry {
            enabled: req.enabled,
            config: conversion.config,
        });
        Ok(EmptyResponse {})
    }

    pub(super) async fn on_remove_config_extension(
        &self,
        req: RemoveConfigExtensionRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        crate::config::extensions::remove_extension(&req.config_key);
        Ok(EmptyResponse {})
    }

    pub(super) async fn on_set_config_extension_enabled(
        &self,
        req: SetConfigExtensionEnabledRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        let updated =
            crate::config::extensions::set_extension_enabled(&req.config_key, req.enabled);
        if !updated {
            return Err(agent_client_protocol::Error::invalid_params()
                .data(format!("Extension '{}' not found", req.config_key)));
        }

        Ok(EmptyResponse {})
    }

    pub(super) async fn on_get_session_extensions(
        &self,
        req: GetSessionExtensionsRequest,
    ) -> Result<GetSessionExtensionsResponse, agent_client_protocol::Error> {
        let session_id = &req.session_id;
        let session = self
            .session_manager
            .get_session(session_id, false)
            .await
            .internal_err()?;

        let extensions = EnabledExtensionsState::extensions_or_default(
            Some(&session.extension_data),
            crate::config::Config::global(),
        );

        let extensions_json = extensions
            .into_iter()
            .map(|e| serde_json::to_value(&e))
            .collect::<Result<Vec<_>, _>>()
            .internal_err()?;

        Ok(GetSessionExtensionsResponse {
            extensions: extensions_json,
        })
    }
}

fn config_to_goose_extension(
    config: &ExtensionConfig,
) -> Result<Option<GooseExtension>, agent_client_protocol::Error> {
    let extension = match config {
        ExtensionConfig::Builtin {
            name,
            description,
            display_name,
            timeout,
            bundled,
            ..
        } => GooseExtension::Builtin {
            name: name.clone(),
            description: empty_string_to_none(description),
            display_name: display_name.clone(),
            timeout: *timeout,
            bundled: *bundled,
        },
        ExtensionConfig::Platform {
            name,
            description,
            display_name,
            bundled,
            ..
        } => GooseExtension::Platform {
            name: name.clone(),
            description: empty_string_to_none(description),
            display_name: display_name.clone(),
            bundled: *bundled,
        },
        ExtensionConfig::Stdio {
            name,
            description,
            cmd,
            args,
            env_keys,
            timeout,
            bundled,
            ..
        } => GooseExtension::Mcp {
            server: McpServer::Stdio(McpServerStdio::new(name, cmd).args(args.clone())),
            env_keys: env_keys.clone(),
            description: empty_string_to_none(description),
            timeout: *timeout,
            socket: None,
            bundled: *bundled,
        },
        ExtensionConfig::StreamableHttp {
            name,
            description,
            uri,
            env_keys,
            headers,
            timeout,
            socket,
            bundled,
            ..
        } => {
            let headers = headers
                .iter()
                .map(|(key, value)| HttpHeader::new(key, value))
                .collect();
            GooseExtension::Mcp {
                server: McpServer::Http(McpServerHttp::new(name, uri).headers(headers)),
                env_keys: env_keys.clone(),
                description: empty_string_to_none(description),
                timeout: *timeout,
                socket: socket.clone(),
                bundled: *bundled,
            }
        }
        ExtensionConfig::Frontend { .. }
        | ExtensionConfig::InlinePython { .. }
        | ExtensionConfig::Sse { .. } => return Ok(None),
    };
    Ok(Some(extension))
}

struct ConfigExtensionConversion {
    config: ExtensionConfig,
    secret_updates: Vec<(String, serde_json::Value)>,
}

fn goose_extension_to_config(
    extension: GooseExtension,
) -> Result<ConfigExtensionConversion, agent_client_protocol::Error> {
    let mut secret_updates = Vec::new();
    let config = match extension {
        GooseExtension::Builtin {
            name,
            description,
            display_name,
            timeout,
            bundled,
        } => ExtensionConfig::Builtin {
            name,
            description: description.unwrap_or_default(),
            display_name,
            timeout,
            bundled,
            available_tools: Vec::new(),
        },
        GooseExtension::Platform {
            name,
            description,
            display_name,
            bundled,
        } => ExtensionConfig::Platform {
            name,
            description: description.unwrap_or_default(),
            display_name,
            bundled,
            available_tools: Vec::new(),
        },
        GooseExtension::Mcp {
            server,
            env_keys,
            description,
            timeout,
            socket,
            bundled,
        } => match server {
            McpServer::Stdio(stdio) => {
                if socket.is_some() {
                    return Err(agent_client_protocol::Error::invalid_params()
                        .data("socket is only supported for streamable_http MCP extensions"));
                }
                let mut env_keys = env_keys;
                for env in stdio.env {
                    if !env_keys.contains(&env.name) {
                        env_keys.push(env.name.clone());
                    }
                    secret_updates.push((env.name, serde_json::Value::String(env.value)));
                }
                ExtensionConfig::Stdio {
                    name: stdio.name,
                    description: description.unwrap_or_default(),
                    cmd: stdio.command.to_string_lossy().to_string(),
                    args: stdio.args,
                    envs: Envs::default(),
                    env_keys,
                    timeout,
                    bundled,
                    available_tools: Vec::new(),
                }
            }
            McpServer::Http(http) => ExtensionConfig::StreamableHttp {
                name: http.name,
                description: description.unwrap_or_default(),
                uri: http.url,
                envs: Envs::default(),
                env_keys,
                headers: http
                    .headers
                    .into_iter()
                    .map(|header| (header.name, header.value))
                    .collect(),
                timeout,
                socket,
                bundled,
                available_tools: Vec::new(),
            },
            McpServer::Sse(_) => {
                return Err(agent_client_protocol::Error::invalid_params()
                    .data("SSE is unsupported, migrate to streamable_http"));
            }
            _ => {
                return Err(
                    agent_client_protocol::Error::invalid_params().data("unsupported MCP server")
                );
            }
        },
    };
    Ok(ConfigExtensionConversion {
        config,
        secret_updates,
    })
}

fn config_entry_to_goose_entry(
    entry: ExtensionEntry,
) -> Result<Option<GooseExtensionEntry>, agent_client_protocol::Error> {
    let config_key = entry.config.key();
    let Some(extension) = config_to_goose_extension(&entry.config)? else {
        return Ok(None);
    };
    Ok(Some(GooseExtensionEntry {
        extension,
        enabled: entry.enabled,
        config_key: Some(config_key),
    }))
}

fn empty_string_to_none(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::extension::Envs;
    use agent_client_protocol::schema::{McpServer, McpServerSse};
    use std::collections::HashMap;

    #[test]
    fn builtin_config_converts_to_goose_builtin_extension() {
        let config = ExtensionConfig::Builtin {
            name: "developer".to_string(),
            description: "Developer tools".to_string(),
            display_name: Some("Developer".to_string()),
            timeout: Some(30),
            bundled: Some(true),
            available_tools: vec!["shell".to_string()],
        };

        let extension = config_to_goose_extension(&config)
            .expect("conversion should succeed")
            .expect("builtin should be supported");

        let GooseExtension::Builtin {
            name,
            description,
            display_name,
            timeout,
            bundled,
        } = extension
        else {
            panic!("expected builtin extension");
        };

        assert_eq!(name, "developer");
        assert_eq!(description.as_deref(), Some("Developer tools"));
        assert_eq!(display_name.as_deref(), Some("Developer"));
        assert_eq!(timeout, Some(30));
        assert_eq!(bundled, Some(true));
    }

    #[test]
    fn platform_config_converts_to_goose_platform_extension() {
        let config = ExtensionConfig::Platform {
            name: "todo".to_string(),
            description: "Todo tools".to_string(),
            display_name: Some("Todo".to_string()),
            bundled: Some(true),
            available_tools: vec!["write_todos".to_string()],
        };

        let extension = config_to_goose_extension(&config)
            .expect("conversion should succeed")
            .expect("platform should be supported");

        let GooseExtension::Platform {
            name,
            description,
            display_name,
            bundled,
        } = extension
        else {
            panic!("expected platform extension");
        };

        assert_eq!(name, "todo");
        assert_eq!(description.as_deref(), Some("Todo tools"));
        assert_eq!(display_name.as_deref(), Some("Todo"));
        assert_eq!(bundled, Some(true));
    }

    #[test]
    fn stdio_config_converts_to_goose_mcp_extension_without_literal_envs() {
        let config = ExtensionConfig::Stdio {
            name: "test-stdio".to_string(),
            description: "Test stdio".to_string(),
            cmd: "test-command".to_string(),
            args: vec!["--flag".to_string(), "value".to_string()],
            envs: Envs::new(HashMap::from([(
                "SECRET_TOKEN".to_string(),
                "literal-secret".to_string(),
            )])),
            env_keys: vec!["SECRET_TOKEN".to_string()],
            timeout: Some(42),
            bundled: None,
            available_tools: vec![],
        };

        let extension = config_to_goose_extension(&config)
            .expect("conversion should succeed")
            .expect("stdio should be supported");

        let GooseExtension::Mcp {
            server,
            env_keys,
            description,
            timeout,
            socket,
            bundled,
        } = extension
        else {
            panic!("expected mcp extension");
        };

        assert_eq!(env_keys, vec!["SECRET_TOKEN"]);
        assert_eq!(description.as_deref(), Some("Test stdio"));
        assert_eq!(timeout, Some(42));
        assert_eq!(socket, None);
        assert_eq!(bundled, None);

        let McpServer::Stdio(stdio) = server else {
            panic!("expected stdio server");
        };

        assert_eq!(stdio.name, "test-stdio");
        assert_eq!(stdio.command.to_string_lossy(), "test-command");
        assert_eq!(stdio.args, vec!["--flag", "value"]);
        assert!(stdio.env.is_empty(), "literal envs should not be exposed");
    }

    #[test]
    fn streamable_http_config_converts_to_goose_mcp_extension_without_literal_envs() {
        let config = ExtensionConfig::StreamableHttp {
            name: "test-http".to_string(),
            description: "Test HTTP".to_string(),
            uri: "https://example.com/mcp".to_string(),
            envs: Envs::new(HashMap::from([(
                "API_TOKEN".to_string(),
                "literal-secret".to_string(),
            )])),
            env_keys: vec!["API_TOKEN".to_string()],
            headers: HashMap::from([(
                "Authorization".to_string(),
                "Bearer ${API_TOKEN}".to_string(),
            )]),
            timeout: Some(99),
            socket: Some("@egress.sock".to_string()),
            bundled: None,
            available_tools: vec![],
        };

        let extension = config_to_goose_extension(&config)
            .expect("conversion should succeed")
            .expect("streamable http should be supported");

        let GooseExtension::Mcp {
            server,
            env_keys,
            description,
            timeout,
            socket,
            bundled,
        } = extension
        else {
            panic!("expected mcp extension");
        };

        assert_eq!(env_keys, vec!["API_TOKEN"]);
        assert_eq!(description.as_deref(), Some("Test HTTP"));
        assert_eq!(timeout, Some(99));
        assert_eq!(socket.as_deref(), Some("@egress.sock"));
        assert_eq!(bundled, None);

        let McpServer::Http(http) = server else {
            panic!("expected http server");
        };

        assert_eq!(http.name, "test-http");
        assert_eq!(http.url, "https://example.com/mcp");
        assert_eq!(http.headers.len(), 1);
        assert_eq!(http.headers[0].name, "Authorization");
        assert_eq!(http.headers[0].value, "Bearer ${API_TOKEN}");
    }

    #[test]
    fn inline_python_config_is_skipped() {
        let config = ExtensionConfig::InlinePython {
            name: "python-tools".to_string(),
            description: "Python tools".to_string(),
            code: "print('hello')".to_string(),
            timeout: Some(12),
            dependencies: Some(vec!["requests".to_string()]),
            available_tools: vec!["fetch".to_string()],
        };

        let extension = config_to_goose_extension(&config).expect("conversion should succeed");

        assert!(extension.is_none());
    }

    #[test]
    fn frontend_config_is_skipped() {
        let tool = rmcp::model::Tool::new(
            "pick_color",
            "Pick a color",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "hex": { "type": "string" }
                }
            })
            .as_object()
            .expect("schema should be object")
            .clone(),
        );
        let config = ExtensionConfig::Frontend {
            name: "frontend-tools".to_string(),
            description: "Frontend tools".to_string(),
            tools: vec![tool],
            instructions: Some("Use frontend tools carefully".to_string()),
            bundled: None,
            available_tools: vec!["pick_color".to_string()],
        };

        let extension = config_to_goose_extension(&config).expect("conversion should succeed");

        assert!(extension.is_none());
    }

    #[test]
    fn sse_config_is_skipped() {
        let config = ExtensionConfig::Sse {
            name: "legacy-sse".to_string(),
            description: "Legacy SSE".to_string(),
            uri: Some("https://example.com/sse".to_string()),
        };

        let extension = config_to_goose_extension(&config).expect("conversion should succeed");

        assert!(extension.is_none());
    }

    #[test]
    fn goose_mcp_stdio_extension_converts_to_config_without_literal_envs() {
        let extension = GooseExtension::Mcp {
            server: McpServer::Stdio(
                McpServerStdio::new("test-stdio", "test-command")
                    .args(vec!["--flag".to_string(), "value".to_string()]),
            ),
            env_keys: vec!["SECRET_TOKEN".to_string()],
            description: Some("Test stdio".to_string()),
            timeout: Some(42),
            socket: None,
            bundled: Some(true),
        };

        let conversion = goose_extension_to_config(extension).expect("conversion should succeed");
        assert!(conversion.secret_updates.is_empty());

        let ExtensionConfig::Stdio {
            name,
            description,
            cmd,
            args,
            envs,
            env_keys,
            timeout,
            bundled,
            available_tools,
        } = conversion.config
        else {
            panic!("expected stdio config");
        };

        assert_eq!(name, "test-stdio");
        assert_eq!(description, "Test stdio");
        assert_eq!(cmd, "test-command");
        assert_eq!(args, vec!["--flag", "value"]);
        assert!(
            envs.get_env().is_empty(),
            "literal envs should not be persisted"
        );
        assert_eq!(env_keys, vec!["SECRET_TOKEN"]);
        assert_eq!(timeout, Some(42));
        assert_eq!(bundled, Some(true));
        assert!(available_tools.is_empty());
    }

    #[test]
    fn goose_mcp_stdio_extension_extracts_literal_envs_for_config_add() {
        let extension = GooseExtension::Mcp {
            server: McpServer::Stdio(McpServerStdio::new("test-stdio", "test-command").env(vec![
                agent_client_protocol::schema::EnvVariable::new("SECRET_TOKEN", "literal-secret"),
                agent_client_protocol::schema::EnvVariable::new("OTHER_TOKEN", "other-secret"),
            ])),
            env_keys: vec!["SECRET_TOKEN".to_string()],
            description: Some("Test stdio".to_string()),
            timeout: Some(42),
            socket: None,
            bundled: Some(true),
        };

        let conversion = goose_extension_to_config(extension).expect("conversion should succeed");

        assert_eq!(
            conversion.secret_updates,
            vec![
                (
                    "SECRET_TOKEN".to_string(),
                    serde_json::Value::String("literal-secret".to_string())
                ),
                (
                    "OTHER_TOKEN".to_string(),
                    serde_json::Value::String("other-secret".to_string())
                )
            ]
        );

        let ExtensionConfig::Stdio { envs, env_keys, .. } = conversion.config else {
            panic!("expected stdio config");
        };

        assert!(
            envs.get_env().is_empty(),
            "literal envs should not be persisted"
        );
        assert_eq!(env_keys, vec!["SECRET_TOKEN", "OTHER_TOKEN"]);
    }

    #[test]
    fn goose_mcp_streamable_http_extension_converts_to_config_without_literal_envs() {
        let extension = GooseExtension::Mcp {
            server: McpServer::Http(
                McpServerHttp::new("test-http", "https://example.com/mcp").headers(vec![
                    HttpHeader::new("Authorization", "Bearer ${API_TOKEN}"),
                ]),
            ),
            env_keys: vec!["API_TOKEN".to_string()],
            description: Some("Test HTTP".to_string()),
            timeout: Some(99),
            socket: Some("@egress.sock".to_string()),
            bundled: Some(true),
        };

        let conversion = goose_extension_to_config(extension).expect("conversion should succeed");
        assert!(conversion.secret_updates.is_empty());

        let ExtensionConfig::StreamableHttp {
            name,
            description,
            uri,
            envs,
            env_keys,
            headers,
            timeout,
            socket,
            bundled,
            available_tools,
        } = conversion.config
        else {
            panic!("expected streamable http config");
        };

        assert_eq!(name, "test-http");
        assert_eq!(description, "Test HTTP");
        assert_eq!(uri, "https://example.com/mcp");
        assert!(
            envs.get_env().is_empty(),
            "literal envs should not be persisted"
        );
        assert_eq!(env_keys, vec!["API_TOKEN"]);
        assert_eq!(
            headers,
            HashMap::from([(
                "Authorization".to_string(),
                "Bearer ${API_TOKEN}".to_string()
            )])
        );
        assert_eq!(timeout, Some(99));
        assert_eq!(socket.as_deref(), Some("@egress.sock"));
        assert_eq!(bundled, Some(true));
        assert!(available_tools.is_empty());
    }

    #[test]
    fn goose_builtin_extension_converts_to_config() {
        let builtin = GooseExtension::Builtin {
            name: "developer".to_string(),
            description: Some("Developer tools".to_string()),
            display_name: Some("Developer".to_string()),
            timeout: Some(30),
            bundled: Some(true),
        };

        let conversion = goose_extension_to_config(builtin).expect("conversion should succeed");
        assert!(conversion.secret_updates.is_empty());

        let ExtensionConfig::Builtin {
            name,
            description,
            display_name,
            timeout,
            bundled,
            available_tools,
        } = conversion.config
        else {
            panic!("expected builtin config");
        };

        assert_eq!(name, "developer");
        assert_eq!(description, "Developer tools");
        assert_eq!(display_name.as_deref(), Some("Developer"));
        assert_eq!(timeout, Some(30));
        assert_eq!(bundled, Some(true));
        assert!(available_tools.is_empty());
    }

    #[test]
    fn goose_platform_extension_converts_to_config() {
        let platform = GooseExtension::Platform {
            name: "todo".to_string(),
            description: Some("Todo tools".to_string()),
            display_name: Some("Todo".to_string()),
            bundled: Some(true),
        };

        let conversion = goose_extension_to_config(platform).expect("conversion should succeed");
        assert!(conversion.secret_updates.is_empty());

        let ExtensionConfig::Platform {
            name,
            description,
            display_name,
            bundled,
            available_tools,
        } = conversion.config
        else {
            panic!("expected platform config");
        };

        assert_eq!(name, "todo");
        assert_eq!(description, "Todo tools");
        assert_eq!(display_name.as_deref(), Some("Todo"));
        assert_eq!(bundled, Some(true));
        assert!(available_tools.is_empty());
    }

    #[test]
    fn goose_mcp_sse_extension_is_rejected_for_config_add() {
        let extension = GooseExtension::Mcp {
            server: McpServer::Sse(McpServerSse::new("legacy-sse", "https://example.com/sse")),
            env_keys: Vec::new(),
            description: None,
            timeout: None,
            socket: None,
            bundled: None,
        };

        assert!(goose_extension_to_config(extension).is_err());
    }
}
