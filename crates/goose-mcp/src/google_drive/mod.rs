use indoc::indoc;
use regex::Regex;
use serde_json::{json, Value};

use std::{env, fs, future::Future, io::Write, path::Path, pin::Pin};

use mcp_core::{
    handler::{PromptError, ResourceError, ToolError},
    prompt::Prompt,
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;

use mcp_core::content::Content;

use google_drive3::{
    self,
    api::{File, Scope},
    hyper_rustls::{self, HttpsConnector},
    hyper_util::{self, client::legacy::connect::HttpConnector},
    yup_oauth2::{
        self,
        authenticator_delegate::{DefaultInstalledFlowDelegate, InstalledFlowDelegate},
        InstalledFlowAuthenticator,
    },
    DriveHub,
};

use http_body_util::BodyExt;

/// async function to be pinned by the `present_user_url` method of the trait
/// we use the existing `DefaultInstalledFlowDelegate::present_user_url` method as a fallback for
/// when the browser did not open for example, the user still see's the URL.
async fn browser_user_url(url: &str, need_code: bool) -> Result<String, String> {
    tracing::info!(oauth_url = url, "Attempting OAuth login flow");
    if let Err(e) = webbrowser::open(url) {
        tracing::debug!(oauth_url = url, error = ?e, "Failed to open OAuth flow");
        println!("Please open this URL in your browser:\n{}", url);
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

        let expanded_keyfile = shellexpand::tilde(keyfile_path_str.as_str());
        let keyfile_path = Path::new(expanded_keyfile.as_ref());

        let expanded_credentials = shellexpand::tilde(credentials_path_str.as_str());
        let credentials_path = Path::new(expanded_credentials.as_ref());

        tracing::info!(
            credentials_path = credentials_path_str,
            keyfile_path = keyfile_path_str,
            "Google Drive MCP server authentication config paths"
        );

        if !keyfile_path.exists() && oauth_config.is_ok() {
            tracing::debug!(
                oauth_config = ?oauth_config,
                "Google Drive MCP server OAuth config"
            );
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
                "corpora": {
                    "type": "string",
                    "description": "Which corpus to search, either 'user' (default), 'drive' or 'allDrives'",
                },
                "pageSize": {
                    "type": "number",
                    "description": "How many items to return from the search query, default 10, max 100",
                }
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

        let instructions = indoc::formatdoc! {r#"
            Google Drive MCP Server Instructions

            ## Overview
            The Google Drive MCP server provides two main tools for interacting with Google Drive files:
            1. search - Find files in your Google Drive
            2. read - Read file contents directly using a uri in the `gdrive:///uri` format

            ## Available Tools

            ### 1. Search Tool
            Search for files in Google Drive, by name and ordered by most recently viewedByMeTime.
            A corpora parameter controls which corpus is searched.
            Returns: List of files with their names, MIME types, and IDs

            ### 2. Read File Tool
            Read a file's contents using its ID, and optionally include images as base64 encoded data.
            The default is to exclude images, to include images set includeImages to true in the query.

            Images take up a large amount of context, this should only be used if a
            user explicity needs the image data.

            Limitations: Google Sheets exporting only supports reading the first sheet. This is an important limitation that should
            be communicated to the user whenever dealing with a Google Sheet (mimeType: application/vnd.google-apps.spreadsheet).

            ## File Format Handling
            The server automatically handles different file types:
            - Google Docs → Markdown
            - Google Sheets → CSV
            - Google Presentations → Plain text
            - Text/JSON files → UTF-8 text
            - Binary files → Base64 encoded

            ## Common Usage Pattern

            1. First, search for the file you want to read, searching by name.
            2. Then, use the file URI from the search results to read its contents.

            ## Best Practices
            1. Always use search first to find the correct file URI
            2. Search results include file types (MIME types) to help identify the right file
            3. Search is limited to 10 results per query, so use specific search terms
            4. The server has read-only access to Google Drive

            ## Error Handling
            If you encounter errors:
            1. Verify the file URI is correct
            2. Ensure you have access to the file
            3. Check if the file format is supported
            4. Verify the server is properly configured

            Remember: Always use the tools in sequence - search first to get the file URI, then read to access the contents.
        "#};

        Self {
            tools: vec![search_tool, read_tool],
            instructions,
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

        // extract corpora query parameter, validate options, or default to "user"
        let corpus = params
            .get("corpora")
            .and_then(|c| c.as_str())
            .map(|s| {
                if ["user", "drive", "allDrives"].contains(&s) {
                    Ok(s)
                } else {
                    Err(ToolError::InvalidParameters(format!(
                        "corpora must be either 'user', 'drive', or 'allDrives', got {}",
                        s
                    )))
                }
            })
            .unwrap_or(Ok("user"))?;

        // extract pageSize, and convert it to an i32, default to 10
        let page_size: i32 = params
            .get("pageSize")
            .map(|s| {
                s.as_i64()
                    .and_then(|n| i32::try_from(n).ok())
                    .ok_or_else(|| ToolError::InvalidParameters(format!("Invalid pageSize: {}", s)))
                    .and_then(|n| {
                        if (0..=100).contains(&n) {
                            Ok(n)
                        } else {
                            Err(ToolError::InvalidParameters(format!(
                                "pageSize must be between 0 and 100, got {}",
                                n
                            )))
                        }
                    })
            })
            .unwrap_or(Ok(10))?;

        let result = self
            .drive
            .files()
            .list()
            .corpora(corpus)
            .q(format!("name contains '{}'", query).as_str())
            .order_by("viewedByMeTime desc")
            .param("fields", "files(id, name, mimeType, modifiedTime, size)")
            .page_size(page_size)
            .supports_all_drives(true)
            .include_items_from_all_drives(true)
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
                        .map(|files| {
                            files.into_iter().map(|f| {
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
            .get(uri)
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
            .export(uri, export_mime_type)
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

                        Ok(vec![Content::text(content).with_priority(0.1)])
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
            .get(uri)
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

                            Ok(vec![Content::text(content).with_priority(0.1)])
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

    async fn read_google_resource(&self, uri: String) -> Result<String, ResourceError> {
        self.read(json!({"uri": uri}))
            .await
            .map_err(|e| ResourceError::ExecutionError(e.to_string()))
            .map(|contents| {
                contents
                    .into_iter()
                    .map(|content| content.as_text().unwrap_or_default().to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
    }

    async fn list_google_resources(&self, params: Value) -> Vec<Resource> {
        let next_page_token = params.get("cursor").and_then(|q| q.as_str());

        let mut query = self
            .drive
            .files()
            .list()
            .order_by("viewedByMeTime desc")
            .page_size(10)
            .param("fields", "nextPageToken, files(id, name, mimeType)")
            .supports_all_drives(true)
            .include_items_from_all_drives(true)
            .clear_scopes() // Scope::MeetReadonly is the default, remove it
            .add_scope(Scope::Readonly);

        // add a next token if we have one
        if let Some(token) = next_page_token {
            query = query.page_token(token)
        }

        let result = query.doit().await;

        match result {
            Err(_) => {
                //Err(ResourceError::ExecutionError(format!(
                //    "Failed to execute google drive list query, {}.",
                //    e,
                //)));
                vec![]
            }
            Ok(r) => {
                r.1.files
                    .map(|files| {
                        files.into_iter().map(|f| Resource {
                            uri: f.id.unwrap_or_default(),
                            mime_type: f.mime_type.unwrap_or_default(),
                            name: f.name.unwrap_or_default(),
                            description: None,
                            annotations: None,
                        })
                    })
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
            }
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
            .with_resources(false, false)
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
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { self.list_google_resources(json!({})).await })
        })
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let this = self.clone();
        let uri_clone = uri.to_string();
        Box::pin(async move { this.read_google_resource(uri_clone).await })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        vec![]
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!(
                "Prompt {} not found",
                prompt_name
            )))
        })
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
