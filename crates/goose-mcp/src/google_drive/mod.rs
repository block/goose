mod oauth_pkce;
pub mod storage;

use anyhow::{Context, Error};
use base64::Engine;
use indoc::indoc;
use mcp_core::tool::ToolAnnotations;
use oauth_pkce::PkceOAuth2Client;
use regex::Regex;
use serde_json::{json, Value};
use std::io::Cursor;
use std::{env, fs, future::Future, path::Path, pin::Pin, sync::Arc};
use storage::CredentialsManager;

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
    api::{Comment, File, FileShortcutDetails, Reply, Scope},
    hyper_rustls::{self, HttpsConnector},
    hyper_util::{self, client::legacy::connect::HttpConnector},
    DriveHub,
};
use google_sheets4::{self, Sheets};
use http_body_util::BodyExt;

// Constants for credential storage
pub const KEYCHAIN_SERVICE: &str = "mcp_google_drive";
pub const KEYCHAIN_USERNAME: &str = "oauth_credentials";
pub const KEYCHAIN_DISK_FALLBACK_ENV: &str = "GOOGLE_DRIVE_DISK_FALLBACK";

const GOOGLE_DRIVE_SCOPES: Scope = Scope::Full;

#[derive(Debug)]
enum FileOperation {
    Create { name: String },
    Update { file_id: String },
}
#[derive(PartialEq)]
enum PaginationState {
    Start,
    Next(String),
    End,
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

        // Check if we should fall back to disk, must be explicitly enabled
        let fallback_to_disk = match env::var(KEYCHAIN_DISK_FALLBACK_ENV) {
            Ok(value) => value.to_lowercase() == "true",
            Err(_) => false,
        };

        // Create a credentials manager for storing tokens securely
        let credentials_manager = Arc::new(CredentialsManager::new(
            credentials_path.clone(),
            fallback_to_disk,
            KEYCHAIN_SERVICE.to_string(),
            KEYCHAIN_USERNAME.to_string(),
        ));

        // Read the OAuth credentials from the keyfile
        match fs::read_to_string(keyfile_path) {
            Ok(_) => {
                // Create the PKCE OAuth2 clien
                let auth = PkceOAuth2Client::new(keyfile_path, credentials_manager.clone())
                    .expect("Failed to create OAuth2 client");

                // Create the HTTP client
                let client = hyper_util::client::legacy::Client::builder(
                    hyper_util::rt::TokioExecutor::new(),
                )
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

                // Create and return the DriveHub, Sheets and our PKCE OAuth2 client
                (drive_hub, sheets_hub, credentials_manager)
            }
            Err(e) => {
                tracing::error!(
                    "Failed to read OAuth config from {}: {}",
                    keyfile_path.display(),
                    e
                );
                panic!("Failed to read OAuth config: {}", e);
            }
        }
    }

    pub async fn new() -> Self {
        // handle auth
        let (drive, sheets, credentials_manager) = Self::google_auth().await;

        let search_tool = Tool::new(
            "search".to_string(),
            indoc! {r#"
                Search for files in google drive by name, given an input search query. At least one of ('name', 'mimeType', or 'parent') are required.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                "name": {
                    "type": "string",
                    "description": "String to search for in the file's name.",
                },
                "mimeType": {
                    "type": "string",
                    "description": "MIME type to constrain the search to.",
                },
                "parent": {
                    "type": "string",
                    "description": "ID of a folder to limit the search to",
                },
                "driveId": {
                    "type": "string",
                    "description": "ID of a shared drive to constrain the search to when using the corpus 'drive'.",
                },
                "corpora": {
                    "type": "string",
                    "description": "Which corpus to search, either 'user' (default), 'drive' (requires a driveID) or 'allDrives'",
                },
                "pageSize": {
                    "type": "number",
                    "description": "How many items to return from the search query, default 10, max 100",
                }
              },
            }),
            Some(ToolAnnotations {
                    title: Some("Search GDrive".to_string()),
                    read_only_hint: true,
                    destructive_hint: false,
                    idempotent_hint: false,
                    open_world_hint: false,
                }),
        );

        let read_tool = Tool::new(
            "read".to_string(),
            indoc! {r#"
                Read a file from google drive using the file uri.
                Optionally include base64 encoded images, false by default.

                Example extracting URIs from URLs:
                Given "https://docs.google.com/document/d/1QG8d8wtWe7ZfmG93sW-1h2WXDJDUkOi-9hDnvJLmWrc/edit?tab=t.0#heading=h.5v419d3h97tr"
                Pass in "gdrive:///1QG8d8wtWe7ZfmG93sW-1h2WXDJDUkOi-9hDnvJLmWrc"
                Do not include any other path parameters.
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
            Some(ToolAnnotations {
                title: Some("Read GDrive".to_string()),
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
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
                  "parentId": {
                      "type": "string",
                      "description": "ID of the parent folder in which to create the file. (default: creates files in the root of 'My Drive')",
                  },
                  "allowSharedDrives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  }
              },
              "required": ["name", "mimeType"],
            }),
            Some(ToolAnnotations {
                title: Some("Upload file to GDrive".to_string()),
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );

        let create_file_tool = Tool::new(
            "create_file".to_string(),
            indoc! {r#"
                Create a Google file (Document, Spreadsheet, Slides, folder, or shortcut) in Google Drive.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "name": {
                      "type": "string",
                      "description": "Name of the file to create",
                  },
                  "fileType": {
                      "type": "string",
                      "enum": ["document", "spreadsheet", "slides", "folder", "shortcut"],
                      "description": "Type of Google file to create (document, spreadsheet, slides, folder, or shortcut)",
                  },
                  "body": {
                      "type": "string",
                      "description": "Text content for the file (required for document and spreadsheet types)",
                  },
                  "path": {
                      "type": "string",
                      "description": "Path to a file to upload (required for slides type)",
                  },
                  "parentId": {
                      "type": "string",
                      "description": "ID of the parent folder in which to create the file (default: creates files in the root of 'My Drive')",
                  },
                  "targetId": {
                      "type": "string",
                      "description": "ID of the file to target when creating a shortcut",
                  },
                  "allowSharedDrives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  }
              },
              "required": ["name", "fileType"],
            }),
            Some(ToolAnnotations {
                title: Some("Create new file in GDrive".to_string()),
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );

        let move_file_tool = Tool::new(
            "move_file".to_string(),
            indoc! {r#"
                Move a Google Drive file, folder, or shortcut to a new parent folder. You cannot move a folder to a different drive.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "fileId": {
                      "type": "string",
                      "description": "The ID of the file to update.",
                  },
                  "currentFolderId": {
                      "type": "string",
                      "description": "The ID of the current parent folder.",
                  },
                  "newFolderId": {
                      "type": "string",
                      "description": "The ID of the folder to move the file to.",
                  },
              },
              "required": ["fileId", "currentFolderId", "newFolderId"],
            }),
            Some(ToolAnnotations {
                title: Some("Move file".to_string()),
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );

        let update_file_tool = Tool::new(
            "update_file".to_string(),
            indoc! {r#"
                Update a normal non-Google file (not Document, Spreadsheet, and Slides) in Google Drive with new content.
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
                  "allowSharedDrives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  }
              },
              "required": ["fileId", "mimeType"],
            }),
            Some(ToolAnnotations {
                title: Some("Update a non-Google file".to_string()),
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );

        let update_google_file_tool = Tool::new(
            "update_google_file".to_string(),
            indoc! {r#"
                Update a Google file (Document, Spreadsheet, or Slides) in Google Drive.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                  "fileId": {
                      "type": "string",
                      "description": "ID of the file to update",
                  },
                  "fileType": {
                      "type": "string",
                      "enum": ["document", "spreadsheet", "slides"],
                      "description": "Type of Google file to update (document, spreadsheet, or slides)",
                  },
                  "body": {
                      "type": "string",
                      "description": "Text content for the file (required for document and spreadsheet types)",
                  },
                  "path": {
                      "type": "string",
                      "description": "Path to a file to upload (required for slides type)",
                  },
                  "allowSharedDrives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  }
              },
              "required": ["fileId", "fileType"],
            }),
            Some(ToolAnnotations {
                title: Some("Update a Google file".to_string()),
                read_only_hint: false,
                destructive_hint: true,
                idempotent_hint: false,
                open_world_hint: false,
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
                - update_values: Update values in a range
                - update_cell: Update a single cell value
                - add_sheet: Add a new sheet (tab) to a spreadsheet
                - clear_values: Clear values from a range
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
                      "enum": ["list_sheets", "get_columns", "get_values", "update_values", "update_cell", "add_sheet", "clear_values"],
                      "description": "The operation to perform on the spreadsheet",
                  },
                  "sheetName": {
                      "type": "string",
                      "description": "The name of the sheet to work with (optional for some operations)",
                  },
                  "range": {
                      "type": "string",
                      "description": "The A1 notation of the range to retrieve or update values (e.g., 'Sheet1!A1:D10')",
                  },
                  "values": {
                      "type": "string",
                      "description": "CSV formatted data for update operations (required for update_values)",
                  },
                  "cell": {
                      "type": "string",
                      "description": "The A1 notation of the cell to update (e.g., 'Sheet1!A1') for update_cell operation",
                  },
                  "value": {
                      "type": "string",
                      "description": "The value to set in the cell for update_cell operation",
                  },
                  "title": {
                      "type": "string",
                      "description": "Title for the new sheet (required for add_sheet)",
                  },
                  "valueInputOption": {
                      "type": "string",
                      "enum": ["RAW", "USER_ENTERED"],
                      "description": "How input data should be interpreted (default: USER_ENTERED)",
                  }
              },
              "required": ["spreadsheetId", "operation"],
            }),
            None,
        );

        let get_comments_tool = Tool::new(
            "get_comments".to_string(),
            indoc! {r#"
                List comments for a file in google drive.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                "fileId": {
                    "type": "string",
                    "description": "Id of the file to list comments for.",
                }
              },
              "required": ["fileId"],
            }),
            Some(ToolAnnotations {
                title: Some("List file comments".to_string()),
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );

        let create_comment_tool = Tool::new(
            "create_comment".to_string(),
            indoc! {r#"
                Create a comment for the latest revision of a Google Drive file. The Google Drive API only supports unanchored comments (they don't refer to a specific location in the file).
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                "fileId": {
                    "type": "string",
                    "description": "Id of the file to comment on.",
                },
                "comment": {
                    "type": "string",
                    "description": "Content of the comment.",
                }
              },
              "required": ["fileId", "comment"],
            }),
            Some(ToolAnnotations {
                title: Some("Create file comment".to_string()),
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );

        let reply_tool = Tool::new(
            "reply".to_string(),
            indoc! {r#"
                Add a reply to a comment thread, or resolve a comment.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                "fileId": {
                    "type": "string",
                    "description": "Id of the file.",
                },
                "commentId": {
                    "type": "string",
                    "description": "Id of the comment to which you'd like to reply.",
                },
                "content": {
                    "type": "string",
                    "description": "Content of the reply.",
                },
                "resolveComment": {
                    "type": "boolean",
                    "description": "Whether to resolve the comment. Defaults to false.",
                }
              },
              "required": ["fileId", "commentId", "content"],
            }),
            Some(ToolAnnotations {
                title: Some("Reply to a comment".to_string()),
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );

        let list_drives_tool = Tool::new(
            "list_drives".to_string(),
            indoc! {r#"
                List shared Google drives.
            "#}
            .to_string(),
            json!({
              "type": "object",
              "properties": {
                "name_contains": {
                    "type": "string",
                    "description": "Optional name to search for when listing drives.",
                }
              },
            }),
            Some(ToolAnnotations {
                title: Some("List shared google drives".to_string()),
                read_only_hint: true,
                destructive_hint: false,
                idempotent_hint: false,
                open_world_hint: false,
            }),
        );

        let instructions = indoc::formatdoc! {r#"
            Google Drive MCP Server Instructions

            ## Overview
            The Google Drive MCP server provides tools for interacting with Google Drive files and Google Sheets:
            1. search - Find files in your Google Drive
            2. read - Read file contents directly using a uri in the `gdrive:///uri` format
            3. sheets_tool - Work with Google Sheets data using various operations
            4. create_file - Create Google Workspace files (Docs, Sheets, or Slides)
            5. update_google_file - Update existing Google Workspace files (Docs, Sheets, or Slides)
            6. update_file - Update existing normal non-Google Workspace files

            ## Available Tools

            ### 1. Search Tool
            Search for files in Google Drive, by name and ordered by most recently viewedByMeTime.
            A corpora parameter controls which corpus is searched.
            Returns: List of files with their names, MIME types, and IDs

            ### 2. Read File Tool
            Read a file's contents using its ID, and optionally include images as base64 encoded data.
            The default is to exclude images, to include images set includeImages to true in the query.

            Example mappings for Google Drive resources to `gdrive:///$URI` format:
            - Google Document File:
              Example URL: https://docs.google.com/document/d/1QG8d8wtWe7ZfmG93sW-1h2WXDJDUkOi-9hDnvJLmWrc/edit?tab=t.0#heading=h.5v419d3h97tr
              URI Format: gdrive:///1QG8d8wtWe7ZfmG93sW-1h2WXDJDUkOi-9hDnvJLmWrc

            - Google Sheet:
              Example URL: https://docs.google.com/spreadsheets/d/1J5KHqWsGFzweuiQboX7dlm8Ejv90Po16ocEBahzCt4W/edit?gid=1249300797#gid=1249300797
              URI Format: gdrive:///1J5KHqWsGFzweuiQboX7dlm8Ejv90Po16ocEBahzCt4W

            - Google Slides:
              Example URL: https://docs.google.com/presentation/d/1zXWqsGpHJEu40oqb1omh68sW9liu7EKFBCdnPaJVoQ5et/edit#slide=id.p1
              URI Format: gdrive:///1zXWqsGpHJEu40oqb1omh68sW9liu7EKFBCdnPaJVoQ5et

            Images take up a large amount of context, this should only be used if a
            user explicity needs the image data.

            Limitations: Google Sheets exporting only supports reading the first sheet. This is an important limitation that should
            be communicated to the user whenever dealing with a Google Sheet (mimeType: application/vnd.google-apps.spreadsheet).

            ### 3. Sheets Tool
            Work with Google Sheets data using various operations:
            - list_sheets: List all sheets in a spreadsheet
            - get_columns: Get column headers from a specific sheet
            - get_values: Get values from a range
            - update_values: Update values in a range (requires CSV formatted data)
            - update_cell: Update a single cell value
            - add_sheet: Add a new sheet (tab) to a spreadshee
            - clear_values: Clear values from a range

            For update_values operation, provide CSV formatted data in the values parameter.
            Each line represents a row, with values separated by commas.
            Example: "John,Doe,30\nJane,Smith,25"

            For update_cell operation, provide the cell reference (e.g., 'Sheet1!A1') and the value to set.

            ### 4. Create File Tool
            Create Google Workspace files (Docs, Sheets, or Slides) directly in Google Drive.
            - For Google Docs: Converts Markdown text to a Google Document
            - For Google Sheets: Converts CSV text to a Google Spreadsheet
            - For Google Slides: Converts a PowerPoint file to Google Slides (requires a path to the powerpoint file)

            ### 5. Update File Tool
            Update existing Google Workspace files (Docs, Sheets, or Slides) in Google Drive.
            - For Google Docs: Updates with new Markdown text
            - For Google Sheets: Updates with new CSV text
            - For Google Slides: Updates with a new PowerPoint file (requires a path to the powerpoint file)
                - Note: This functionally is an overwrite to the slides, warn the user before using this tool.

            Parameters:
            - spreadsheetId: The ID of the spreadsheet (can be obtained from search results)
            - operation: The operation to perform (one of the operations listed above)
            - sheetName: The name of the sheet to work with (optional for some operations)
            - range: The A1 notation of the range to retrieve or update values (e.g., 'Sheet1!A1:D10')
            - values: CSV formatted data for update operations
            - cell: The A1 notation of the cell to update (e.g., 'Sheet1!A1') for update_cell operation
            - value: The value to set in the cell for update_cell operation
            - title: Title for the new sheet (required for add_sheet operation)
            - valueInputOption: How input data should be interpreted (RAW or USER_ENTERED)

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
            4. When updating sheet values, format the data as CSV with one row per line

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
                create_file_tool,
                move_file_tool,
                update_file_tool,
                update_google_file_tool,
                sheets_tool,
                get_comments_tool,
                create_comment_tool,
                reply_tool,
                list_drives_tool,
            ],
            instructions,
            drive,
            sheets,
            credentials_manager,
        }
    }

    // Implement search tool functionality
    async fn search(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let name = params.get("name").and_then(|q| q.as_str());
        let mime_type = params.get("mimeType").and_then(|q| q.as_str());
        let drive_id = params.get("driveId").and_then(|q| q.as_str());
        let parent = params.get("parent").and_then(|q| q.as_str());

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

        let mut query = Vec::new();
        if let Some(n) = name {
            query.push(
                format!(
                    "name contains '{}'",
                    n.replace('\\', "\\\\").replace('\'', "\\'")
                )
                .to_string(),
            );
        }
        if let Some(m) = mime_type {
            query.push(format!("mimeType = '{}'", m).to_string());
        }
        if let Some(p) = parent {
            query.push(format!("'{}' in parents", p).to_string());
        }
        let query_string = query.join(" and ");
        if query_string.is_empty() {
            return Err(ToolError::InvalidParameters(
                "No query provided. Please include one of ('name', 'mimeType', 'parent')."
                    .to_string(),
            ));
        }
        let mut builder = self
            .drive
            .files()
            .list()
            .corpora(corpus)
            .q(query_string.as_str())
            .order_by("viewedByMeTime desc")
            .param("fields", "files(id, name, mimeType, modifiedTime, size)")
            .page_size(page_size)
            .supports_all_drives(true)
            .include_items_from_all_drives(true)
            .clear_scopes() // Scope::MeetReadonly is the default, remove it
            .add_scope(GOOGLE_DRIVE_SCOPES);
        // You can only use the drive_id param when the corpus is "drive".
        if let (Some(d), "drive") = (drive_id, corpus) {
            builder = builder.drive_id(d);
        }
        let result = builder.doit().await;

        match result {
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to execute google drive search query '{}', {}.",
                query_string.as_str(),
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

                Ok(vec![Content::text(content.to_string()).with_priority(0.3)])
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
            .add_scope(GOOGLE_DRIVE_SCOPES)
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

    // Helper function that processes one captured image.
    // It decodes the base64 data, resizes the image if its width exceeds `max_width`,
    // and then returns a new image tag (always output as PNG).
    // logic copied from developer/mod.rs
    fn process_image(&self, caps: &regex::Captures, max_width: u32) -> Result<Content, Error> {
        let base64_data = &caps["data"];

        // Decode the Base64 data.
        let image_bytes = base64::prelude::BASE64_STANDARD
            .decode(base64_data)
            .context("Failed to decode base64 image data")?;

        // Load the image from the decoded bytes.
        let img = xcap::image::load_from_memory(&image_bytes)
            .context("Failed to load image from memory")?;

        // Resize the image if necessary.
        let mut processed_image = img;
        if processed_image.width() > max_width {
            let scale = max_width as f32 / processed_image.width() as f32;
            let new_height = (processed_image.height() as f32 * scale) as u32;
            processed_image = xcap::image::DynamicImage::ImageRgba8(xcap::image::imageops::resize(
                &processed_image,
                max_width,
                new_height,
                xcap::image::imageops::FilterType::Lanczos3,
            ));
        }

        // Write the processed image to an in-memory buffer in PNG format.
        let mut buffer: Vec<u8> = Vec::new();
        processed_image
            .write_to(&mut Cursor::new(&mut buffer), xcap::image::ImageFormat::Png)
            .context("Failed to write processed image to buffer")?;

        // Re-encode the buffer back into a Base64 string.
        let data = base64::prelude::BASE64_STANDARD.encode(&buffer);
        Ok(Content::image(data, "image/png"))
    }

    /// Resizes all base64-encoded images found in the input string.
    /// If any image fails to process, an error is returned.
    fn resize_images(&self, input: &str) -> Result<Vec<Content>, Error> {
        // Regex to match and capture the MIME type and Base64 data.
        let image_regex =
            Regex::new(r"<data:image/(?P<mime>[a-zA-Z0-9.+-]+);base64,(?P<data>[^>]+)>")
                .context("Failed to compile regex")?;

        let mut result: Vec<Content> = Vec::new();

        // Iterate over all matches, process them, and rebuild the output string.
        for caps in image_regex.captures_iter(input) {
            let processed_tag = self
                .process_image(&caps, 768)
                .context("Failed to process one of the images")?;
            result.push(processed_tag);
        }

        Ok(result)
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
            .add_scope(GOOGLE_DRIVE_SCOPES)
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
                        if !include_images {
                            let content = self.strip_image_body(&response);
                            Ok(vec![Content::text(content).with_priority(0.1)])
                        } else {
                            let images = self.resize_images(&response).map_err(|e| {
                                ToolError::ExecutionError(format!(
                                    "Failed to resize image(s): {}",
                                    e
                                ))
                            })?;

                            let content = self.strip_image_body(&response);
                            Ok(std::iter::once(Content::text(content).with_priority(0.1))
                                .chain(images.iter().cloned())
                                .collect::<Vec<Content>>())
                        }
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
            .add_scope(GOOGLE_DRIVE_SCOPES)
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
                            if !include_images {
                                let content = self.strip_image_body(&response);
                                Ok(vec![Content::text(content).with_priority(0.1)])
                            } else {
                                let images = self.resize_images(&response).map_err(|e| {
                                    ToolError::ExecutionError(format!(
                                        "Failed to resize image(s): {}",
                                        e
                                    ))
                                })?;

                                let content = self.strip_image_body(&response);
                                Ok(std::iter::once(Content::text(content).with_priority(0.1))
                                    .chain(images.iter().cloned())
                                    .collect::<Vec<Content>>())
                            }
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

        // Validation: check for / path separators as invalid uris
        if drive_uri.contains('/') {
            return Err(ToolError::InvalidParameters(format!(
                "The uri '{}' conatins extra '/'. Only the base URI is allowed.",
                uri
            )));
        }

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
                    .add_scope(GOOGLE_DRIVE_SCOPES)
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
                    .add_scope(GOOGLE_DRIVE_SCOPES)
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
                    .add_scope(GOOGLE_DRIVE_SCOPES)
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
            "update_values" => {
                let range = params
                    .get("range")
                    .and_then(|q| q.as_str())
                    .ok_or(ToolError::InvalidParameters(
                        "The range is required for update_values operation".to_string(),
                    ))?;

                let values_csv = params
                    .get("values")
                    .and_then(|q| q.as_str())
                    .ok_or(ToolError::InvalidParameters(
                        "The values parameter is required for update_values operation".to_string(),
                    ))?;

                // Parse the CSV data into a 2D array of values
                let mut values: Vec<Vec<serde_json::Value>> = Vec::new();
                for line in values_csv.lines() {
                    let row: Vec<serde_json::Value> = line
                        .split(',')
                        .map(|cell| serde_json::Value::String(cell.trim().to_string()))
                        .collect();
                    if !row.is_empty() {
                        values.push(row);
                    }
                }

                // Determine the input option (default to USER_ENTERED)
                let value_input_option = params
                    .get("valueInputOption")
                    .and_then(|q| q.as_str())
                    .unwrap_or("USER_ENTERED");

                // Create the ValueRange objec
                let value_range = google_sheets4::api::ValueRange {
                    range: Some(range.to_string()),
                    values: Some(values),
                    major_dimension: None,
                };

                // Update the values
                let result = self
                    .sheets
                    .spreadsheets()
                    .values_update(value_range, spreadsheet_id, range)
                    .value_input_option(value_input_option)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Sheets values_update query, {}.",
                        e
                    ))),
                    Ok(r) => {
                        let update_response = r.1;
                        let updated_cells = update_response.updated_cells.unwrap_or(0);
                        let updated_rows = update_response.updated_rows.unwrap_or(0);
                        let updated_columns = update_response.updated_columns.unwrap_or(0);
                        let updated_range = update_response.updated_range.unwrap_or_default();

                        let response = format!(
                            "Successfully updated values in range '{}'. Updated {} cells across {} rows and {} columns.",
                            updated_range, updated_cells, updated_rows, updated_columns
                        );

                        Ok(vec![Content::text(response).with_priority(0.1)])
                    }
                }
            },
            "update_cell" => {
                let cell = params
                    .get("cell")
                    .and_then(|q| q.as_str())
                    .ok_or(ToolError::InvalidParameters(
                        "The cell parameter is required for update_cell operation".to_string(),
                    ))?;

                let value = params
                    .get("value")
                    .and_then(|q| q.as_str())
                    .ok_or(ToolError::InvalidParameters(
                        "The value parameter is required for update_cell operation".to_string(),
                    ))?;

                // Determine the input option (default to USER_ENTERED)
                let value_input_option = params
                    .get("valueInputOption")
                    .and_then(|q| q.as_str())
                    .unwrap_or("USER_ENTERED");

                // Create a single-cell ValueRange objec
                let value_range = google_sheets4::api::ValueRange {
                    range: Some(cell.to_string()),
                    values: Some(vec![vec![serde_json::Value::String(value.to_string())]]),
                    major_dimension: None,
                };

                // Update the cell value
                let result = self
                    .sheets
                    .spreadsheets()
                    .values_update(value_range, spreadsheet_id, cell)
                    .value_input_option(value_input_option)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Sheets update_cell operation, {}.",
                        e
                    ))),
                    Ok(r) => {
                        let update_response = r.1;
                        let updated_range = update_response.updated_range.unwrap_or_default();

                        Ok(vec![Content::text(format!(
                            "Successfully updated cell '{}' with value '{}'.",
                            updated_range, value
                        )).with_priority(0.1)])
                    }
                }
            },
            "add_sheet" => {
                let title = params
                    .get("title")
                    .and_then(|q| q.as_str())
                    .ok_or(ToolError::InvalidParameters(
                        "The title parameter is required for add_sheet operation".to_string(),
                    ))?;

                // Create the AddSheetReques
                let add_sheet_request = google_sheets4::api::AddSheetRequest {
                    properties: Some(google_sheets4::api::SheetProperties {
                        title: Some(title.to_string()),
                        sheet_id: None, // Google will auto-assign a sheet ID
                        index: None,
                        sheet_type: None,
                        grid_properties: None,
                        hidden: None,
                        tab_color: None,
                        right_to_left: None,
                        data_source_sheet_properties: None,
                        tab_color_style: None,
                    }),
                };

                // Create the BatchUpdateSpreadsheetReques
                let batch_update_request = google_sheets4::api::BatchUpdateSpreadsheetRequest {
                    requests: Some(vec![google_sheets4::api::Request {
                        add_sheet: Some(add_sheet_request),
                        ..google_sheets4::api::Request::default()
                    }]),
                    include_spreadsheet_in_response: Some(true),
                    response_ranges: None,
                    response_include_grid_data: None,
                };

                // Execute the batch update
                let result = self
                    .sheets
                    .spreadsheets()
                    .batch_update(batch_update_request, spreadsheet_id)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Sheets add_sheet operation, {}.",
                        e
                    ))),
                    Ok(r) => {
                        let response = r.1;
                        let replies = response.replies.unwrap_or_default();

                        if let Some(first_reply) = replies.first() {
                            if let Some(add_sheet_response) = &first_reply.add_sheet {
                                if let Some(properties) = &add_sheet_response.properties {
                                    let sheet_id = properties.sheet_id.unwrap_or(0);
                                    let title = properties.title.as_deref().unwrap_or("Unknown");

                                    let response = format!(
                                        "Successfully added new sheet '{}' with ID {}.",
                                        title, sheet_id
                                    );

                                    return Ok(vec![Content::text(response).with_priority(0.1)]);
                                }
                            }
                        }

                        // Generic success message if we couldn't extract specific details
                        Ok(vec![Content::text(format!(
                            "Successfully added new sheet '{}'.",
                            title
                        )).with_priority(0.1)])
                    }
                }
            },
            "clear_values" => {
                let range = params
                    .get("range")
                    .and_then(|q| q.as_str())
                    .ok_or(ToolError::InvalidParameters(
                        "The range is required for clear_values operation".to_string(),
                    ))?;

                // Create the ClearValuesReques
                let clear_values_request = google_sheets4::api::ClearValuesRequest::default();

                // Execute the clear values reques
                let result = self
                    .sheets
                    .spreadsheets()
                    .values_clear(clear_values_request, spreadsheet_id, range)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Sheets clear_values operation, {}.",
                        e
                    ))),
                    Ok(r) => {
                        let response = r.1;
                        let cleared_range = response.cleared_range.unwrap_or_default();

                        Ok(vec![Content::text(format!(
                            "Successfully cleared values in range '{}'.",
                            cleared_range
                        )).with_priority(0.1)])
                    }
                }
            },
            _ => Err(ToolError::InvalidParameters(format!(
                "Invalid operation: {}. Supported operations are: list_sheets, get_columns, get_values, update_values, update_cell, add_sheet, clear_values",
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
            .add_scope(GOOGLE_DRIVE_SCOPES);

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

    #[allow(clippy::too_many_arguments)]
    async fn upload_to_drive(
        &self,
        operation: FileOperation,
        content: Box<dyn ReadSeek>,
        source_mime_type: &str,
        target_mime_type: &str,
        parent: Option<&str>,
        support_all_drives: bool,
        target_id: Option<&str>,
    ) -> Result<Vec<Content>, ToolError> {
        let mut req = File {
            mime_type: Some(target_mime_type.to_string()),
            ..Default::default()
        };

        let builder = self.drive.files();

        let result = match operation {
            FileOperation::Create { ref name } => {
                req.name = Some(name.to_string());

                // we only accept parent_id from create tool calls
                if let Some(p) = parent {
                    req.parents = Some(vec![p.to_string()]);
                }

                if let Some(t) = target_id {
                    req.shortcut_details = Some(FileShortcutDetails {
                        target_id: Some(t.to_string()),
                        ..Default::default()
                    });
                }

                builder
                    .create(req)
                    .use_content_as_indexable_text(true)
                    .supports_all_drives(support_all_drives)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .upload(content, source_mime_type.parse().unwrap())
                    .await
            }
            FileOperation::Update { ref file_id } => {
                builder
                    .update(req, file_id)
                    .use_content_as_indexable_text(true)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
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

        let parent_id = params.get("parentId").and_then(|q| q.as_str());

        let allow_shared_drives = params
            .get("allowSharedDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();

        self.upload_to_drive(
            FileOperation::Create {
                name: filename.to_string(),
            },
            reader,
            mime_type,
            mime_type,
            parent_id,
            allow_shared_drives,
            None,
        )
        .await
    }

    async fn create_file(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        // Extract common parameters
        let filename =
            params
                .get("name")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The name param is required".to_string(),
                ))?;

        let file_type =
            params
                .get("fileType")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileType param is required".to_string(),
                ))?;

        let parent_id = params.get("parentId").and_then(|q| q.as_str());

        let target_id = params.get("targetId").and_then(|q| q.as_str());

        let allow_shared_drives = params
            .get("allowSharedDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();

        // Determine source and target MIME types based on file_type
        let (source_mime_type, target_mime_type, reader): (String, String, Box<dyn ReadSeek>) =
            match file_type {
                "document" => {
                    let body = params.get("body").and_then(|q| q.as_str()).ok_or(
                        ToolError::InvalidParameters(
                            "The body param is required for document file type".to_string(),
                        ),
                    )?;

                    (
                        "text/markdown".to_string(),
                        "application/vnd.google-apps.document".to_string(),
                        Box::new(Cursor::new(body.as_bytes().to_owned())),
                    )
                }
                "spreadsheet" => {
                    let body = params.get("body").and_then(|q| q.as_str()).ok_or(
                        ToolError::InvalidParameters(
                            "The body param is required for spreadsheet file type".to_string(),
                        ),
                    )?;
                    (
                        "text/csv".to_string(),
                        "application/vnd.google-apps.spreadsheet".to_string(),
                        Box::new(Cursor::new(body.as_bytes().to_owned())),
                    )
                }
                "slides" => {
                    let path = params.get("path").and_then(|q| q.as_str()).ok_or(
                        ToolError::InvalidParameters(
                            "The path param is required for slides file type".to_string(),
                        ),
                    )?;

                    let file = std::fs::File::open(path).map_err(|e| {
                        ToolError::ExecutionError(
                            format!("Error opening {}: {}", path, e).to_string(),
                        )
                    })?;

                    (
                        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                            .to_string(),
                        "application/vnd.google-apps.presentation".to_string(),
                        Box::new(file),
                    )
                }
                "folder" => {
                    let emptybuf: [u8; 0] = [];
                    let empty_stream = Cursor::new(emptybuf);
                    (
                        "application/vnd.google-apps.folder".to_string(),
                        "application/vnd.google-apps.folder".to_string(),
                        Box::new(empty_stream),
                    )
                }
                "shortcut" => {
                    if target_id.is_none() {
                        return Err(ToolError::InvalidParameters(
                            "The targetId param is required when creating a shortcut".to_string(),
                        ))
                    }
                    let emptybuf: [u8; 0] = [];
                    let empty_stream = Cursor::new(emptybuf);
                    (
                        "application/vnd.google-apps.shortcut".to_string(),
                        "application/vnd.google-apps.shortcut".to_string(),
                        Box::new(empty_stream),
                    )
                }

                _ => {
                    return Err(ToolError::InvalidParameters(format!(
                        "Invalid fileType: {}. Supported types are: document, spreadsheet, slides, folder, shortcut",
                        file_type
                    )))
                }
            };

        // Upload the file to Google Drive
        self.upload_to_drive(
            FileOperation::Create {
                name: filename.to_string(),
            },
            reader,
            &source_mime_type,
            &target_mime_type,
            parent_id,
            allow_shared_drives,
            target_id,
        )
        .await
    }

    async fn move_file(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let file_id =
            params
                .get("fileId")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileId param is required".to_string(),
                ))?;
        let current_folder_id = params
            .get("currentFolderId")
            .and_then(|q| q.as_str())
            .ok_or(ToolError::InvalidParameters(
                "The currentFolderId param is required".to_string(),
            ))?;
        let new_folder_id = params.get("newFolderId").and_then(|q| q.as_str()).ok_or(
            ToolError::InvalidParameters("The newFolderId param is required".to_string()),
        )?;
        let req = File::default();
        let result = self
            .drive
            .files()
            .update(req, file_id)
            .add_parents(new_folder_id)
            .remove_parents(current_folder_id)
            .clear_scopes()
            .add_scope(GOOGLE_DRIVE_SCOPES)
            .supports_all_drives(true)
            .doit_without_upload()
            .await;

        match result {
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to move google drive file {}, {}.",
                file_id, e
            ))),
            Ok(r) => Ok(vec![Content::text(format!(
                "{} ({}) (uri: {})",
                r.1.name.unwrap_or_default(),
                r.1.mime_type.unwrap_or_default(),
                r.1.id.unwrap_or_default()
            ))]),
        }
    }

    async fn update_file(&self, params: Value) -> Result<Vec<Content>, ToolError> {
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

        let allow_shared_drives = params
            .get("allowSharedDrives")
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
            allow_shared_drives,
            None,
        )
        .await
    }

    async fn update_google_file(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        // Extract common parameters
        let file_id =
            params
                .get("fileId")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileId param is required".to_string(),
                ))?;

        let file_type =
            params
                .get("fileType")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileType param is required".to_string(),
                ))?;

        let allow_shared_drives = params
            .get("allowSharedDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();

        // Determine source and target MIME types based on file_type
        let (source_mime_type, target_mime_type, reader): (String, String, Box<dyn ReadSeek>) =
            match file_type {
                "document" => {
                    let body = params.get("body").and_then(|q| q.as_str()).ok_or(
                        ToolError::InvalidParameters(
                            "The body param is required for document file type".to_string(),
                        ),
                    )?;

                    (
                        "text/markdown".to_string(),
                        "application/vnd.google-apps.document".to_string(),
                        Box::new(Cursor::new(body.as_bytes().to_owned())),
                    )
                }
                "spreadsheet" => {
                    let body = params.get("body").and_then(|q| q.as_str()).ok_or(
                        ToolError::InvalidParameters(
                            "The body param is required for spreadsheet file type".to_string(),
                        ),
                    )?;
                    (
                        "text/csv".to_string(),
                        "application/vnd.google-apps.spreadsheet".to_string(),
                        Box::new(Cursor::new(body.as_bytes().to_owned())),
                    )
                }
                "slides" => {
                    let path = params.get("path").and_then(|q| q.as_str()).ok_or(
                        ToolError::InvalidParameters(
                            "The path param is required for slides file type".to_string(),
                        ),
                    )?;

                    let file = std::fs::File::open(path).map_err(|e| {
                        ToolError::ExecutionError(
                            format!("Error opening {}: {}", path, e).to_string(),
                        )
                    })?;

                    (
                        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                            .to_string(),
                        "application/vnd.google-apps.presentation".to_string(),
                        Box::new(file),
                    )
                }
                _ => {
                    return Err(ToolError::InvalidParameters(format!(
                        "Invalid fileType: {}. Supported types are: document, spreadsheet, slides",
                        file_type
                    )))
                }
            };

        // Upload the file to Google Drive
        self.upload_to_drive(
            FileOperation::Update {
                file_id: file_id.to_string(),
            },
            reader,
            &source_mime_type,
            &target_mime_type,
            None,
            allow_shared_drives,
            None,
        )
        .await
    }

    async fn get_comments(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let file_id =
            params
                .get("fileId")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileId param is required".to_string(),
                ))?;

        let mut results: Vec<String> = Vec::new();
        let mut state = PaginationState::Start;
        while state != PaginationState::End {
            let mut comment_list = self
                .drive
                .comments()
                .list(file_id)
                // 100 is the maximum according to the API.
                .page_size(100)
                .param("fields", "*")
                .clear_scopes()
                .add_scope(GOOGLE_DRIVE_SCOPES);
            if let PaginationState::Next(pt) = state {
                comment_list = comment_list.page_token(&pt);
            }
            let result = comment_list.doit().await;
            match result {
                Err(e) => {
                    return Err(ToolError::ExecutionError(format!(
                        "Failed to execute google drive comment list, {}.",
                        e
                    )))
                }
                Ok(r) => {
                    let mut content =
                        r.1.comments
                            .map(|comments| {
                                comments.into_iter().map(|c| {
                                    format!(
                                        "Author:{:?} Quoted File Content: {:?} Content: {} Replies: {:?} (created time: {}) (modified time: {})(anchor: {}) (resolved: {}) (id: {})",
                                        c.author.unwrap_or_default(),
                                        c.quoted_file_content.unwrap_or_default(),
                                        c.content.unwrap_or_default(),
                                        c.replies.unwrap_or_default(),
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
                            .collect::<Vec<_>>();
                    results.append(&mut content);
                    state = match r.1.next_page_token {
                        Some(npt) => PaginationState::Next(npt),
                        None => PaginationState::End,
                    }
                }
            }
        }
        Ok(vec![Content::text(results.join("\n"))])
    }

    async fn create_comment(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let file_id =
            params
                .get("fileId")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileId param is required".to_string(),
                ))?;
        let comment =
            params
                .get("comment")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The comment param is required".to_string(),
                ))?;

        let req = Comment {
            content: Some(comment.to_string()),
            ..Default::default()
        };
        let result = self
            .drive
            .comments()
            .create(req, file_id)
            .clear_scopes() // Scope::MeetReadonly is the default, remove it
            .add_scope(GOOGLE_DRIVE_SCOPES)
            .param("fields", "*")
            // .param("fields", "action, author, content, createdTime, id")
            .doit()
            .await;
        match result {
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to add comment for google drive file {}, {}.",
                file_id, e
            ))),
            Ok(r) => Ok(vec![Content::text(format!(
                "Author: {:?} Content: {} Created: {} uri: {} quoted_content: {:?}",
                r.1.author.unwrap_or_default(),
                r.1.content.unwrap_or_default(),
                r.1.created_time.unwrap_or_default(),
                r.1.id.unwrap_or_default(),
                r.1.quoted_file_content.unwrap_or_default()
            ))]),
        }
    }

    async fn reply(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let file_id =
            params
                .get("fileId")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileId param is required".to_string(),
                ))?;
        let comment_id = params.get("commentId").and_then(|q| q.as_str()).ok_or(
            ToolError::InvalidParameters("The commentId param is required".to_string()),
        )?;
        let content =
            params
                .get("content")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The content param is required if the action is create".to_string(),
                ))?;
        let resolve_comment = params
            .get("resolveComment")
            .and_then(|q| q.as_bool())
            .unwrap_or(false);

        let mut req = Reply {
            content: Some(content.to_string()),
            ..Default::default()
        };

        if resolve_comment {
            req.action = Some("resolve".to_string());
        }
        let result = self
            .drive
            .replies()
            .create(req, file_id, comment_id)
            .clear_scopes() // Scope::MeetReadonly is the default, remove it
            .add_scope(GOOGLE_DRIVE_SCOPES)
            .param("fields", "action, author, content, createdTime, id")
            .doit()
            .await;
        match result {
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to manage reply to comment {} for google drive file {}, {}.",
                comment_id, file_id, e
            ))),
            Ok(r) => Ok(vec![Content::text(format!(
                "Action: {} Author: {:?} Content: {} Created: {} uri: {}",
                r.1.action.unwrap_or_default(),
                r.1.author.unwrap_or_default(),
                r.1.content.unwrap_or_default(),
                r.1.created_time.unwrap_or_default(),
                r.1.id.unwrap_or_default()
            ))]),
        }
    }

    async fn list_drives(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let query = params.get("name_contains").and_then(|q| q.as_str());

        let mut results: Vec<String> = Vec::new();
        let mut state = PaginationState::Start;
        while state != PaginationState::End {
            let mut builder = self
                .drive
                .drives()
                .list()
                .page_size(100)
                .clear_scopes() // Scope::MeetReadonly is the default, remove it
                .add_scope(GOOGLE_DRIVE_SCOPES);
            if let Some(q) = query {
                builder = builder.q(format!("name contains '{}'", q).as_str());
            }
            if let PaginationState::Next(pt) = state {
                builder = builder.page_token(&pt);
            }
            let result = builder.doit().await;

            match result {
                Err(e) => {
                    return Err(ToolError::ExecutionError(format!(
                        "Failed to execute google drive list, {}.",
                        e
                    )))
                }
                Ok(r) => {
                    let mut content =
                        r.1.drives
                            .map(|drives| {
                                drives.into_iter().map(|f| {
                                    format!(
                                        "{} (capabilities: {:?}) (uri: {})",
                                        f.name.unwrap_or_default(),
                                        f.capabilities.unwrap_or_default(),
                                        f.id.unwrap_or_default()
                                    )
                                })
                            })
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>();
                    results.append(&mut content);
                    state = match r.1.next_page_token {
                        Some(npt) => PaginationState::Next(npt),
                        None => PaginationState::End,
                    }
                }
            }
        }
        Ok(vec![Content::text(results.join("\n"))])
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
                "create_file" => this.create_file(arguments).await,
                "move_file" => this.move_file(arguments).await,
                "update_file" => this.update_file(arguments).await,
                "update_google_file" => this.update_google_file(arguments).await,
                "sheets_tool" => this.sheets_tool(arguments).await,
                "create_comment" => this.create_comment(arguments).await,
                "get_comments" => this.get_comments(arguments).await,
                "reply" => this.reply(arguments).await,
                "list_drives" => this.list_drives(arguments).await,
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
