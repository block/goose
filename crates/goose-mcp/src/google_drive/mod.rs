mod token_storage;

use indoc::indoc;
use regex::Regex;
use serde_json::{json, Value};
use token_storage::{CredentialsManager, KeychainTokenStorage};

use std::io::Cursor;
use std::sync::Arc;
use std::{env, fs, future::Future, path::Path, pin::Pin};

use mcp_core::content::Content;
use mcp_core::{
    handler::{PromptError, ResourceError, ToolError},
    prompt::Prompt,
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;

use google_drive3::common::ReadSeek;
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
use google_sheets4::{self, Sheets};
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

#[derive(Debug)]
enum FileOperation {
    Create { name: String },
    Update { file_id: String },
}

pub struct GoogleDriveRouter {
    tools: Vec<Tool>,
    instructions: String,
    drive: DriveHub<HttpsConnector<HttpConnector>>,
    sheets: Sheets<HttpsConnector<HttpConnector>>,
    credentials_manager: Arc<CredentialsManager>,
}

impl GoogleDriveRouter {
    async fn google_auth() -> (
        DriveHub<HttpsConnector<HttpConnector>>,
        Sheets<HttpsConnector<HttpConnector>>,
        Arc<CredentialsManager>,
    ) {
        let keyfile_path_str = env::var("GOOGLE_DRIVE_OAUTH_PATH")
            .unwrap_or_else(|_| "./gcp-oauth.keys.json".to_string());
        let credentials_path_str = env::var("GOOGLE_DRIVE_CREDENTIALS_PATH")
            .unwrap_or_else(|_| "./gdrive-server-credentials.json".to_string());

        let expanded_keyfile = shellexpand::tilde(keyfile_path_str.as_str());
        let keyfile_path = Path::new(expanded_keyfile.as_ref());

        let expanded_credentials = shellexpand::tilde(credentials_path_str.as_str());
        let credentials_path = expanded_credentials.to_string();

        tracing::info!(
            credentials_path = credentials_path_str,
            keyfile_path = keyfile_path_str,
            "Google Drive MCP server authentication config paths"
        );

        if let Ok(oauth_config) = env::var("GOOGLE_DRIVE_OAUTH_CONFIG") {
            // Ensure the parent directory exists (create_dir_all is idempotent)
            if let Some(parent) = keyfile_path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    tracing::error!(
                        "Failed to create parent directories for {}: {}",
                        keyfile_path.display(),
                        e
                    );
                }
            }

            // Check if the file exists and whether its content matches
            // in every other case we attempt to overwrite
            let need_to_write = match fs::read_to_string(keyfile_path) {
                Ok(existing) if existing == oauth_config => false,
                Ok(_) | Err(_) => true,
            };

            // Overwrite the file if needed
            if need_to_write {
                if let Err(e) = fs::write(keyfile_path, &oauth_config) {
                    tracing::error!(
                        "Failed to write OAuth config to {}: {}",
                        keyfile_path.display(),
                        e
                    );
                } else {
                    tracing::debug!(
                        "Wrote Google Drive MCP server OAuth config to {}",
                        keyfile_path.display()
                    );
                }
            }
        }

        // Create a credentials manager for storing tokens securely
        let credentials_manager = Arc::new(CredentialsManager::new(credentials_path.clone()));

        // Read the application secret from the OAuth keyfile
        let secret = yup_oauth2::read_application_secret(keyfile_path)
            .await
            .expect("expected keyfile for google auth");

        // Create custom token storage using our credentials manager
        let token_storage = KeychainTokenStorage::new(
            secret
                .project_id
                .clone()
                .unwrap_or("unknown-project-id".to_string())
                .to_string(),
            credentials_manager.clone(),
        );

        // Create the authenticator with the installed flow
        let auth = InstalledFlowAuthenticator::builder(
            secret,
            yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
        )
        .with_storage(Box::new(token_storage)) // Use our custom storage
        .flow_delegate(Box::new(LocalhostBrowserDelegate))
        .build()
        .await
        .expect("expected successful authentication");

        // Create the HTTP client
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

        let drive_hub = DriveHub::new(client.clone(), auth.clone());
        let sheets_hub = Sheets::new(client, auth);

        // Create and return the DriveHub
        (drive_hub, sheets_hub, credentials_manager)
    }

    pub async fn new() -> Self {
        let (drive, sheets, credentials_manager) = Self::google_auth().await;

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

        let upload_tool = Tool::new(
            "upload".to_string(),
            indoc! {r#"
                Upload a file to Google Drive.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "name": {
                      "type": "string",
                      "description": "The desired filename to use for the uploaded file.",
                  },
                  "mimeType": {
                      "type": "string",
                      "description": "The MIME type of the file.",
                  },
                  "body": {
                      "type": "string",
                      "description": "Plain text body of the file to upload. Mutually exclusive with path.",
                  },
                  "path": {
                      "type": "string",
                      "description": "Path to the file to upload. Mutually exclusive with body.",
                  },
                  "parent_id": {
                      "type": "string",
                      "description": "ID of the parent folder in which to create the file. (default: creates files in the root of 'My Drive')",
                  },
                  "allow_shared_drives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  }
              },
              "required": ["name", "mimeType"],
            }),
        );

        let create_doc_tool = Tool::new(
            "create_doc".to_string(),
            indoc! {r#"
                Create a Google Doc from markdown text in Google Drive.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "name": {
                      "type": "string",
                      "description": "Name of the file to create",
                  },
                  "body": {
                      "type": "string",
                      "description": "Markdown text of the file to create.",
                  },
                  "parent_id": {
                      "type": "string",
                      "description": "ID of the parent folder in which to create the file. (default: creates files in the root of 'My Drive')",
                  },
                  "allow_shared_drives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  }
              },
              "required": ["name", "body"],
            }),
        );

        let create_sheets_tool = Tool::new(
            "create_sheets".to_string(),
            indoc! {r#"
                Create a Google Sheets document from csv text in Google Drive.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "name": {
                      "type": "string",
                      "description": "Name of the file to create",
                  },
                  "body": {
                      "type": "string",
                      "description": "CSV text of the file to create.",
                  },
                  "parent_id": {
                      "type": "string",
                      "description": "ID of the parent folder in which to create the file. (default: creates files in the root of 'My Drive')",
                  },
                  "allow_shared_drives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  }
              },
              "required": ["name", "body"],
            }),
        );

        let create_slides_tool = Tool::new(
            "create_slides".to_string(),
            indoc! {r#"
                Create a Google Slides document in Google Drive by converting a PowerPoint file.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "name": {
                      "type": "string",
                      "description": "Name of the file to create",
                  },
                  "path": {
                      "type": "string",
                      "description": "Path to a PowerPoint file to upload.",
                  },
                  "parent_id": {
                      "type": "string",
                      "description": "ID of the parent folder in which to create the file. (default: creates files in the root of 'My Drive')",
                  },
                  "allow_shared_drives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  }
              },
              "required": ["name", "path"],
            }),
        );

        let update_tool = Tool::new(
            "update".to_string(),
            indoc! {r#"
                Update a Google Drive file with new content.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "fileId": {
                      "type": "string",
                      "description": "The ID of the file to update.",
                  },
                  "mimeType": {
                      "type": "string",
                      "description": "The MIME type of the file.",
                  },
                  "body": {
                      "type": "string",
                      "description": "Plain text body of the file to upload. Mutually exclusive with path.",
                  },
                  "path": {
                      "type": "string",
                      "description": "Path to a local file to use to update the Google Drive file. Mutually exclusive with body.",
                  },
                  "allow_shared_drives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  }
              },
              "required": ["fileId", "mimeType"],
            }),
        );

        let update_doc_tool = Tool::new(
            "update_doc".to_string(),
            indoc! {r#"
                Update a Google Doc from markdown text.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "fileId": {
                      "type": "string",
                      "description": "ID of the file to update",
                  },
                  "body": {
                      "type": "string",
                      "description": "Complete markdown text of the file to update.",
                  },
                  "allow_shared_drives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  }
              },
              "required": ["fileId", "body"],
            }),
        );

        let update_sheets_tool = Tool::new(
            "update_sheets".to_string(),
            indoc! {r#"
                Update a Google Sheets document from csv text.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "fileId": {
                      "type": "string",
                      "description": "ID of the file to update",
                  },
                  "body": {
                      "type": "string",
                      "description": "Complete CSV text of the updated file.",
                  },
                  "allow_shared_drives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  }
              },
              "required": ["fileId", "body"],
            }),
        );

        let update_slides_tool = Tool::new(
            "update_slides".to_string(),
            indoc! {r#"
                Updatea Google Slides document in Google Drive by converting a PowerPoint file.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "fileId": {
                      "type": "string",
                      "description": "ID of the file to update",
                  },
                  "path": {
                      "type": "string",
                      "description": "Path to a PowerPoint file to upload to replace the existing file.",
                  },
                  "allow_shared_drives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  }
              },
              "required": ["fileId", "path"],
            }),
        );

        let sheets_tool = Tool::new(
            "sheets_tool".to_string(),
            indoc! {r#"
                Work with Google Sheets data using various operations.
                Supports operations:
                - list_sheets: List all sheets in a spreadsheet
                - get_columns: Get column headers from a specific sheet
                - get_values: Get values from a range
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "spreadsheetId": {
                      "type": "string",
                      "description": "The ID of the spreadsheet to work with",
                  },
                  "operation": {
                      "type": "string",
                      "enum": ["list_sheets", "get_columns", "get_values"],
                      "description": "The operation to perform on the spreadsheet",
                  },
                  "sheetName": {
                      "type": "string",
                      "description": "The name of the sheet to work with (optional for some operations)",
                  },
                  "range": {
                      "type": "string",
                      "description": "The A1 notation of the range to retrieve values (e.g., 'Sheet1!A1:D10')",
                  }
              },
              "required": ["spreadsheetId", "operation"],
            }),
        );

        let comment_list_tool = Tool::new(
            "comment_list".to_string(),
            indoc! {r#"
                List comments for a file in google drive by id, given an input file id.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                "fileId": {
                    "type": "string",
                    "description": "Id of the file to list comments for.",
                },
                "pageSize": {
                    "type": "number",
                    "description": "How many items to return from the search query, default 10, max 100",
                }
              },
              "required": ["fileId"],
            }),
        );

        let instructions = indoc::formatdoc! {r#"
            Google Drive MCP Server Instructions

            ## Overview
            The Google Drive MCP server provides tools for interacting with Google Drive files and Google Sheets:
            1. search - Find files in your Google Drive
            2. read - Read file contents directly using a uri in the `gdrive:///uri` format
            3. sheets_tool - Work with Google Sheets data using various operations

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

            ### 3. Sheets Tool
            Work with Google Sheets data using various operations:
            - list_sheets: List all sheets in a spreadsheet
            - get_columns: Get column headers from a specific sheet
            - get_values: Get values from a range

            Parameters:
            - spreadsheetId: The ID of the spreadsheet (can be obtained from search results)
            - operation: The operation to perform (one of the operations listed above)
            - sheetName: The name of the sheet to work with (optional for some operations)
            - range: The A1 notation of the range to retrieve values (e.g., 'Sheet1!A1:D10')

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
            3. For Google Sheets, use the sheets_tool with the appropriate operation.

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
            tools: vec![
                search_tool,
                read_tool,
                upload_tool,
                create_doc_tool,
                create_sheets_tool,
                create_slides_tool,
                update_tool,
                update_doc_tool,
                update_sheets_tool,
                update_slides_tool,
                sheets_tool,
                comment_list_tool,
            ],
            instructions,
            drive,
            sheets,
            credentials_manager,
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

    // Implement sheets_tool functionality
    async fn sheets_tool(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let spreadsheet_id = params.get("spreadsheetId").and_then(|q| q.as_str()).ok_or(
            ToolError::InvalidParameters("The spreadsheetId is required".to_string()),
        )?;

        let operation = params.get("operation").and_then(|q| q.as_str()).ok_or(
            ToolError::InvalidParameters("The operation is required".to_string()),
        )?;

        match operation {
            "list_sheets" => {
                // Get spreadsheet metadata to list all sheets
                let result = self
                    .sheets
                    .spreadsheets()
                    .get(spreadsheet_id)
                    .clear_scopes()
                    .add_scope(Scope::Readonly)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Sheets get query, {}.",
                        e
                    ))),
                    Ok(r) => {
                        let spreadsheet = r.1;
                        let sheets = spreadsheet.sheets.unwrap_or_default();
                        let sheets_info = sheets
                            .into_iter()
                            .filter_map(|sheet| {
                                let properties = sheet.properties?;
                                let title = properties.title?;
                                let sheet_id = properties.sheet_id?;
                                let grid_properties = properties.grid_properties?;
                                Some(format!(
                                    "Sheet: {} (ID: {}, Rows: {}, Columns: {})",
                                    title,
                                    sheet_id,
                                    grid_properties.row_count.unwrap_or(0),
                                    grid_properties.column_count.unwrap_or(0)
                                ))
                            })
                            .collect::<Vec<String>>()
                            .join("\n");

                        Ok(vec![Content::text(sheets_info).with_priority(0.1)])
                    }
                }
            },
            "get_columns" => {
                // Get the sheet name if provided, otherwise we'll use the first sheet
                let sheet_name = params
                    .get("sheetName")
                    .and_then(|q| q.as_str())
                    .map(|s| format!("{}!1:1", s))
                    .unwrap_or_else(|| "1:1".to_string()); // Default to first row of first sheet

                let result = self
                    .sheets
                    .spreadsheets()
                    .values_get(spreadsheet_id, &sheet_name)
                    .clear_scopes()
                    .add_scope(Scope::Readonly)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Sheets get_columns query, {}.",
                        e
                    ))),
                    Ok(r) => {
                        let value_range = r.1;
                        // Extract just the headers (first row)
                        let headers = match value_range.values {
                            Some(mut values) if !values.is_empty() => {
                                // Take the first row only
                                let headers = values.remove(0);
                                let header_values: Vec<String> = headers
                                    .into_iter()
                                    .map(|cell| cell.as_str().unwrap_or_default().to_string())
                                    .collect();
                                header_values.join(", ")
                            }
                            _ => "No headers found".to_string(),
                        };

                        Ok(vec![Content::text(headers).with_priority(0.1)])
                    }
                }
            },
            "get_values" => {
                let range = params
                    .get("range")
                    .and_then(|q| q.as_str())
                    .ok_or(ToolError::InvalidParameters(
                        "The range is required for get_values operation".to_string(),
                    ))?;

                let result = self
                    .sheets
                    .spreadsheets()
                    .values_get(spreadsheet_id, range)
                    .clear_scopes()
                    .add_scope(Scope::Readonly)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Sheets values_get query, {}.",
                        e
                    ))),
                    Ok(r) => {
                        let value_range = r.1;
                        // Convert the values to a CSV string
                        let csv_content = match value_range.values {
                            Some(values) => {
                                let mut csv_string = String::new();
                                for row in values {
                                    let row_values: Vec<String> = row
                                        .into_iter()
                                        .map(|cell| cell.as_str().unwrap_or_default().to_string())
                                        .collect();
                                    csv_string.push_str(&row_values.join(","));
                                    csv_string.push('\n');
                                }
                                csv_string
                            }
                            None => "No data found".to_string(),
                        };

                        Ok(vec![Content::text(csv_content).with_priority(0.1)])
                    }
                }
            },
            _ => Err(ToolError::InvalidParameters(format!(
                "Invalid operation: {}. Supported operations are: list_sheets, get_columns, get_values",
                operation
            ))),
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

    async fn upload_to_drive(
        &self,
        operation: FileOperation,
        content: Box<dyn ReadSeek>,
        source_mime_type: &str,
        target_mime_type: &str,
        parent: Option<&str>,
        support_all_drives: bool,
    ) -> Result<Vec<Content>, ToolError> {
        let mut req = File {
            mime_type: Some(target_mime_type.to_string()),
            ..Default::default()
        };
        if let Some(p) = parent {
            req.parents = Some(vec![p.to_string()]);
        }

        let builder = self.drive.files();
        let result = match operation {
            FileOperation::Create { ref name } => {
                req.name = Some(name.to_string());
                builder
                    .create(req)
                    .use_content_as_indexable_text(true)
                    .supports_all_drives(support_all_drives)
                    .upload(content, source_mime_type.parse().unwrap())
                    .await
            }
            FileOperation::Update { ref file_id } => {
                builder
                    .update(req, file_id)
                    .use_content_as_indexable_text(true)
                    .supports_all_drives(support_all_drives)
                    .upload(content, source_mime_type.parse().unwrap())
                    .await
            }
        };
        match result {
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to upload google drive file {:?}, {}.",
                operation, e
            ))),
            Ok(r) => Ok(vec![Content::text(format!(
                "{} ({}) (uri: {})",
                r.1.name.unwrap_or_default(),
                r.1.mime_type.unwrap_or_default(),
                r.1.id.unwrap_or_default()
            ))]),
        }
    }

    async fn upload(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let filename =
            params
                .get("name")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The name param is required".to_string(),
                ))?;
        let mime_type =
            params
                .get("mimeType")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The mimeType param is required".to_string(),
                ))?;
        let body = params.get("body").and_then(|q| q.as_str());
        let path = params.get("path").and_then(|q| q.as_str());
        let reader: Box<dyn ReadSeek> = match (body, path) {
            (None, None) | (Some(_), Some(_)) => {
                return Err(ToolError::InvalidParameters(
                    "Either the body or path param is required".to_string(),
                ))
            }
            (Some(b), None) => Box::new(Cursor::new(b.as_bytes().to_owned())),
            (None, Some(p)) => Box::new(std::fs::File::open(p).map_err(|e| {
                ToolError::ExecutionError(format!("Error opening {}: {}", p, e).to_string())
            })?),
        };
        let parent = params.get("parent").and_then(|q| q.as_str());
        let support_all_drives = params
            .get("supportAllDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();
        self.upload_to_drive(
            FileOperation::Create {
                name: filename.to_string(),
            },
            reader,
            mime_type,
            mime_type,
            parent,
            support_all_drives,
        )
        .await
    }

    async fn create_doc(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let filename =
            params
                .get("name")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The name param is required".to_string(),
                ))?;
        let body =
            params
                .get("body")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The body param is required".to_string(),
                ))?;
        let source_mime_type = "text/markdown";
        let target_mime_type = "application/vnd.google-apps.document";
        let parent = params.get("parent").and_then(|q| q.as_str());
        let support_all_drives = params
            .get("supportAllDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();
        let cursor = Box::new(Cursor::new(body.as_bytes().to_owned()));
        self.upload_to_drive(
            FileOperation::Create {
                name: filename.to_string(),
            },
            cursor,
            source_mime_type,
            target_mime_type,
            parent,
            support_all_drives,
        )
        .await
    }

    async fn create_sheets(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let filename =
            params
                .get("name")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The name param is required".to_string(),
                ))?;
        let body =
            params
                .get("body")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The body param is required".to_string(),
                ))?;
        let source_mime_type = "text/csv";
        let target_mime_type = "application/vnd.google-apps.spreadsheet";
        let parent = params.get("parent").and_then(|q| q.as_str());
        let support_all_drives = params
            .get("supportAllDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();
        let cursor = Box::new(Cursor::new(body.as_bytes().to_owned()));
        self.upload_to_drive(
            FileOperation::Create {
                name: filename.to_string(),
            },
            cursor,
            source_mime_type,
            target_mime_type,
            parent,
            support_all_drives,
        )
        .await
    }

    async fn create_slides(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let filename =
            params
                .get("name")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The name param is required".to_string(),
                ))?;
        let path =
            params
                .get("path")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The path param is required".to_string(),
                ))?;
        let reader = Box::new(std::fs::File::open(path).map_err(|e| {
            ToolError::ExecutionError(format!("Error opening {}: {}", path, e).to_string())
        })?);
        let source_mime_type =
            "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let target_mime_type = "application/vnd.google-apps.presentation";
        let parent = params.get("parent").and_then(|q| q.as_str());
        let support_all_drives = params
            .get("supportAllDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();
        self.upload_to_drive(
            FileOperation::Create {
                name: filename.to_string(),
            },
            reader,
            source_mime_type,
            target_mime_type,
            parent,
            support_all_drives,
        )
        .await
    }

    async fn update(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let file_id =
            params
                .get("fileId")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileId param is required".to_string(),
                ))?;
        let mime_type =
            params
                .get("mimeType")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The mimeType param is required".to_string(),
                ))?;
        let body = params.get("body").and_then(|q| q.as_str());
        let path = params.get("path").and_then(|q| q.as_str());
        let reader: Box<dyn ReadSeek> = match (body, path) {
            (None, None) | (Some(_), Some(_)) => {
                return Err(ToolError::InvalidParameters(
                    "Either the body or path param is required".to_string(),
                ))
            }
            (Some(b), None) => Box::new(Cursor::new(b.as_bytes().to_owned())),
            (None, Some(p)) => Box::new(std::fs::File::open(p).map_err(|e| {
                ToolError::ExecutionError(format!("Error opening {}: {}", p, e).to_string())
            })?),
        };
        let support_all_drives = params
            .get("supportAllDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();

        self.upload_to_drive(
            FileOperation::Update {
                file_id: file_id.to_string(),
            },
            reader,
            mime_type,
            mime_type,
            None,
            support_all_drives,
        )
        .await
    }

    async fn update_doc(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let file_id =
            params
                .get("fileId")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileId param is required".to_string(),
                ))?;
        let body =
            params
                .get("body")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The body param is required".to_string(),
                ))?;
        let source_mime_type = "text/markdown";
        let target_mime_type = "application/vnd.google-apps.document";
        let support_all_drives = params
            .get("supportAllDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();
        let cursor = Box::new(Cursor::new(body.as_bytes().to_owned()));
        self.upload_to_drive(
            FileOperation::Update {
                file_id: file_id.to_string(),
            },
            cursor,
            source_mime_type,
            target_mime_type,
            None,
            support_all_drives,
        )
        .await
    }

    async fn update_sheets(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let file_id =
            params
                .get("fileId")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileId param is required".to_string(),
                ))?;
        let body =
            params
                .get("body")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The body param is required".to_string(),
                ))?;
        let source_mime_type = "text/csv";
        let target_mime_type = "application/vnd.google-apps.spreadsheet";
        let support_all_drives = params
            .get("supportAllDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();
        let cursor = Box::new(Cursor::new(body.as_bytes().to_owned()));
        self.upload_to_drive(
            FileOperation::Update {
                file_id: file_id.to_string(),
            },
            cursor,
            source_mime_type,
            target_mime_type,
            None,
            support_all_drives,
        )
        .await
    }

    async fn update_slides(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let file_id =
            params
                .get("fileId")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileId param is required".to_string(),
                ))?;
        let path =
            params
                .get("path")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The path param is required".to_string(),
                ))?;
        let reader = Box::new(std::fs::File::open(path).map_err(|e| {
            ToolError::ExecutionError(format!("Error opening {}: {}", path, e).to_string())
        })?);
        let source_mime_type =
            "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        let target_mime_type = "application/vnd.google-apps.presentation";
        let support_all_drives = params
            .get("supportAllDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();
        self.upload_to_drive(
            FileOperation::Update {
                file_id: file_id.to_string(),
            },
            reader,
            source_mime_type,
            target_mime_type,
            None,
            support_all_drives,
        )
        .await
    }

    async fn comment_list(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let file_id =
            params
                .get("fileId")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileId param is required".to_string(),
                ))?;

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
            .comments()
            .list(file_id)
            .page_size(page_size)
            .param(
                "fields",
                "comments(author, content, createdTime, modifiedTime, id, anchor, resolved)",
            )
            .clear_scopes()
            .add_scope(Scope::Readonly)
            .doit()
            .await;

        match result {
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to execute google drive comment list, {}.",
                e
            ))),
            Ok(r) => {
                let content =
                    r.1.comments
                        .map(|comments| {
                            comments.into_iter().map(|c| {
                                format!(
                                    "Author:{:?} Content: {} (created time: {}) (modified time: {})(anchor: {}) (resolved: {}) (id: {})",
                                    c.author.unwrap_or_default(),
                                    c.content.unwrap_or_default(),
                                    c.created_time.unwrap_or_default(),
                                    c.modified_time.unwrap_or_default(),
                                    c.anchor.unwrap_or_default(),
                                    c.resolved.unwrap_or_default(),
                                    c.id.unwrap_or_default()
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
                "upload" => this.upload(arguments).await,
                "create_doc" => this.create_doc(arguments).await,
                "create_sheets" => this.create_sheets(arguments).await,
                "create_slides" => this.create_slides(arguments).await,
                "update" => this.update(arguments).await,
                "update_doc" => this.update_doc(arguments).await,
                "update_sheets" => this.update_sheets(arguments).await,
                "update_slides" => this.update_slides(arguments).await,
                "sheets_tool" => this.sheets_tool(arguments).await,
                "comment_list" => this.comment_list(arguments).await,
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
            sheets: self.sheets.clone(),
            credentials_manager: self.credentials_manager.clone(),
        }
    }
}
