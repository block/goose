use super::*;
use agent_client_protocol::schema::{HttpHeader, McpServerHttp, McpServerStdio};

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
    use agent_client_protocol::schema::McpServer;
    use rmcp::model::Tool;
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
        } = extension
        else {
            panic!("expected builtin extension");
        };

        assert_eq!(name, "developer");
        assert_eq!(description.as_deref(), Some("Developer tools"));
        assert_eq!(display_name.as_deref(), Some("Developer"));
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
        } = extension
        else {
            panic!("expected platform extension");
        };

        assert_eq!(name, "todo");
        assert_eq!(description.as_deref(), Some("Todo tools"));
        assert_eq!(display_name.as_deref(), Some("Todo"));
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
        } = extension
        else {
            panic!("expected mcp extension");
        };

        assert_eq!(env_keys, vec!["SECRET_TOKEN"]);
        assert_eq!(description.as_deref(), Some("Test stdio"));
        assert_eq!(timeout, Some(42));
        assert_eq!(socket, None);

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
        } = extension
        else {
            panic!("expected mcp extension");
        };

        assert_eq!(env_keys, vec!["API_TOKEN"]);
        assert_eq!(description.as_deref(), Some("Test HTTP"));
        assert_eq!(timeout, Some(99));
        assert_eq!(socket.as_deref(), Some("@egress.sock"));

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
    fn inline_python_config_converts_to_goose_inline_python_extension() {
        let config = ExtensionConfig::InlinePython {
            name: "python-tools".to_string(),
            description: "Python tools".to_string(),
            code: "print('hello')".to_string(),
            timeout: Some(12),
            dependencies: Some(vec!["requests".to_string()]),
            available_tools: vec!["fetch".to_string()],
        };

        let extension = config_to_goose_extension(&config)
            .expect("conversion should succeed")
            .expect("inline python should be supported");

        let GooseExtension::InlinePython {
            name,
            description,
            code,
            timeout,
            dependencies,
        } = extension
        else {
            panic!("expected inline python extension");
        };

        assert_eq!(name, "python-tools");
        assert_eq!(description.as_deref(), Some("Python tools"));
        assert_eq!(code, "print('hello')");
        assert_eq!(timeout, Some(12));
        assert_eq!(dependencies, vec!["requests"]);
    }

    #[test]
    fn frontend_config_converts_to_goose_frontend_extension() {
        let tool = Tool::new(
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

        let extension = config_to_goose_extension(&config)
            .expect("conversion should succeed")
            .expect("frontend should be supported");

        let GooseExtension::Frontend {
            name,
            description,
            tools,
            instructions,
        } = extension
        else {
            panic!("expected frontend extension");
        };

        assert_eq!(name, "frontend-tools");
        assert_eq!(description.as_deref(), Some("Frontend tools"));
        assert_eq!(
            instructions.as_deref(),
            Some("Use frontend tools carefully")
        );
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "pick_color");
        assert_eq!(tools[0]["description"], "Pick a color");
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
}

fn config_to_goose_extension(
    config: &ExtensionConfig,
) -> Result<Option<GooseExtension>, agent_client_protocol::Error> {
    let extension = match config {
        ExtensionConfig::Builtin {
            name,
            description,
            display_name,
            ..
        } => GooseExtension::Builtin {
            name: name.clone(),
            description: empty_string_to_none(description),
            display_name: display_name.clone(),
        },
        ExtensionConfig::Platform {
            name,
            description,
            display_name,
            ..
        } => GooseExtension::Platform {
            name: name.clone(),
            description: empty_string_to_none(description),
            display_name: display_name.clone(),
        },
        ExtensionConfig::Stdio {
            name,
            description,
            cmd,
            args,
            env_keys,
            timeout,
            ..
        } => GooseExtension::Mcp {
            server: McpServer::Stdio(McpServerStdio::new(name, cmd).args(args.clone())),
            env_keys: env_keys.clone(),
            description: empty_string_to_none(description),
            timeout: *timeout,
            socket: None,
        },
        ExtensionConfig::StreamableHttp {
            name,
            description,
            uri,
            env_keys,
            headers,
            timeout,
            socket,
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
            }
        }
        ExtensionConfig::Frontend {
            name,
            description,
            tools,
            instructions,
            ..
        } => {
            let tools = tools
                .iter()
                .map(serde_json::to_value)
                .collect::<Result<Vec<_>, _>>()
                .internal_err()?;
            GooseExtension::Frontend {
                name: name.clone(),
                description: empty_string_to_none(description),
                tools,
                instructions: instructions.clone(),
            }
        }
        ExtensionConfig::InlinePython {
            name,
            description,
            code,
            timeout,
            dependencies,
            ..
        } => GooseExtension::InlinePython {
            name: name.clone(),
            description: empty_string_to_none(description),
            code: code.clone(),
            timeout: *timeout,
            dependencies: dependencies.clone().unwrap_or_default(),
        },
        ExtensionConfig::Sse { .. } => return Ok(None),
    };
    Ok(Some(extension))
}

fn config_entry_to_goose_entry(
    entry: crate::config::extensions::ExtensionEntry,
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

    pub(super) async fn on_add_config_extension(
        &self,
        req: AddConfigExtensionRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        let mut obj = match req.extension_config {
            serde_json::Value::Object(obj) => obj,
            _ => {
                return Err(agent_client_protocol::Error::invalid_params()
                    .data("extensionConfig must be a JSON object"));
            }
        };
        obj.insert(
            "name".to_string(),
            serde_json::Value::String(req.name.clone()),
        );

        let config: crate::agents::ExtensionConfig =
            serde_json::from_value(serde_json::Value::Object(obj)).map_err(|e| {
                agent_client_protocol::Error::invalid_params().data(format!("bad config: {e}"))
            })?;

        crate::config::extensions::set_extension(crate::config::extensions::ExtensionEntry {
            enabled: req.enabled,
            config,
        });
        Ok(EmptyResponse {})
    }

    pub(super) async fn on_remove_config_extension(
        &self,
        req: RemoveConfigExtensionRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        let keys = crate::config::extensions::get_all_extension_names();
        if !keys.iter().any(|k| k == &req.config_key) {
            return Err(agent_client_protocol::Error::invalid_params()
                .data(format!("Extension '{}' not found", req.config_key)));
        }
        crate::config::extensions::remove_extension(&req.config_key);
        Ok(EmptyResponse {})
    }

    pub(super) async fn on_toggle_config_extension(
        &self,
        req: ToggleConfigExtensionRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        let keys = crate::config::extensions::get_all_extension_names();
        if !keys.iter().any(|k| k == &req.config_key) {
            return Err(agent_client_protocol::Error::invalid_params()
                .data(format!("Extension '{}' not found", req.config_key)));
        }
        crate::config::extensions::set_extension_enabled(&req.config_key, req.enabled);
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
