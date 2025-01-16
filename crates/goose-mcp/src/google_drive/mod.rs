use indoc::indoc;
use regex::Regex;
use serde_json::{json, Value};

use std::{env, fs, future::Future, io::Write, path::Path, pin::Pin};

use mcp_core::{
    handler::{ResourceError, ToolError},
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;

use mcp_core::content::Content;

use google_drive3::{
    self,
    api::{ File, Scope},
    hyper_rustls::{self, HttpsConnector},
    hyper_util::{self, client::legacy::connect::HttpConnector},
    yup_oauth2::{
        self,
        authenticator_delegate::{DefaultInstalledFlowDelegate, InstalledFlowDelegate},
        InstalledFlowAuthenticator,
    },
    DriveHub,
};

use http_body_util::{BodyExt};

/// async function to be pinned by the `present_user_url` method of the trait
/// we use the existing `DefaultInstalledFlowDelegate::present_user_url` method as a fallback for
/// when the browser did not open for example, the user still see's the URL.
async fn browser_user_url(url: &str, need_code: bool) -> Result<String, String> {
    if webbrowser::open(url).is_ok() {
        println!("webbrowser was successfully opened.");
    }
    let def_delegate = DefaultInstalledFlowDelegate;
    def_delegate.present_user_url(url, need_code).await
}

/// our custom delegate struct we will implement a flow delegate trait for:
/// in this case we will implement the `InstalledFlowDelegated` trait
#[derive(Copy, Clone)]
struct LocalhostBrowserDelegate;

/// here we implement only the present_user_url method with the added webbrowser opening
/// the other behaviour of the trait does not need to be changed.
impl InstalledFlowDelegate for LocalhostBrowserDelegate {
    /// the actual presenting of URL and browser opening happens in the function defined above here
    /// we only pin it
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(browser_user_url(url, need_code))
    }
}

pub struct GoogleDriveRouter {
    tools: Vec<Tool>,
    instructions: String,
    drive: DriveHub<HttpsConnector<HttpConnector>>,
}

impl GoogleDriveRouter {
    async fn google_auth() -> DriveHub<HttpsConnector<HttpConnector>> {
        let oauth_config = env::var("GOOGLE_DRIVE_OAUTH_CONFIG");
        let keyfile_path_str = env::var("GOOGLE_DRIVE_OAUTH_PATH")
            .unwrap_or_else(|_| "./gcp-oauth.keys.json".to_string());
        let credentials_path_str = env::var("GOOGLE_DRIVE_CREDENTIALS_PATH")
            .unwrap_or_else(|_| "./gdrive-server-credentials.json".to_string());

        let keyfile_path = Path::new(&keyfile_path_str);
        let credentials_path = Path::new(&credentials_path_str);

        if !keyfile_path.exists() && oauth_config.is_ok() {
            // TODO: add tracing
            // attempt to create the path
            if let Some(parent_dir) = keyfile_path.parent() {
                let _ = fs::create_dir_all(parent_dir);
            }

            if let Ok(mut file) = fs::File::create(keyfile_path) {
                let _ = file.write_all(oauth_config.unwrap().as_bytes());
            }
        }

        let secret = yup_oauth2::read_application_secret(keyfile_path)
            .await
            .expect("expected keyfile for google auth");

        let auth = InstalledFlowAuthenticator::builder(
            secret,
            yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
        )
        .persist_tokens_to_disk(credentials_path)
        .flow_delegate(Box::new(LocalhostBrowserDelegate))
        .build()
        .await
        .expect("expected successful authentication");

        //let scopes = &["https://www.googleapis.com/auth/drive.readonly"];

        //// token(<scopes>) is the one important function of this crate; it does everything to
        //// obtain a token that can be sent e.g. as Bearer token.
        //// ignore success case, handle errors
        //if let Err(e) = auth.token(scopes).await {
        //    eprintln!("Unable to successfully authenticate: {}", e);
        //}

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build(
                    hyper_rustls::HttpsConnectorBuilder::new()
                        .with_native_roots()
                        .unwrap()
                        .https_or_http()
                        .enable_http1()
                        .build(),
                );

        DriveHub::new(client, auth)
    }

    pub async fn new() -> Self {
        let drive = Self::google_auth().await;

        // handle auth
        let search_tool = Tool::new(
            "search".to_string(),
            indoc! {r#"
                Search for files in google drive by name, given an input search query.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
              "query": {
                  "type": "string",
                  "description": "Search query",
                  },
              },
              "required": ["query"],
            }),
        );

        let read_tool = Tool::new(
            "read".to_string(),
            indoc! {r#"
                Read a file from google drive using the file uri.
                Optionally include base64 encoded images, false by default.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "uri": {
                      "type": "string",
                      "description": "google drive uri of the file to read",
                  },
                  "includeImages": {
                      "type": "boolean",
                      "description": "Whether or not to include images as base64 encoded strings, defaults to false",
                  }
              },
              "required": ["uri"],
            }),
        );

        Self {
            tools: vec![search_tool, read_tool],
            instructions: "".to_string(),
            drive,
        }
    }

    // Implement search tool functionality
    async fn search(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let query = params
            .get("query")
            .and_then(|q| q.as_str())
            .ok_or(ToolError::InvalidParameters(
                "The query string is required".to_string(),
            ))?
            .replace('\\', "\\\\")
            .replace('\'', "\\'");

        let result = self
            .drive
            .files()
            .list()
            .q(format!("name contains '{}'", query).as_str())
            .order_by("viewedByMeTime desc")
            .supports_all_drives(true)
            .include_items_from_all_drives(true)
            .page_size(10)
            .param("fields", "files(id, name, mimeType, modifiedTime, size)")
            .clear_scopes() // Scope::MeetReadonly is the default, remove it
            .add_scope(Scope::Readonly)
            .doit()
            .await;

        match result {
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to execute google drive search query, {}.",
                e
            ))),
            Ok(r) => {
                let content =
                    r.1.files
                        .map(|fs| {
                            fs.into_iter().map(|f| {
                                format!(
                                    "{} ({}) (uri: {})",
                                    f.name.unwrap_or_default(),
                                    f.mime_type.unwrap_or_default(),
                                    f.id.unwrap_or_default()
                                )
                            })
                        })
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join("\n");

                Ok(vec![Content::text(content.to_string())])
            }
        }
    }

    async fn fetch_file_metadata(&self, uri: &str) -> Result<File, ToolError> {
        self.drive
            .files()
            .get(&uri)
            .param("fields", "mimeType")
            .supports_all_drives(true)
            .clear_scopes()
            .add_scope(Scope::Readonly)
            .doit()
            .await
            .map_err(|e| {
                ToolError::ExecutionError(format!(
                    "Failed to execute Google Drive get query, {}.",
                    e
                ))
            })
            .map(|r| r.1)
    }

    fn strip_image_body(&self, input: &str) -> String {
        let image_regex = Regex::new(r"<data:image/[a-zA-Z0-9.-]+;base64,[^>]+>").unwrap();
        image_regex.replace_all(input, "").to_string()
    }

    // Downloading content with alt=media only works if the file is stored in Drive.
    // To download Google Docs, Sheets, and Slides use files.export instead.
    async fn export_google_file(
        &self,
        uri: &str,
        mime_type: &str,
        include_images: bool,
    ) -> Result<Vec<Content>, ToolError> {
        let export_mime_type = match mime_type {
            "application/vnd.google-apps.document" => "text/markdown",
            "application/vnd.google-apps.spreadsheet" => "text/csv",
            "application/vnd.google-apps.presentation" => "text/plain",
            _ => "text/plain",
        };

        let result = self
            .drive
            .files()
            .export(&uri, export_mime_type)
            .param("alt", "media")
            .clear_scopes()
            .add_scope(Scope::Readonly)
            .doit()
            .await;

        match result {
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to execute google drive export for {}, {}.",
                uri, e
            ))),
            Ok(r) => {
                if let Ok(body) = r.into_body().collect().await {
                    if let Ok(response) = String::from_utf8(body.to_bytes().to_vec()) {
                        let content = if !include_images {
                            self.strip_image_body(&response)
                        } else {
                            response
                        };

                        Ok(vec![Content::text(content)])
                    } else {
                        Err(ToolError::ExecutionError(format!(
                            "Failed to export google drive to string, {}.",
                            uri,
                        )))
                    }
                } else {
                    Err(ToolError::ExecutionError(format!(
                        "Failed to export google drive document, {}.",
                        uri,
                    )))
                }
            }
        }
    }

    // handle for files we can use files.get on
    async fn get_google_file(
        &self,
        uri: &str,
        include_images: bool,
    ) -> Result<Vec<Content>, ToolError> {
        let result = self
            .drive
            .files()
            .get(&uri)
            .param("alt", "media")
            .clear_scopes()
            .add_scope(Scope::Readonly)
            .doit()
            .await;

        match result {
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to execute google drive export for {}, {}.",
                uri, e
            ))),
            Ok(r) => {
                let file = r.1;
                let mime_type = file
                    .mime_type
                    .unwrap_or("application/octet-stream".to_string());
                if mime_type.starts_with("text/") || mime_type == "application/json" {
                    if let Ok(body) = r.0.into_body().collect().await {
                        if let Ok(response) = String::from_utf8(body.to_bytes().to_vec()) {
                            let content = if !include_images {
                                self.strip_image_body(&response)
                            } else {
                                response
                            };

                            Ok(vec![Content::text(content)])
                        } else {
                            Err(ToolError::ExecutionError(format!(
                                "Failed to convert google drive to string, {}.",
                                uri,
                            )))
                        }
                    } else {
                        Err(ToolError::ExecutionError(format!(
                            "Failed to get google drive document, {}.",
                            uri,
                        )))
                    }
                } else {
                    //TODO: handle base64 image case, see typscript mcp-gdrive
                    Err(ToolError::ExecutionError(format!(
                        "Suported mimeType {}, for {}",
                        mime_type, uri,
                    )))
                }
            }
        }
    }

    async fn read(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let uri =
            params
                .get("uri")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The uri of the file is required".to_string(),
                ))?;

        let drive_uri = uri.replace("gdrive:///", "");

        let include_images = params
            .get("includeImages")
            .and_then(|i| i.as_bool())
            .unwrap_or(false);

        let metadata = self.fetch_file_metadata(&drive_uri).await?;
        let mime_type = metadata.mime_type.ok_or_else(|| {
            ToolError::ExecutionError(format!("Missing mime type in file metadata for {}.", uri))
        })?;

        // Handle Google Docs export
        if mime_type.starts_with("application/vnd.google-apps") {
            self.export_google_file(&drive_uri, &mime_type, include_images)
                .await
        } else {
            self.get_google_file(&drive_uri, include_images).await
        }
    }
}

impl Router for GoogleDriveRouter {
    fn name(&self) -> String {
        "google_drive".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            //.with_resources(false, false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "search" => this.search(arguments).await,
                "read" => this.read(arguments).await,
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        //TODO: implement
        Vec::new()
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        //TODO: implement
        Box::pin(async move { Ok("".to_string()) })
    }
}

impl Clone for GoogleDriveRouter {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            instructions: self.instructions.clone(),
            drive: self.drive.clone(),
        }
    }
}
