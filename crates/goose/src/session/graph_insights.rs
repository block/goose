//! Project-centric graph insights for 3D visualization
//!
//! Detects git repositories and normalizes directories to projects,
//! then clusters sessions around their projects for visualization.

use anyhow::Result;
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub val: f64,
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<NodeMetadata>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeMetadata {
    pub session_count: Option<i64>,
    pub message_count: Option<i64>,
    pub token_count: Option<i64>,
    pub first_activity: Option<String>,
    pub last_activity: Option<String>,
    pub directories: Option<Vec<String>>,
    pub project_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryStats {
    pub directory: String,
    pub session_count: i64,
    pub message_count: i64,
    pub total_tokens: i64,
    pub first_activity: String,
    pub last_activity: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStats {
    pub provider: String,
    pub session_count: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionTypeStats {
    pub session_type: String,
    pub count: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DailyActivity {
    pub date: String,
    pub session_count: i64,
    pub message_count: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GraphInsights {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
    pub summary: InsightsSummary,
    pub directories: Vec<DirectoryStats>,
    pub providers: Vec<ProviderStats>,
    pub session_types: Vec<SessionTypeStats>,
    pub daily_activity: Vec<DailyActivity>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InsightsSummary {
    pub total_sessions: i64,
    pub total_messages: i64,
    pub total_tokens: i64,
    pub unique_directories: i64,
    pub unique_projects: i64,
    pub date_range_start: Option<String>,
    pub date_range_end: Option<String>,
}

#[derive(Debug, Clone)]
struct ProjectInfo {
    name: String,
    project_type: ProjectType,
    directories: Vec<String>,
    session_count: i64,
    message_count: i64,
    token_count: i64,
    first_activity: Option<String>,
    last_activity: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum ProjectType {
    GitRepo,
    Directory,
}

fn get_git_project_name(dir: &str) -> Option<String> {
    if !Path::new(dir).exists() {
        return None;
    }

    // Check if it's a git repo
    let git_dir_check = Command::new("git")
        .args(["-C", dir, "rev-parse", "--git-dir"])
        .output()
        .ok()?;

    if !git_dir_check.status.success() {
        return None;
    }

    // Try to get remote origin URL (most reliable for identifying the project)
    let remote_output = Command::new("git")
        .args(["-C", dir, "config", "--get", "remote.origin.url"])
        .output()
        .ok()?;

    if remote_output.status.success() {
        let url = String::from_utf8_lossy(&remote_output.stdout)
            .trim()
            .to_string();
        return Some(extract_repo_name_from_url(&url));
    }

    // Fallback: get the repo root directory name
    let root_output = Command::new("git")
        .args(["-C", dir, "rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if root_output.status.success() {
        let root = String::from_utf8_lossy(&root_output.stdout)
            .trim()
            .to_string();
        return root.split('/').last().map(|s| s.to_string());
    }

    None
}

fn extract_repo_name_from_url(url: &str) -> String {
    // Handle various URL formats:
    // git@github.com:user/repo.git
    // https://github.com/user/repo.git
    // git@github.com:user/repo
    // https://github.com/user/repo

    let url = url.trim_end_matches(".git");

    // SSH format: git@host:user/repo
    if let Some(path) = url.strip_prefix("git@") {
        if let Some((_host, path)) = path.split_once(':') {
            return path.to_string();
        }
    }

    // HTTPS format: https://host/user/repo
    if url.starts_with("https://") || url.starts_with("http://") {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 2 {
            // Return user/repo
            return parts[parts.len() - 2..].join("/");
        }
    }

    // Fallback: just use the last component
    url.split('/').last().unwrap_or(url).to_string()
}

fn get_directory_project_name(dir: &str) -> String {
    // For non-git directories, use the last path component
    // But handle special cases like /tmp
    let path = Path::new(dir);

    if dir == "/tmp" || dir == "/var/tmp" {
        return "tmp".to_string();
    }

    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn get_project_color(project_type: &ProjectType, index: usize) -> String {
    match project_type {
        ProjectType::GitRepo => {
            // Blue/purple palette for git repos
            let colors = [
                "#6366F1", "#8B5CF6", "#A855F7", "#3B82F6", "#0EA5E9", "#06B6D4",
            ];
            colors[index % colors.len()].to_string()
        }
        ProjectType::Directory => {
            // Green/teal palette for plain directories
            let colors = ["#10B981", "#14B8A6", "#22C55E", "#84CC16"];
            colors[index % colors.len()].to_string()
        }
    }
}

/// Query and build project-centric graph insights
pub async fn build_graph_insights(pool: &Pool<Sqlite>) -> Result<GraphInsights> {
    // Get all directories with their stats
    let directory_rows = sqlx::query_as::<_, (String, i64, i64, Option<i64>, String, String)>(
        r#"SELECT s.working_dir, COUNT(DISTINCT s.id), COUNT(m.id), SUM(s.total_tokens),
           MIN(date(s.created_at)), MAX(date(s.updated_at))
           FROM sessions s LEFT JOIN messages m ON s.id = m.session_id
           GROUP BY s.working_dir ORDER BY COUNT(m.id) DESC"#,
    )
    .fetch_all(pool)
    .await?;

    // Build project map: normalize directories to projects
    let mut project_map: HashMap<String, ProjectInfo> = HashMap::new();

    for row in &directory_rows {
        let dir = &row.0;
        let session_count = row.1;
        let message_count = row.2;
        let token_count = row.3.unwrap_or(0);
        let first_activity = row.4.clone();
        let last_activity = row.5.clone();

        // Determine project name and type
        let (project_name, project_type) = if let Some(git_name) = get_git_project_name(dir) {
            (git_name, ProjectType::GitRepo)
        } else {
            (get_directory_project_name(dir), ProjectType::Directory)
        };

        // Merge into existing project or create new
        project_map
            .entry(project_name.clone())
            .and_modify(|p| {
                p.directories.push(dir.clone());
                p.session_count += session_count;
                p.message_count += message_count;
                p.token_count += token_count;
                // Update date range
                if let Some(ref existing) = p.first_activity {
                    if first_activity < *existing {
                        p.first_activity = Some(first_activity.clone());
                    }
                }
                if let Some(ref existing) = p.last_activity {
                    if last_activity > *existing {
                        p.last_activity = Some(last_activity.clone());
                    }
                }
            })
            .or_insert(ProjectInfo {
                name: project_name,
                project_type,
                directories: vec![dir.clone()],
                session_count,
                message_count,
                token_count,
                first_activity: Some(first_activity),
                last_activity: Some(last_activity),
            });
    }

    // Sort projects by message count
    let mut projects: Vec<ProjectInfo> = project_map.into_values().collect();
    projects.sort_by(|a, b| b.message_count.cmp(&a.message_count));

    // Limit to top 30 projects for visualization clarity
    let projects: Vec<ProjectInfo> = projects.into_iter().take(30).collect();

    // Build nodes and links
    let mut nodes = Vec::new();
    let mut links = Vec::new();

    // Create project nodes
    for (i, project) in projects.iter().enumerate() {
        let project_type_str = match project.project_type {
            ProjectType::GitRepo => "git",
            ProjectType::Directory => "dir",
        };

        nodes.push(GraphNode {
            id: format!("project_{}", i),
            name: project.name.clone(),
            node_type: "project".to_string(),
            val: (project.message_count as f64).sqrt() * 3.0,
            color: Some(get_project_color(&project.project_type, i)),
            metadata: Some(NodeMetadata {
                session_count: Some(project.session_count),
                message_count: Some(project.message_count),
                token_count: Some(project.token_count),
                first_activity: project.first_activity.clone(),
                last_activity: project.last_activity.clone(),
                directories: Some(project.directories.clone()),
                project_type: Some(project_type_str.to_string()),
            }),
        });
    }

    // Get recent sessions for visualization (last 100 sessions)
    let session_rows = sqlx::query_as::<_, (String, String, i64, String)>(
        r#"SELECT s.id, s.working_dir, COUNT(m.id), date(s.created_at)
           FROM sessions s LEFT JOIN messages m ON s.id = m.session_id
           WHERE s.created_at >= date('now', '-30 days')
           GROUP BY s.id
           ORDER BY s.created_at DESC
           LIMIT 100"#,
    )
    .fetch_all(pool)
    .await?;

    // Create session nodes and link to projects
    for (sess_idx, sess_row) in session_rows.iter().enumerate() {
        let session_id = &sess_row.0;
        let working_dir = &sess_row.1;
        let msg_count = sess_row.2;
        let created_date = &sess_row.3;

        // Find the project for this session
        let project_idx = projects.iter().position(|p| p.directories.contains(working_dir));

        if let Some(proj_idx) = project_idx {
            // Create session node
            let node_id = format!("session_{}", sess_idx);
            nodes.push(GraphNode {
                id: node_id.clone(),
                name: format!("Session {}", &session_id[..8.min(session_id.len())]),
                node_type: "session".to_string(),
                val: (msg_count as f64).sqrt().max(1.0),
                color: Some("#9CA3AF".to_string()), // Gray for sessions
                metadata: Some(NodeMetadata {
                    session_count: None,
                    message_count: Some(msg_count),
                    token_count: None,
                    first_activity: Some(created_date.clone()),
                    last_activity: None,
                    directories: None,
                    project_type: None,
                }),
            });

            // Link session to project
            links.push(GraphLink {
                source: node_id,
                target: format!("project_{}", proj_idx),
                value: 1.0,
            });
        }
    }

    // Summary statistics
    let summary_row =
        sqlx::query_as::<_, (i64, i64, Option<i64>, i64, Option<String>, Option<String>)>(
            r#"SELECT COUNT(DISTINCT s.id), COUNT(m.id), SUM(s.total_tokens),
           COUNT(DISTINCT s.working_dir), MIN(date(s.created_at)), MAX(date(s.updated_at))
           FROM sessions s LEFT JOIN messages m ON s.id = m.session_id"#,
        )
        .fetch_one(pool)
        .await?;

    let summary = InsightsSummary {
        total_sessions: summary_row.0,
        total_messages: summary_row.1,
        total_tokens: summary_row.2.unwrap_or(0),
        unique_directories: summary_row.3,
        unique_projects: projects.len() as i64,
        date_range_start: summary_row.4,
        date_range_end: summary_row.5,
    };

    // Directory stats (for backward compat)
    let directories: Vec<DirectoryStats> = directory_rows
        .iter()
        .take(50)
        .map(|r| DirectoryStats {
            directory: r.0.clone(),
            session_count: r.1,
            message_count: r.2,
            total_tokens: r.3.unwrap_or(0),
            first_activity: r.4.clone(),
            last_activity: r.5.clone(),
        })
        .collect();

    // Provider statistics
    let provider_rows = sqlx::query_as::<_, (String, i64)>(
        r#"SELECT COALESCE(provider_name, 'unknown'), COUNT(*) FROM sessions
           GROUP BY provider_name ORDER BY COUNT(*) DESC"#,
    )
    .fetch_all(pool)
    .await?;

    let providers: Vec<ProviderStats> = provider_rows
        .iter()
        .map(|r| ProviderStats {
            provider: r.0.clone(),
            session_count: r.1,
        })
        .collect();

    // Session type statistics
    let session_type_rows = sqlx::query_as::<_, (String, i64, Option<i64>)>(
        r#"SELECT session_type, COUNT(*), SUM(total_tokens) FROM sessions
           GROUP BY session_type ORDER BY COUNT(*) DESC"#,
    )
    .fetch_all(pool)
    .await?;

    let session_types: Vec<SessionTypeStats> = session_type_rows
        .iter()
        .map(|r| SessionTypeStats {
            session_type: r.0.clone(),
            count: r.1,
            total_tokens: r.2.unwrap_or(0),
        })
        .collect();

    // Daily activity
    let daily_rows = sqlx::query_as::<_, (String, i64, i64)>(
        r#"SELECT date(s.updated_at), COUNT(DISTINCT s.id), COUNT(m.id)
           FROM sessions s LEFT JOIN messages m ON s.id = m.session_id
           WHERE s.updated_at >= date('now', '-30 days')
           GROUP BY date(s.updated_at) ORDER BY date(s.updated_at) DESC"#,
    )
    .fetch_all(pool)
    .await?;

    let daily_activity: Vec<DailyActivity> = daily_rows
        .iter()
        .map(|r| DailyActivity {
            date: r.0.clone(),
            session_count: r.1,
            message_count: r.2,
        })
        .collect();

    Ok(GraphInsights {
        nodes,
        links,
        summary,
        directories,
        providers,
        session_types,
        daily_activity,
    })
}
