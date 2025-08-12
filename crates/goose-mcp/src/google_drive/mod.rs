mod google_labels;
mod oauth_pkce;
pub mod storage;

use anyhow::{Context, Error};
use base64::Engine;
use chrono::NaiveDate;
use indoc::indoc;
use lazy_static::lazy_static;
use mcp_core::{
    handler::{PromptError, ResourceError, ToolError},
    protocol::ServerCapabilities,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;
use oauth_pkce::PkceOAuth2Client;
use regex::Regex;
use rmcp::model::{
    AnnotateAble, Content, JsonRpcMessage, Prompt, RawResource, Resource, Tool, ToolAnnotations,
};
use rmcp::object;
use serde_json::{json, Value};
use std::io::Cursor;
use std::{env, fs, future::Future, path::Path, pin::Pin, sync::Arc};
use storage::CredentialsManager;
use tokio::sync::mpsc;

use google_docs1::{self, Docs};
use google_drive3::common::ReadSeek;
use google_drive3::{
    self,
    api::{
        Comment, File, FileShortcutDetails, LabelFieldModification, LabelModification,
        ModifyLabelsRequest, Permission, Reply, Scope,
    },
    hyper_rustls::{self, HttpsConnector},
    hyper_util::{self, client::legacy::connect::HttpConnector},
    DriveHub,
};
use google_labels::DriveLabelsHub;
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
const PERMISSIONTYPE: &[&str] = &["user", "group", "domain", "anyone"];
const ROLES: &[&str] = &[
    "owner",
    "organizer",
    "fileOrganizer",
    "writer",
    "commenter",
    "reader",
];

lazy_static! {
    static ref GOOGLE_DRIVE_ID_REGEX: Regex =
        Regex::new(r"^(?:https:\/\/)(?:[\w-]+\.)?google\.com\/(?:[^\/]+\/)*d\/([a-zA-Z0-9_-]+)")
            .unwrap();
}

fn extract_google_drive_id(url: &str) -> Option<&str> {
    GOOGLE_DRIVE_ID_REGEX
        .captures(url)
        .and_then(|caps| caps.get(1).map(|m| m.as_str()))
}

pub struct GoogleDriveRouter {
    tools: Vec<Tool>,
    instructions: String,
    drive: DriveHub<HttpsConnector<HttpConnector>>,
    drive_labels: DriveLabelsHub<HttpsConnector<HttpConnector>>,
    sheets: Sheets<HttpsConnector<HttpConnector>>,
    docs: Docs<HttpsConnector<HttpConnector>>,
    credentials_manager: Arc<CredentialsManager>,
}

impl GoogleDriveRouter {
    async fn google_auth() -> (
        DriveHub<HttpsConnector<HttpConnector>>,
        DriveLabelsHub<HttpsConnector<HttpConnector>>,
        Sheets<HttpsConnector<HttpConnector>>,
        Docs<HttpsConnector<HttpConnector>>,
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
                // Create the PKCE OAuth2 client
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
                let drive_labels_hub = DriveLabelsHub::new(client.clone(), auth.clone());
                let sheets_hub = Sheets::new(client.clone(), auth.clone());
                let docs_hub = Docs::new(client, auth);

                // Create and return the DriveHub, Sheets and our PKCE OAuth2 client
                (
                    drive_hub,
                    drive_labels_hub,
                    sheets_hub,
                    docs_hub,
                    credentials_manager,
                )
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
        let (drive, drive_labels, sheets, docs, credentials_manager) = Self::google_auth().await;

        let search_tool = Tool::new(
            "search".to_string(),
            indoc! {r#"
                List or search for files or labels in google drive by name, given an input search query. At least one of ('name', 'mimeType', or 'parent') are required for file searches.
            "#}
                .to_string(),
            object!({
              "type": "object",
              "properties": {
                "driveType": {
                    "type": "string",
                    "description": "Required type of object to list or search (file, label)."
                },
                "name": {
                    "type": "string",
                    "description": "String to search for in the file's name.",
                },
                "mimeType": {
                    "type": "string",
                    "description": "Use when searching for a file to constrain the results to just this MIME type.",
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
                },
                "includeLabels": {
                    "type": "boolean",
                    "description": "When searching or listing files, also get any applied labels.",
                }
              },
              "required": ["driveType"],
            })
        ).annotate(ToolAnnotations {
            title: Some("Search GDrive".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let read_tool = Tool::new(
            "read".to_string(),
            indoc! {r#"
                Read a file from google drive using the file URI or the full google drive URL.
                One of URI or URL MUST is required.

                Optionally include base64 encoded images, false by default.

                Example extracting URIs from URLs:
                Given "https://docs.google.com/document/d/1QG8d8wtWe7ZfmG93sW-1h2WXDJDUkOi-9hDnvJLmWrc/edit?tab=t.0#heading=h.5v419d3h97tr"
                Pass in "gdrive:///1QG8d8wtWe7ZfmG93sW-1h2WXDJDUkOi-9hDnvJLmWrc"
                Do not include any other path parameters when using URI.
            "#}
                .to_string(),
            object!({
              "type": "object",
              "properties": {
                  "uri": {
                      "type": "string",
                      "description": "google drive uri of the file to read, use this when you have the file URI",
                  },
                  "url": {
                      "type": "string",
                      "description": "the full google drive URL to read the file from, use this when the user gives a full https url",
                  },
                  "includeImages": {
                      "type": "boolean",
                      "description": "Whether or not to include images as base64 encoded strings, defaults to false",
                  }
              },
            })
        ).annotate(ToolAnnotations {
            title: Some("Read GDrive".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false)
        });

        let create_file_tool = Tool::new(
            "create_file".to_string(),
            indoc! {r#"
                Create a new file, including Document, Spreadsheet, Slides, folder, or shortcut, in Google Drive.
            "#}
                .to_string(),
            object!({
              "type": "object",
              "properties": {
                  "name": {
                      "type": "string",
                      "description": "Name of the file to create",
                  },
                  "mimeType": {
                      "type": "string",
                      "description": "The MIME type of the file.",
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
              "required": ["name", "mimeType"],
            })
        ).annotate(ToolAnnotations {
            title: Some("Create new file in GDrive".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let move_file_tool = Tool::new(
            "move_file".to_string(),
            indoc! {r#"
                Move a Google Drive file, folder, or shortcut to a new parent folder. You cannot move a folder to a different drive.
            "#}
                .to_string(),
            object!({
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
            })
        ).annotate(ToolAnnotations {
            title: Some("Move file".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let update_file_tool = Tool::new(
            "update_file".to_string(),
            indoc! {r#"
                Update an existing file in Google Drive with new content or edit the file's labels.
            "#}
                .to_string(),
            object!({
              "type": "object",
              "properties": {
                  "fileId": {
                      "type": "string",
                      "description": "The ID of the file to update.",
                  },
                  "allowSharedDrives": {
                      "type": "boolean",
                      "description": "Whether to allow access to shared drives or just your personal drive (default: false)",
                  },
                  "mimeType": {
                      "type": "string",
                      "description": "The MIME type of the file.",
                  },
                  "body": {
                      "type": "string",
                      "description": "Plain text body of the file to upload. Mutually exclusive with path (required for Google Document and Google Spreadsheet types).",
                  },
                  "path": {
                      "type": "string",
                      "description": "Path to a local file to use to update the Google Drive file. Mutually exclusive with body (required for Google Slides type)",
                  },
                  "updateLabels": {
                      "type": "array",
                      "description": "Array of label operations to perform on the file. Each operation may remove one label, unset one field, or update one field.",
                      "items": {
                          "type": "object",
                          "properties": {
                              "labelId": {
                                  "type": "string",
                                  "description": "The ID of the label to be operated upon."
                              },
                              "operation": {
                                  "type": "string",
                                  "enum":  ["removeLabel", "unsetField", "addOrUpdateLabel"],
                                  "description": "The operation to perform. You may 'removeLabel' to completely remove the label from the file, 'unsetField' to remove a field from an applied label, or 'addOrUpdateLabel' to add a new label (with or without fields), or change the value of a field on an applied label."
                              },
                              "fieldId": {
                                  "type": "string",
                                  "description": "The ID of the field to be operated upon."
                              },
                              "dateValue": {
                                  "type": "array",
                                  "description": "If updating a date field, an array of RFC 3339 dates (format YYYY-MM-DD) to update to.",
                                  "items": {
                                      "type": "string",
                                      "description": "An RFC 3339 full-date format YYYY-MM-DD.",
                                  }
                              },
                              "textValue": {
                                  "type": "array",
                                  "description": "If updating a text field, the string values to update to.",
                                  "items": {
                                      "type": "string",
                                      "description": "Text field values.",
                                  }
                              },
                              "choiceValue": {
                                  "type": "array",
                                  "description": "If updating a Choice field, the ID(s) of the desired choice field(s).",
                                  "items": {
                                      "type": "string",
                                      "description": "Choice ID as a string",
                                  }
                              },
                              "integerValue": {
                                  "type": "array",
                                  "description": "If updating an integer field, the integer values to use.",
                                  "items": {
                                      "type": "integer",
                                      "description": "The integer value.",
                                  }
                              },
                              "userValue": {
                                  "type": "array",
                                  "description": "If updating a user field, an array of the email address(es) of the user(s) to set as the field value.",
                                  "items": {
                                      "type": "string",
                                      "description": "Email address as a string",
                                  }
                              }
                          }
                      }
                  },
              },
              "required": ["fileId"],
              "dependentRequired": {
                  "body": ["mimeType"],
                  "path": ["mimeType"]
              }
            })
        ).annotate(ToolAnnotations {
            title: Some("Update a file's contents or labels".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

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
            object!({
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
            })
        ).annotate(ToolAnnotations {
            title: Some("Work with Google Sheets data using various operations.".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let docs_tool = Tool::new(
            "docs_tool".to_string(),
            indoc! {r#"
                Work with Google Docs data using various operations.
                Supports operations:
                - get_document: Get the full document content
                - insert_text: Insert text at a specific location
                - append_text: Append text to the end of the document
                - replace_text: Replace all instances of text
                - create_paragraph: Create a new paragraph
                - delete_content: Delete content between positions
            "#}
                .to_string(),
            object!({
              "type": "object",
              "properties": {
                  "documentId": {
                      "type": "string",
                      "description": "The ID of the document to work with",
                  },
                  "operation": {
                      "type": "string",
                      "enum": ["get_document", "insert_text", "append_text", "replace_text", "create_paragraph", "delete_content"],
                      "description": "The operation to perform on the document",
                  },
                  "text": {
                      "type": "string",
                      "description": "The text to insert, append, or use for replacement",
                  },
                  "replaceText": {
                      "type": "string",
                      "description": "The text to be replaced",
                  },
                  "position": {
                      "type": "number",
                      "description": "The position in the document (index) for operations that require a position",
                  },
                  "startPosition": {
                      "type": "number",
                      "description": "The start position for delete_content operation",
                  },
                  "endPosition": {
                      "type": "number",
                      "description": "The end position for delete_content operation",
                  }
              },
              "required": ["documentId", "operation"],
            })
        ).annotate(ToolAnnotations {
            title: Some("Work with Google Docs data using various operations.".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let get_comments_tool = Tool::new(
            "get_comments".to_string(),
            indoc! {r#"
                List comments for a file in google drive.
            "#}
            .to_string(),
            object!({
              "type": "object",
              "properties": {
                "fileId": {
                    "type": "string",
                    "description": "Id of the file to list comments for.",
                }
              },
              "required": ["fileId"],
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("List file comments".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let manage_comment_tool = Tool::new(
            "manage_comment".to_string(),
            indoc! {r#"
                Manage comment for a Google Drive file.

                Supports the operations:
                - create: Create a comment for the latest revision of a Google Drive file. The Google Drive API only supports unanchored comments (they don't refer to a specific location in the file).
                - reply: Add a reply to a comment thread, or resolve a comment.
            "#}
                .to_string(),
            object!({
              "type": "object",
              "properties": {
                "fileId": {
                    "type": "string",
                    "description": "Id of the file.",
                },
                "operation": {
                    "type": "string",
                    "description": "Desired comment management operation.",
                    "enum": ["create", "reply"],
                },
                "content": {
                    "type": "string",
                    "description": "Content of the comment to create or reply.",
                },
                "commentId": {
                    "type": "string",
                    "description": "Id of the comment to which you'd like to reply. ",
                },
                "resolveComment": {
                    "type": "boolean",
                    "description": "Whether to resolve the comment in reply. Defaults to false.",
                }
              },
              "required": ["fileId", "operation", "content"],
            })
        ).annotate(ToolAnnotations {
            title: Some("Manage file comment".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let list_drives_tool = Tool::new(
            "list_drives".to_string(),
            indoc! {r#"
                List shared Google drives.
            "#}
            .to_string(),
            object!({
              "type": "object",
              "properties": {
                "name_contains": {
                    "type": "string",
                    "description": "Optional name to search for when listing drives.",
                }
              },
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("List shared google drives".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let get_permissions_tool = Tool::new(
            "get_permissions".to_string(),
            indoc! {r#"
                List sharing permissions for a file, folder, or shared drive.
            "#}
            .to_string(),
            object!({
              "type": "object",
              "properties": {
                "fileId": {
                    "type": "string",
                    "description": "Id of the file, folder, or shared drive.",
                }
              },
              "required": ["fileId"],
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("List sharing permissions".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let sharing_tool = Tool::new(
            "sharing".to_string(),
            indoc! {r#"
                Manage sharing for a Google Drive file or folder.

                Supports the operations:
                - create: Create a new permission for a 'type' identified by the 'target' param to have the 'role' privileges.
                - update: Update an existing permission to a different role. (You cannot change the type or to whom it is targeted).
                - delete: Delete an existing permission.
            "#}
                .to_string(),
            object!({
              "type": "object",
              "properties": {
                "fileId": {
                    "type": "string",
                    "description": "Id of the file or folder.",
                },
                "operation": {
                    "type": "string",
                    "description": "Desired sharing operation.",
                    "enum": ["create", "update", "delete"],
                },
                "permissionId": {
                    "type": "string",
                    "description": "Permission Id for delete or update operations.",
                },
                "role": {
                    "type": "string",
                    "description": "Role to apply to permission for create or update operations.",
                    "enum": ["owner", "organizer", "fileOrganizer", "writer", "commenter", "reader"]
                },
                "type": {
                    "type": "string",
                    "description": "Type of permission to create or update.",
                    "enum": ["user", "group", "domain", "anyone"],
                },
                "target": {
                    "type": "string",
                    "description": "For the user and group types, the email address. For a domain type, the domain name. (The anyone type does not require a target). Required for the create operation.",
                },
                "emailMessage": {
                    "type": "string",
                    "description": "Email notification message to send to users and groups.",
                },
              },
              "required": ["fileId", "operation"],
            })
        ).annotate(ToolAnnotations {
            title: Some("Manage file sharing".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        });

        let instructions = indoc::formatdoc! {r#"
            Google Drive MCP Server Instructions

            ## Overview
            The Google Drive MCP server provides tools for interacting with Google Drive files, Google Sheets, and Google Docs:
            1. search - List or search for files or labels in your Google Drive
            2. read - Read file contents directly using a uri in the `gdrive:///uri` format
            3. move_file - Move a file to a new location in Google Drive
            4. list_drives - List the shared drives to which you have access
            5. get_permissions - List the permissions of a file or folder
            6. sharing - Share a file or folder with others
            7. get_comments - List a file or folder's comments
            8. manage_comment - Manage comment for a Google Drive file.
            9. create_file - Create a new file
            10. update_file - Update an existing file's contents or labels
            11. sheets_tool - Work with Google Sheets data using various operations
            12. docs_tool - Work with Google Docs data using various operations

            ## Available Tools

            ### 1. Search Tool
            Search for or list files or labels in Google Drive. Files are
            searched by name and ordered by most recently viewedByMeTime.
            A corpora parameter controls which corpus is searched.
            Returns: List of files with their names, MIME types, and IDs or a
            list of labels and their fields.

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

            #### File Format Handling
            The read file tool's output will be converted:
            - Google Docs → Markdown
            - Google Sheets → CSV
            - Google Presentations → Plain text
            - Text/JSON files → UTF-8 text
            - Binary files → Base64 encoded

            ### 3. Move File Tool
            Move a file from its current folder to a new folder, including folders on another drive.

            ### 4. List Drives Tool
            Lists the user's available Shared Drives.

            ### 5. Get Permissions Tool
            Lists the permissions for a file or folder. Permissions in Google
            Drive consist of a type ('user', 'group', 'domain', 'anyone') and a role
            ('owner', 'organizer', 'fileOrganizer', 'writer', 'commenter',
            'reader').

            ### 6. Sharing Tool
            Create a new permission, update the role on an existing permission,
            or delete a permission. User, group, and domain permissions should
            have a provided "target" email address or domain name.

            ### 7. Get Comments Tool
            Lists the comments for a Google Workspace file.

            ### 8. Manage Comment Tool
            Create or reply comment for a Google Drive file.

            ### 9. Create File Tool
            Create any kind of file, including Google Workspace files (Docs, Sheets, or Slides) directly in Google Drive.
            - For Google Docs: Converts Markdown text to a Google Document
            - For Google Sheets: Converts CSV text to a Google Spreadsheet
            - For Google Slides: Converts a PowerPoint file to Google Slides (requires a path to the powerpoint file)
            - Other: No file conversion.

            *Note*: All updates overwrite the existing content with the new
            content provided. To modify specific parts of the document, you must
            include the changes as part of the entire document.

            ### 10. Update File Tool
            Replace the entire contents of an existing file with new content,
            including Google Workspace files (Docs, Sheets, or Slides), or
            update the labels applied to a file.
            - For Google Docs: Updates with new Markdown text
            - For Google Sheets: Updates with new CSV text
            - For Google Slides: Updates with a new PowerPoint file (requires a path to the powerpoint file)
            - Other: No file conversion.

            Label operations include adding a new label, unsetting a field for
            an already-applied label, removing a label, or changing the field
            value for an applied label.

            ### 11. Sheets Tool
            Work with Google Sheets data using various operations:
            - list_sheets: List all sheets in a spreadsheet
            - get_columns: Get column headers from a specific sheet
            - get_values: Get values from a range
            - update_values: Update values in a range (requires CSV formatted data)
            - update_cell: Update a single cell value
            - add_sheet: Add a new sheet (tab) to a spreadsheet
            - clear_values: Clear values from a range

            For update_values operation, provide CSV formatted data in the values parameter.
            Each line represents a row, with values separated by commas.
            Example: "John,Doe,30\nJane,Smith,25"

            For update_cell operation, provide the cell reference (e.g., 'Sheet1!A1') and the value to set.

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

            ### 12. Docs Tool
            Work with Google Docs data using various operations:
            - get_document: Get the full document content
            - insert_text: Insert text at a specific location
            - append_text: Append text to the end of the document
            - replace_text: Replace all instances of text
            - create_paragraph: Create a new paragraph
            - delete_content: Delete content between positions

            Parameters:
            - documentId: The ID of the document (can be obtained from search results)
            - operation: The operation to perform (one of the operations listed above)
            - text: The text to insert, append, or use for replacement
            - replaceText: The text to be replaced (for replace_text operation)
            - position: The position in the document (index) for operations that require a position
            - startPosition: The start position for delete_content operation
            - endPosition: The end position for delete_content operation

            ## Common Usage Pattern

            1. First, search for the file you want to read, searching by name.
            2. Then, use the file URI from the search results to read its contents.
            3. For Google Sheets, use the sheets_tool with the appropriate operation.
            4. For Google Docs, use the docs_tool with the appropriate operation.

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
                create_file_tool,
                move_file_tool,
                update_file_tool,
                sheets_tool,
                docs_tool,
                get_comments_tool,
                manage_comment_tool,
                list_drives_tool,
                get_permissions_tool,
                sharing_tool,
            ],
            instructions,
            drive,
            drive_labels,
            sheets,
            docs,
            credentials_manager,
        }
    }

    // Implement search tool functionality
    async fn search(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        // To minimize tool growth, we search/list for a number of different
        // objects in Gdrive with sub-funcs.
        let drive_type = params.get("driveType").and_then(|q| q.as_str()).ok_or(
            ToolError::InvalidParameters("The type is required".to_string()),
        )?;
        match drive_type {
            "file" => return self.search_files(params).await,
            "label" => return self.list_labels(params).await,
            t => Err(ToolError::InvalidParameters(format!(
                "type must be one of ('file', 'label'), got {}",
                t
            ))),
        }
    }

    async fn search_files(&self, params: Value) -> Result<Vec<Content>, ToolError> {
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

        let include_labels = params
            .get("includeLabels")
            .and_then(|b| b.as_bool())
            .unwrap_or(false);

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
            .param(
                "fields",
                &format!(
                    "files(id, name, mimeType, modifiedTime, size{})",
                    if include_labels { ", labelInfo" } else { "" }
                ),
            )
            .page_size(page_size)
            .supports_all_drives(true)
            .include_items_from_all_drives(true)
            .clear_scopes() // Scope::MeetReadonly is the default, remove it
            .add_scope(GOOGLE_DRIVE_SCOPES);
        // You can only use the drive_id param when the corpus is "drive".
        if let (Some(d), "drive") = (drive_id, corpus) {
            builder = builder.drive_id(d);
        }
        // If we want labels, we have to go look up the IDs first.
        // let mut label_results: Vec<Label> = Vec::new();
        if include_labels {
            let label_builder = self
                .drive_labels
                .labels()
                .list()
                .param("view", "LABEL_VIEW_BASIC");
            // .param("view", "LABEL_VIEW_FULL");

            let label_results = match label_builder.doit().await {
                Ok(r) => r.1.labels.unwrap_or_default(),
                Err(e) => {
                    return Err(ToolError::ExecutionError(format!(
                        "Failed to execute google drive label list '{}'.",
                        e
                    )))
                }
            };
            let label_ids = label_results
                .iter()
                .filter_map(|l| l.id.clone())
                .collect::<Vec<_>>()
                .join(",");
            builder = builder.include_labels(&label_ids);
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
                                    "{} ({}) (uri: {}){}",
                                    f.name.unwrap_or_default(),
                                    f.mime_type.unwrap_or_default(),
                                    f.id.unwrap_or_default(),
                                    if include_labels {
                                        format!(" (labels: {:?})", f.label_info.unwrap_or_default())
                                    } else {
                                        "".to_string()
                                    }
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
    #[cfg(feature = "screen_capture")]
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
    #[cfg(feature = "screen_capture")]
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
                            let content = self.strip_image_body(&response);
                            let contents = vec![Content::text(content).with_priority(0.1)];

                            #[cfg(feature = "screen_capture")]
                            {
                                match self.resize_images(&response) {
                                    Ok(images) => contents.extend(images),
                                    Err(e) => {
                                        return Err(ToolError::ExecutionError(format!(
                                            "Failed to resize image(s): {}",
                                            e
                                        )))
                                    }
                                }
                            }
                            Ok(contents)
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
        mime_type: &str,
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
                if mime_type.starts_with("text/") || mime_type == "application/json" {
                    if let Ok(body) = r.0.into_body().collect().await {
                        if let Ok(response) = String::from_utf8(body.to_bytes().to_vec()) {
                            if !include_images {
                                let content = self.strip_image_body(&response);
                                Ok(vec![Content::text(content).with_priority(0.1)])
                            } else {
                                let content = self.strip_image_body(&response);
                                let contents = vec![Content::text(content).with_priority(0.1)];
                                #[cfg(feature = "screen_capture")]
                                {
                                    match self.resize_images(&response) {
                                        Ok(images) => contents.extend(images),
                                        Err(e) => {
                                            return Err(ToolError::ExecutionError(format!(
                                                "Failed to resize image(s): {}",
                                                e
                                            )))
                                        }
                                    }
                                }
                                Ok(contents)
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
        let (maybe_uri, maybe_url) = (
            params.get("uri").and_then(|q| q.as_str()),
            params.get("url").and_then(|q| q.as_str()),
        );

        let drive_uri = match (maybe_uri, maybe_url) {
            (Some(uri), None) => {
                let drive_uri = uri.replace("gdrive:///", "");

                // Validation: check for / path separators as invalid uris
                if drive_uri.contains('/') {
                    return Err(ToolError::InvalidParameters(format!(
                        "The uri '{}' contains extra '/'. Only the base URI is allowed.",
                        uri
                    )));
                }

                drive_uri
            }
            (None, Some(url)) => {
                if let Some(drive_uri) = extract_google_drive_id(url) {
                    drive_uri.to_string()
                } else {
                    return Err(ToolError::InvalidParameters(format!(
                        "Failed to extract valid google drive URI from {}",
                        url
                    )));
                }
            }
            (Some(_), Some(_)) => {
                return Err(ToolError::InvalidParameters(
                    "Only one of 'uri' or 'url' should be provided".to_string(),
                ));
            }
            (None, None) => {
                return Err(ToolError::InvalidParameters(
                    "Either 'uri' or 'url' must be provided".to_string(),
                ));
            }
        };

        let include_images = params
            .get("includeImages")
            .and_then(|i| i.as_bool())
            .unwrap_or(false);

        let metadata = self.fetch_file_metadata(&drive_uri).await?;
        let mime_type = metadata.mime_type.ok_or_else(|| {
            ToolError::ExecutionError(format!(
                "Missing mime type in file metadata for {}.",
                drive_uri
            ))
        })?;

        // Handle Google Docs export
        if mime_type.starts_with("application/vnd.google-apps") {
            self.export_google_file(&drive_uri, &mime_type, include_images)
                .await
        } else {
            self.get_google_file(&drive_uri, &mime_type, include_images)
                .await
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
                    .map(|content| {
                        content
                            .as_text()
                            .map(|text| text.text.clone())
                            .unwrap_or_default()
                    })
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
                        files.into_iter().map(|f| {
                            RawResource {
                                uri: f.id.unwrap_or_default(),
                                mime_type: f.mime_type,
                                name: f.name.unwrap_or_default(),
                                description: None,
                                size: None,
                            }
                            .no_annotation()
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

    async fn create_file(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        // Extract common parameters
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

        let parent_id = params.get("parentId").and_then(|q| q.as_str());
        let target_id = params.get("targetId").and_then(|q| q.as_str());
        let body = params.get("body").and_then(|q| q.as_str());
        let path = params.get("path").and_then(|q| q.as_str());

        let allow_shared_drives = params
            .get("allowSharedDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();

        // Determine source and target MIME types based on file_type
        let (source_mime_type, target_mime_type, reader): (String, String, Box<dyn ReadSeek>) =
            match mime_type {
                "application/vnd.google-apps.document" => {
                    if body.is_none() {
                        return Err(ToolError::InvalidParameters(
                            "The body param is required for google document file type".to_string(),
                        ));
                    }

                    (
                        "text/markdown".to_string(),
                        mime_type.to_string(),
                        Box::new(Cursor::new(body.unwrap().as_bytes().to_owned())),
                    )
                }
                "application/vnd.google-apps.spreadsheet" => {
                    if body.is_none() {
                        return Err(ToolError::InvalidParameters(
                            "The body param is required for google spreadsheet file type"
                                .to_string(),
                        ));
                    }

                    (
                        "text/csv".to_string(),
                        mime_type.to_string(),
                        Box::new(Cursor::new(body.unwrap().as_bytes().to_owned())),
                    )
                }
                "application/vnd.google-apps.presentation" => {
                    if path.is_none() {
                        return Err(ToolError::InvalidParameters(
                            "The path param is required for google slides file type".to_string(),
                        ));
                    }

                    let file = std::fs::File::open(path.unwrap()).map_err(|e| {
                        ToolError::ExecutionError(
                            format!("Error opening {}: {}", path.unwrap(), e).to_string(),
                        )
                    })?;

                    (
                        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                            .to_string(),
                        mime_type.to_string(),
                        Box::new(file),
                    )
                }
                "application/vnd.google-apps.folder" => {
                    let emptybuf: [u8; 0] = [];
                    let empty_stream = Cursor::new(emptybuf);
                    (
                        mime_type.to_string(),
                        mime_type.to_string(),
                        Box::new(empty_stream),
                    )
                }
                "application/vnd.google-apps.shortcut" => {
                    if target_id.is_none() {
                        return Err(ToolError::InvalidParameters(
                            "The targetId param is required when creating a shortcut".to_string(),
                        ));
                    }
                    let emptybuf: [u8; 0] = [];
                    let empty_stream = Cursor::new(emptybuf);
                    (
                        mime_type.to_string(),
                        mime_type.to_string(),
                        Box::new(empty_stream),
                    )
                }

                _ => {
                    let reader: Box<dyn ReadSeek> = match (body, path) {
                        (None, None) | (Some(_), Some(_)) => {
                            return Err(ToolError::InvalidParameters(
                                "Either the body or path param is required".to_string(),
                            ))
                        }
                        (Some(b), None) => Box::new(Cursor::new(b.as_bytes().to_owned())),
                        (None, Some(p)) => Box::new(std::fs::File::open(p).map_err(|e| {
                            ToolError::ExecutionError(
                                format!("Error opening {}: {}", p, e).to_string(),
                            )
                        })?),
                    };
                    (mime_type.to_string(), mime_type.to_string(), reader)
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

        let allow_shared_drives = params
            .get("allowSharedDrives")
            .and_then(|q| q.as_bool())
            .unwrap_or_default();

        let mime_type = params.get("mimeType").and_then(|q| q.as_str());
        let body = params.get("body").and_then(|q| q.as_str());
        let path = params.get("path").and_then(|q| q.as_str());

        let mut final_result = vec![];

        if mime_type.is_some() && (body.is_some() || path.is_some()) {
            let update_result = self
                .update_file_contents(file_id, mime_type.unwrap(), body, path, allow_shared_drives)
                .await?;
            final_result.extend(update_result);
        };

        if let Some(label_ops) = params.get("updateLabels").and_then(|q| q.as_array()) {
            let label_result = self.update_label(file_id, label_ops).await?;
            final_result.extend(label_result);
        };
        Ok(final_result)
    }

    async fn update_label(
        &self,
        file_id: &str,
        label_ops: &Vec<Value>,
    ) -> Result<Vec<Content>, ToolError> {
        let mut req = ModifyLabelsRequest::default();
        let mut label_mods = vec![];

        for op in label_ops {
            if let Some(op) = op.as_object() {
                let label_id = op.get("labelId").and_then(|o| o.as_str()).ok_or(
                    ToolError::InvalidParameters(
                        "The labelId param is required for label changes".to_string(),
                    ),
                )?;
                match op.get("operation").and_then(|o| o.as_str()) {
                    Some("removeLabel") => {
                        let removal = LabelModification {
                            label_id: Some(label_id.to_string()),
                            remove_label: Some(true),
                            ..Default::default()
                        };
                        label_mods.push(removal);
                    }
                    Some("unsetField") => {
                        let field_id = op.get("fieldId").and_then(|o| o.as_str()).ok_or(
                            ToolError::InvalidParameters(
                                "The fieldId param is required for unsetting a field.".to_string(),
                            ),
                        )?;
                        let field_mods = LabelFieldModification {
                            field_id: Some(field_id.to_string()),
                            unset_values: Some(true),
                            ..Default::default()
                        };
                        let unset_mod = LabelModification {
                            label_id: Some(label_id.to_string()),
                            field_modifications: Some(vec![field_mods]),
                            ..Default::default()
                        };
                        label_mods.push(unset_mod);
                    }
                    Some("addOrUpdateLabel") => {
                        let mut field_mods = LabelFieldModification::default();
                        // Not all labels _have_ fields.
                        if let Some(field_id) = op.get("fieldId").and_then(|o| o.as_str()) {
                            field_mods.field_id = Some(field_id.to_string());
                        }

                        if let Some(date_value) = op.get("dateValue").and_then(|o| o.as_array()) {
                            let parsed_dates: Result<Vec<NaiveDate>, ToolError> = date_value
                                .iter()
                                .filter_map(|d| d.as_str())
                                .map(|d| {
                                    NaiveDate::parse_from_str(d, "%Y-%m-%d").map_err(|e| {
                                        ToolError::InvalidParameters(format!(
                                            "Error parsing field date: {}",
                                            e
                                        ))
                                    })
                                })
                                .collect();

                            field_mods.set_date_values = Some(parsed_dates?);
                        } else if let Some(text_value) =
                            op.get("textValue").and_then(|o| o.as_array())
                        {
                            field_mods.set_text_values = Some(
                                text_value
                                    .iter()
                                    .map(|s| s.as_str().unwrap_or_default().to_string())
                                    .collect(),
                            );
                        } else if let Some(choice_value) =
                            op.get("choiceValue").and_then(|o| o.as_array())
                        {
                            field_mods.set_selection_values = Some(
                                choice_value
                                    .iter()
                                    .map(|s| s.as_str().unwrap_or_default().to_string())
                                    .collect(),
                            );
                        } else if let Some(int_value) =
                            op.get("integerValue").and_then(|o| o.as_array())
                        {
                            field_mods.set_integer_values = Some(
                                int_value
                                    .iter()
                                    .map(|s| s.as_i64().unwrap_or_default())
                                    .collect(),
                            );
                        } else if let Some(user_value) =
                            op.get("userValue").and_then(|o| o.as_array())
                        {
                            field_mods.set_user_values = Some(
                                user_value
                                    .iter()
                                    .map(|s| s.as_str().unwrap_or_default().to_string())
                                    .collect(),
                            );
                        }

                        let update_mod = LabelModification {
                            label_id: Some(label_id.to_string()),
                            field_modifications: Some(vec![field_mods]),
                            ..Default::default()
                        };
                        label_mods.push(update_mod);
                    }
                    _ => {
                        return Err(ToolError::InvalidParameters(format!(
                            "Label operation invalid: {:?}",
                            op.get("operation")
                        )))
                    }
                }
            };
        }
        req.label_modifications = Some(label_mods);

        let result = self.drive.files().modify_labels(req, file_id).doit().await;
        match result {
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to update label for google drive file {}, {}.",
                file_id, e
            ))),
            Ok(r) => Ok(vec![Content::text(format!(
                "file URI: {}, labels modified: {:?}",
                file_id,
                r.1.modified_labels.unwrap_or_default()
            ))]),
        }
    }

    async fn update_file_contents(
        &self,
        file_id: &str,
        mime_type: &str,
        body: Option<&str>,
        path: Option<&str>,
        allow_shared_drives: bool,
    ) -> Result<Vec<Content>, ToolError> {
        // Determine source and target MIME types based on file_type
        let (source_mime_type, target_mime_type, reader): (String, String, Box<dyn ReadSeek>) =
            match mime_type {
                "application/vnd.google-apps.document" => {
                    if body.is_none() {
                        return Err(ToolError::InvalidParameters(
                            "The body param is required for google document file type".to_string(),
                        ));
                    }

                    (
                        "text/markdown".to_string(),
                        mime_type.to_string(),
                        Box::new(Cursor::new(body.unwrap().as_bytes().to_owned())),
                    )
                }
                "application/vnd.google-apps.spreadsheet" => {
                    if body.is_none() {
                        return Err(ToolError::InvalidParameters(
                            "The body param is required for google spreadsheet file type"
                                .to_string(),
                        ));
                    }

                    (
                        "text/csv".to_string(),
                        mime_type.to_string(),
                        Box::new(Cursor::new(body.unwrap().as_bytes().to_owned())),
                    )
                }
                "application/vnd.google-apps.presentation" => {
                    if path.is_none() {
                        return Err(ToolError::InvalidParameters(
                            "The path param is required for google slides file type".to_string(),
                        ));
                    }

                    let file = std::fs::File::open(path.unwrap()).map_err(|e| {
                        ToolError::ExecutionError(
                            format!("Error opening {}: {}", path.unwrap(), e).to_string(),
                        )
                    })?;

                    (
                        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                            .to_string(),
                        mime_type.to_string(),
                        Box::new(file),
                    )
                }
                _ => {
                    let reader: Box<dyn ReadSeek> = match (body, path) {
                        (None, None) | (Some(_), Some(_)) => {
                            return Err(ToolError::InvalidParameters(
                                "Either the body or path param is required".to_string(),
                            ))
                        }
                        (Some(b), None) => Box::new(Cursor::new(b.as_bytes().to_owned())),
                        (None, Some(p)) => Box::new(std::fs::File::open(p).map_err(|e| {
                            ToolError::ExecutionError(
                                format!("Error opening {}: {}", p, e).to_string(),
                            )
                        })?),
                    };
                    (mime_type.to_string(), mime_type.to_string(), reader)
                }
            };

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

    async fn manage_comment(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let file_id =
            params
                .get("fileId")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileId param is required".to_string(),
                ))?;
        let operation = params.get("operation").and_then(|q| q.as_str()).ok_or(
            ToolError::InvalidParameters("The operation is required".to_string()),
        )?;
        let content =
            params
                .get("content")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The content param is required if the action is create".to_string(),
                ))?;

        match operation {
            "create" => {
                let req = Comment {
                    content: Some(content.to_string()),
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
            "reply" => {
                let comment_id = params.get("commentId").and_then(|q| q.as_str()).ok_or(
                    ToolError::InvalidParameters(
                        "The commentId param is required for reply".to_string(),
                    ),
                )?;

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
            _ => Err(ToolError::InvalidParameters(format!(
                "Invalid operation: {}. Supported operations are: create, reply",
                operation
            ))),
        }
    }

    async fn docs_tool(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let document_id = params.get("documentId").and_then(|q| q.as_str()).ok_or(
            ToolError::InvalidParameters("The documentId is required".to_string()),
        )?;

        let operation = params.get("operation").and_then(|q| q.as_str()).ok_or(
            ToolError::InvalidParameters("The operation is required".to_string()),
        )?;

        match operation {
            "get_document" => {
                // Get the document content
                let result = self
                    .docs
                    .documents()
                    .get(document_id)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Docs get query, {}.",
                        e
                    ))),
                    Ok(r) => {
                        let document = r.1;
                        let title = document.title.unwrap_or_default();

                        // Extract the document content as text
                        let mut content = String::new();
                        content.push_str(&format!("# {}\n\n", title));

                        if let Some(body) = document.body {
                            if let Some(content_items) = body.content {
                                for item in content_items {
                                    if let Some(paragraph) = item.paragraph {
                                        if let Some(elements) = paragraph.elements {
                                            for element in elements {
                                                if let Some(text_run) = element.text_run {
                                                    if let Some(text) = text_run.content {
                                                        content.push_str(&text);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        Ok(vec![Content::text(content).with_priority(0.1)])
                    }
                }
            },
            "insert_text" => {
                let text = params.get("text").and_then(|q| q.as_str()).ok_or(
                    ToolError::InvalidParameters("The text parameter is required for insert_text operation".to_string()),
                )?;

                let position = params.get("position").and_then(|q| q.as_i64()).ok_or(
                    ToolError::InvalidParameters("The position parameter is required for insert_text operation".to_string()),
                )?;

                // Create the insert text request
                let insert_text_request = google_docs1::api::InsertTextRequest {
                    text: Some(text.to_string()),
                    location: Some(google_docs1::api::Location {
                        index: Some(position.try_into().unwrap()),
                        segment_id: None,
                    }),
                    end_of_segment_location: None,
                };

                // Create the batch update request
                let batch_update_request = google_docs1::api::BatchUpdateDocumentRequest {
                    requests: Some(vec![google_docs1::api::Request {
                        insert_text: Some(insert_text_request),
                        ..google_docs1::api::Request::default()
                    }]),
                    write_control: None,
                };

                // Execute the batch update
                let result = self
                    .docs
                    .documents()
                    .batch_update(batch_update_request, document_id)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Docs insert_text operation, {}.",
                        e
                    ))),
                    Ok(_) => {
                        Ok(vec![Content::text(format!(
                            "Successfully inserted text at position {}.",
                            position
                        )).with_priority(0.1)])
                    }
                }
            },
            "append_text" => {
                let text = params.get("text").and_then(|q| q.as_str()).ok_or(
                    ToolError::InvalidParameters("The text parameter is required for append_text operation".to_string()),
                )?;

                // First, get the document to find the end position
                let get_result = self
                    .docs
                    .documents()
                    .get(document_id)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;

                let end_index = match get_result {
                    Err(e) => {
                        return Err(ToolError::ExecutionError(format!(
                            "Failed to get document to determine end position, {}.",
                            e
                        )));
                    },
                    Ok(r) => {
                        let document = r.1;
                        if let Some(body) = document.body {
                            body.content.and_then(|content| {
                                content.last().and_then(|last_item| {
                                    last_item.end_index
                                })
                            }).unwrap_or(1) // Default to 1 if we can't determine the end position
                        } else {
                            1 // Default to 1 if there's no body
                        }
                    }
                };

                // Create the insert text request at the end position
                let insert_text_request = google_docs1::api::InsertTextRequest {
                    text: Some(text.to_string()),
                    location: Some(google_docs1::api::Location {
                        index: Some(end_index - 1), // -1 because end_index is one past the last character
                        segment_id: None,
                    }),
                    end_of_segment_location: None,
                };

                // Create the batch update request
                let batch_update_request = google_docs1::api::BatchUpdateDocumentRequest {
                    requests: Some(vec![google_docs1::api::Request {
                        insert_text: Some(insert_text_request),
                        ..google_docs1::api::Request::default()
                    }]),
                    write_control: None,
                };

                // Execute the batch update
                let result = self
                    .docs
                    .documents()
                    .batch_update(batch_update_request, document_id)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Docs append_text operation, {}.",
                        e
                    ))),
                    Ok(_) => {
                        Ok(vec![Content::text("Successfully appended text to the document.").with_priority(0.1)])
                    }
                }
            },
            "replace_text" => {
                let text = params.get("text").and_then(|q| q.as_str()).ok_or(
                    ToolError::InvalidParameters("The text parameter is required for replace_text operation".to_string()),
                )?;

                let replace_text = params.get("replaceText").and_then(|q| q.as_str()).ok_or(
                    ToolError::InvalidParameters("The replaceText parameter is required for replace_text operation".to_string()),
                )?;

                // Create the replace all text request
                let replace_all_text_request = google_docs1::api::ReplaceAllTextRequest {
                    contains_text: Some(google_docs1::api::SubstringMatchCriteria {
                        text: Some(replace_text.to_string()),
                        match_case: Some(true),
                    }),
                    replace_text: Some(text.to_string()),
                };

                // Create the batch update request
                let batch_update_request = google_docs1::api::BatchUpdateDocumentRequest {
                    requests: Some(vec![google_docs1::api::Request {
                        replace_all_text: Some(replace_all_text_request),
                        ..google_docs1::api::Request::default()
                    }]),
                    write_control: None,
                };

                // Execute the batch update
                let result = self
                    .docs
                    .documents()
                    .batch_update(batch_update_request, document_id)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Docs replace_text operation, {}.",
                        e
                    ))),
                    Ok(r) => {
                        let response = r.1;
                        let replacements = response
                            .replies
                            .and_then(|replies| {
                                replies.first().and_then(|reply| {
                                    reply.replace_all_text.as_ref().map(|replace_response| {
                                        replace_response.occurrences_changed.unwrap_or(0)
                                    })
                                })
                            })
                            .unwrap_or(0);

                        Ok(vec![Content::text(format!(
                            "Successfully replaced {} occurrences of '{}' with '{}'.",
                            replacements, replace_text, text
                        )).with_priority(0.1)])
                    }
                }
            },
            "create_paragraph" => {
                let text = params.get("text").and_then(|q| q.as_str()).ok_or(
                    ToolError::InvalidParameters("The text parameter is required for create_paragraph operation".to_string()),
                )?;

                // Get the end position of the document
                let get_result = self
                    .docs
                    .documents()
                    .get(document_id)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;

                let end_index = match get_result {
                    Err(e) => {
                        return Err(ToolError::ExecutionError(format!(
                            "Failed to get document to determine end position, {}.",
                            e
                        )));
                    },
                    Ok(r) => {
                        let document = r.1;
                        if let Some(body) = document.body {
                            body.content.and_then(|content| {
                                content.last().and_then(|last_item| {
                                    last_item.end_index
                                })
                            }).unwrap_or(1) // Default to 1 if we can't determine the end position
                        } else {
                            1 // Default to 1 if there's no body
                        }
                    }
                };

                // Create the insert text request with a newline at the end
                let insert_text_request = google_docs1::api::InsertTextRequest {
                    text: Some(format!("\n{}", text)),
                    location: Some(google_docs1::api::Location {
                        index: Some(end_index - 1), // -1 because end_index is one past the last character
                        segment_id: None,
                    }),
                    end_of_segment_location: None,
                };

                // Create the batch update request
                let batch_update_request = google_docs1::api::BatchUpdateDocumentRequest {
                    requests: Some(vec![google_docs1::api::Request {
                        insert_text: Some(insert_text_request),
                        ..google_docs1::api::Request::default()
                    }]),
                    write_control: None,
                };

                // Execute the batch update
                let result = self
                    .docs
                    .documents()
                    .batch_update(batch_update_request, document_id)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Docs create_paragraph operation, {}.",
                        e
                    ))),
                    Ok(_) => {
                        Ok(vec![Content::text("Successfully created a new paragraph.").with_priority(0.1)])
                    }
                }
            },
            "delete_content" => {
                let start_position = params.get("startPosition").and_then(|q| q.as_i64()).ok_or(
                    ToolError::InvalidParameters("The startPosition parameter is required for delete_content operation".to_string()),
                )?;

                let end_position = params.get("endPosition").and_then(|q| q.as_i64()).ok_or(
                    ToolError::InvalidParameters("The endPosition parameter is required for delete_content operation".to_string()),
                )?;

                // Create the delete content range request
                let delete_content_range_request = google_docs1::api::DeleteContentRangeRequest {
                    range: Some(google_docs1::api::Range {
                        start_index: Some(start_position.try_into().unwrap()),
                        end_index: Some(end_position.try_into().unwrap()),
                        segment_id: None,
                    }),
                };

                // Create the batch update request
                let batch_update_request = google_docs1::api::BatchUpdateDocumentRequest {
                    requests: Some(vec![google_docs1::api::Request {
                        delete_content_range: Some(delete_content_range_request),
                        ..google_docs1::api::Request::default()
                    }]),
                    write_control: None,
                };

                // Execute the batch update
                let result = self
                    .docs
                    .documents()
                    .batch_update(batch_update_request, document_id)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;

                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to execute Google Docs delete_content operation, {}.",
                        e
                    ))),
                    Ok(_) => {
                        Ok(vec![Content::text(format!(
                            "Successfully deleted content from position {} to {}.",
                            start_position, end_position
                        )).with_priority(0.1)])
                    }
                }
            },
            _ => Err(ToolError::InvalidParameters(format!(
                "Invalid operation: {}. Supported operations are: get_document, insert_text, append_text, replace_text, create_paragraph, delete_content",
                operation
            ))),
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

    fn output_permission(&self, p: Permission) -> String {
        format!(
            "(display_name: {}) (domain: {}) (email_address: {}) (expiration_time: {}) (permission_details: {:?}) (role: {}) (type: {}) (uri: {})",
            p.display_name.unwrap_or_default(),
            p.domain.unwrap_or_default(),
            p.email_address.unwrap_or_default(),
            p.expiration_time.unwrap_or_default(),
            p.permission_details.unwrap_or_default(),
            p.role.unwrap_or_default(),
            p.type_.unwrap_or_default(),
            p.id.unwrap_or_default())
    }

    async fn get_permissions(&self, params: Value) -> Result<Vec<Content>, ToolError> {
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
            let mut builder = self
                .drive
                .permissions()
                .list(file_id)
                .param("fields", "permissions(displayName, domain, emailAddress, expirationTime, permissionDetails, role, type, id)")
                .supports_all_drives(true)
                .page_size(100)
                .clear_scopes() // Scope::MeetReadonly is the default, remove it
                .add_scope(GOOGLE_DRIVE_SCOPES);
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
                        r.1.permissions
                            .map(|perms| perms.into_iter().map(|p| self.output_permission(p)))
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

    async fn sharing(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let file_id =
            params
                .get("fileId")
                .and_then(|q| q.as_str())
                .ok_or(ToolError::InvalidParameters(
                    "The fileId param is required".to_string(),
                ))?;
        let operation = params.get("operation").and_then(|q| q.as_str()).ok_or(
            ToolError::InvalidParameters("The operation is required".to_string()),
        )?;
        let permission_id = params.get("permissionId").and_then(|q| q.as_str());
        let role = params.get("role").and_then(|s| {
            s.as_str().map(|s| {
                if ROLES.contains(&s) {
                    Ok(s)
                } else {
                    Err(ToolError::InvalidParameters("Invalid role: must be one of ('owner', 'organizer', 'fileOrganizer', 'writer', 'commenter', 'reader')".to_string()))
                }
            })
        }).transpose()?;
        let permission_type = params.get("type").and_then(|s|
            s.as_str().map(|s| {
                if PERMISSIONTYPE.contains(&s) {
                    Ok(s)
                } else {
                    Err(ToolError::InvalidParameters("Invalid permission type: must be one of ('user', 'group', 'domain', 'anyone')".to_string()))
                }
            })
        ).transpose()?;
        let target = params.get("target").and_then(|s| s.as_str());
        let email_message = params.get("emailMessage").and_then(|s| s.as_str());

        match operation {
            "create" => {
                let (role, permission_type) = match (role, permission_type) {
                    (Some(r), Some(t)) => (r, t),
                    _ => {
                        return Err(ToolError::InvalidParameters(
                            "The 'create' operation requires the 'role' and 'type' parameters."
                                .to_string(),
                        ))
                    }
                };
                let mut req = Permission {
                    role: Some(role.to_string()),
                    type_: Some(permission_type.to_string()),
                    ..Default::default()
                };
                match (permission_type, target) {
                    ("user", Some(t)) | ("group", Some(t)) => {
                        req.email_address = Some(t.to_string())
                    }
                    ("domain", Some(d)) => req.domain = Some(d.to_string()),
                    ("anyone", None) => {}
                    (_, _) => {
                        return Err(ToolError::InvalidParameters(format!(
                            "The '{}' operation for type '{}' requires the 'target' parameter.",
                            operation, permission_type
                        )))
                    }
                }

                let mut builder = self
                    .drive
                    .permissions()
                    .create(req, file_id)
                    .supports_all_drives(true)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES);
                if let Some(msg) = email_message {
                    builder = builder.email_message(msg);
                }

                let result = builder.doit().await;
                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to manage sharing for google drive file {}, {}.",
                        file_id, e
                    ))),
                    Ok(r) => Ok(vec![Content::text(self.output_permission(r.1))]),
                }
            }
            "update" => {
                let (permission_id, role) = match (permission_id, role) {
                    (Some(p), Some(r)) => (p, r),
                    _ => {
                        return Err(ToolError::InvalidParameters(
                            "The 'update' operation requires the 'permissionId', and 'role'."
                                .to_string(),
                        ))
                    }
                };
                // A permission update requires a permissionId, which is also
                // the ID for that user, group, or domain. We don't _use_ the
                // permission type in the Permission req body, because the
                // update uses "patch semantics", and you can't patch a
                // permission from one user to another without changing its ID.
                let req = Permission {
                    role: Some(role.to_string()),
                    ..Default::default()
                };

                let result = self
                    .drive
                    .permissions()
                    .update(req, file_id, permission_id)
                    .supports_all_drives(true)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;
                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to manage sharing for google drive file {}, {}.",
                        file_id, e
                    ))),
                    Ok(r) => Ok(vec![Content::text(self.output_permission(r.1))]),
                }
            }
            "delete" => {
                let permission_id = permission_id.ok_or(ToolError::InvalidParameters(
                    "The 'delete' operation requires the 'permissionId'.".to_string(),
                ))?;

                let result = self
                    .drive
                    .permissions()
                    .delete(file_id, permission_id)
                    .supports_all_drives(true)
                    .clear_scopes()
                    .add_scope(GOOGLE_DRIVE_SCOPES)
                    .doit()
                    .await;
                match result {
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to manage sharing for google drive file {}, {}.",
                        file_id, e
                    ))),
                    Ok(_) => Ok(vec![Content::text(format!(
                        "Deleted permission: {} from file: {}",
                        file_id, permission_id
                    ))]),
                }
            }
            s => Err(ToolError::InvalidParameters(
                format!(
                    "Parameter 'operation' must be one of ('create', 'update', 'delete'); given {}",
                    s
                )
                .to_string(),
            )),
        }
    }

    async fn list_labels(&self, _params: Value) -> Result<Vec<Content>, ToolError> {
        let builder = self
            .drive_labels
            .labels()
            .list()
            .param("view", "LABEL_VIEW_FULL");

        let result = builder.doit().await;
        match result {
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to list labels for Google Drive {}",
                e
            ))),
            Ok(r) => {
                let content =
                    r.1.labels
                        .map(|labels| {
                            labels.into_iter().map(|l| {
                                format!(
                                    "name: {} label_type: {} properties: {:?} uri: {} fields: {:?}",
                                    l.name.unwrap_or_default(),
                                    l.label_type.unwrap_or_default(),
                                    l.properties.unwrap_or_default(),
                                    l.id.unwrap_or_default(),
                                    l.fields.unwrap_or_default()
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
        _notifier: mpsc::Sender<JsonRpcMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "search" => this.search(arguments).await,
                "read" => this.read(arguments).await,
                "create_file" => this.create_file(arguments).await,
                "move_file" => this.move_file(arguments).await,
                "update_file" => this.update_file(arguments).await,
                "sheets_tool" => this.sheets_tool(arguments).await,
                "docs_tool" => this.docs_tool(arguments).await,
                "manage_comment" => this.manage_comment(arguments).await,
                "get_comments" => this.get_comments(arguments).await,
                "list_drives" => this.list_drives(arguments).await,
                "get_permissions" => this.get_permissions(arguments).await,
                "sharing" => this.sharing(arguments).await,
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
            drive_labels: self.drive_labels.clone(),
            sheets: self.sheets.clone(),
            docs: self.docs.clone(),
            credentials_manager: self.credentials_manager.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_url() {
        let url = "https://docs.google.com/document/d/1QG8d8wtWe7ZfmG93sW-1h2WXDJDUkOi-9hDnvJLmWrc/edit?tab=t.0#heading=h.5v419d3h97tr";
        assert_eq!(
            extract_google_drive_id(url),
            Some("1QG8d8wtWe7ZfmG93sW-1h2WXDJDUkOi-9hDnvJLmWrc")
        );
    }

    #[test]
    fn test_spreadsheets_url() {
        let url = "https://docs.google.com/spreadsheets/d/1J5KHqWsGFzweuiQboX7dlm8Ejv90Po16ocEBahzCt4W/edit?gid=1249300797#gid=1249300797";
        assert_eq!(
            extract_google_drive_id(url),
            Some("1J5KHqWsGFzweuiQboX7dlm8Ejv90Po16ocEBahzCt4W")
        );
    }

    #[test]
    fn test_slides_url() {
        let url = "https://docs.google.com/presentation/d/1zXWqsGpHJEu40oqb1omh68sW9liu7EKFBCdnPaJVoQ5et/edit#slide=id.p1";
        assert_eq!(
            extract_google_drive_id(url),
            Some("1zXWqsGpHJEu40oqb1omh68sW9liu7EKFBCdnPaJVoQ5et")
        );
    }

    #[test]
    fn test_missing_scheme() {
        let url = "docs.google.com/document/d/abcdef12345/edit";
        assert_eq!(extract_google_drive_id(url), None);
    }

    #[test]
    fn test_extra_path_segments() {
        let url = "https://drive.google.com/file/d/1abcdEFGH_ijklMNOpqrstUVwxyz-1234/view";
        assert_eq!(
            extract_google_drive_id(url),
            Some("1abcdEFGH_ijklMNOpqrstUVwxyz-1234")
        );
    }

    #[test]
    fn test_invalid_google_url() {
        let url = "https://example.com/d/12345";
        assert_eq!(extract_google_drive_id(url), None);
    }

    #[test]
    fn test_no_d_segment() {
        let url =
            "https://docs.google.com/document/1QG8d8wtWe7ZfmG93sW-1h2WXDJDUkOi-9hDnvJLmWrc/edit";
        assert_eq!(extract_google_drive_id(url), None);
    }
}
