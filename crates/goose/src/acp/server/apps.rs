use super::*;
use crate::config::paths::Paths;
use crate::goose_apps::{fetch_mcp_apps, mark_deletable_apps, GooseApp, McpAppCache};

const APPS_EXTENSION_NAME: &str = "apps";

impl GooseAcpAgent {
    pub(super) async fn on_list_apps(
        &self,
        req: AppsListRequest,
    ) -> Result<AppsListResponse, agent_client_protocol::Error> {
        let cache = McpAppCache::new().ok();

        let Some(session_id) = req.session_id else {
            let mut apps = cache
                .as_ref()
                .and_then(|cache| cache.list_apps().ok())
                .unwrap_or_default();
            mark_deletable_apps(&mut apps);
            return Ok(AppsListResponse {
                apps: apps_to_values(apps)?,
            });
        };

        let agent = self.get_session_agent(&session_id).await?;
        let mut apps = fetch_mcp_apps(&agent.extension_manager, &session_id)
            .await
            .map_err(|error| {
                agent_client_protocol::Error::internal_error()
                    .data(format!("Failed to list apps: {}", error.message))
            })?;

        if let Some(cache) = cache.as_ref() {
            let active_extensions = apps
                .iter()
                .flat_map(|app| app.mcp_servers.iter().cloned())
                .collect::<std::collections::HashSet<_>>();

            for extension_name in active_extensions {
                if let Err(error) = cache.delete_extension_apps(&extension_name) {
                    warn!(
                        extension_name,
                        %error,
                        "Failed to clean MCP app cache for extension"
                    );
                }
            }

            for app in &apps {
                if let Err(error) = cache.store_app(app) {
                    warn!(app = %app.resource.name, %error, "Failed to cache MCP app");
                }
            }
        }

        mark_deletable_apps(&mut apps);

        Ok(AppsListResponse {
            apps: apps_to_values(apps)?,
        })
    }

    pub(super) async fn on_export_app(
        &self,
        req: AppsExportRequest,
    ) -> Result<AppsExportResponse, agent_client_protocol::Error> {
        let cache = McpAppCache::new().internal_err_ctx("Failed to access app cache")?;
        let apps = cache.list_apps().internal_err_ctx("Failed to list apps")?;

        let app = apps
            .into_iter()
            .find(|app| app.resource.name == req.name)
            .ok_or_else(|| {
                agent_client_protocol::Error::resource_not_found(Some(req.name.clone()))
                    .data(format!("App '{}' not found", req.name))
            })?;

        let html = app
            .to_html()
            .map_err(|error| agent_client_protocol::Error::internal_error().data(error))?;
        Ok(AppsExportResponse { html })
    }

    pub(super) async fn on_import_app(
        &self,
        req: AppsImportRequest,
    ) -> Result<AppsImportResponse, agent_client_protocol::Error> {
        let cache = McpAppCache::new().internal_err_ctx("Failed to access app cache")?;
        let mut app = GooseApp::from_html(&req.html)
            .map_err(|error| agent_client_protocol::Error::invalid_params().data(error))?;

        let original_name = app.resource.name.clone();
        let mut counter = 1;
        let existing_names = cache
            .list_apps()
            .unwrap_or_default()
            .into_iter()
            .map(|app| app.resource.name)
            .collect::<std::collections::HashSet<_>>();

        while existing_names.contains(&app.resource.name) {
            app.resource.name = format!("{}_{}", original_name, counter);
            app.resource.uri = format!("ui://apps/{}", app.resource.name);
            counter += 1;
        }

        app.mcp_servers = vec![APPS_EXTENSION_NAME.to_string()];
        let name = app.resource.name.clone();
        cache
            .store_app(&app)
            .internal_err_ctx("Failed to store imported app")?;

        Ok(AppsImportResponse {
            name: name.clone(),
            message: format!("App '{}' imported successfully", name),
        })
    }

    pub(super) async fn on_delete_app(
        &self,
        req: AppsDeleteRequest,
    ) -> Result<AppsDeleteResponse, agent_client_protocol::Error> {
        let cache = McpAppCache::new().internal_err_ctx("Failed to access app cache")?;
        let apps = cache.list_apps().internal_err_ctx("Failed to list apps")?;

        let app = apps
            .into_iter()
            .find(|app| {
                app.resource.name == req.name
                    && app.mcp_servers.contains(&APPS_EXTENSION_NAME.to_string())
            })
            .ok_or_else(|| {
                agent_client_protocol::Error::resource_not_found(Some(req.name.clone()))
                    .data(format!("App '{}' not found", req.name))
            })?;

        let default_names = McpAppCache::default_app_names();
        if default_names.contains(&app.resource.name) {
            return Err(agent_client_protocol::Error::invalid_params()
                .data(format!("Cannot delete default app '{}'", app.resource.name)));
        }

        delete_app_html_file(&app.resource.name)?;

        for extension_name in &app.mcp_servers {
            cache
                .delete_app(extension_name, &app.resource.uri)
                .internal_err()?;
        }

        Ok(AppsDeleteResponse {
            name: app.resource.name.clone(),
            message: format!("App '{}' deleted successfully", app.resource.name),
        })
    }
}

fn delete_app_html_file(app_name: &str) -> Result<(), agent_client_protocol::Error> {
    if app_name.contains('/')
        || app_name.contains('\\')
        || app_name.contains('\0')
        || app_name == "."
        || app_name == ".."
    {
        return Ok(());
    }

    let html_path = Paths::in_data_dir(APPS_EXTENSION_NAME).join(format!("{app_name}.html"));
    if html_path.exists() {
        std::fs::remove_file(html_path).internal_err()?;
    }

    Ok(())
}

fn apps_to_values(
    apps: Vec<GooseApp>,
) -> Result<Vec<serde_json::Value>, agent_client_protocol::Error> {
    apps.into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .internal_err()
}
