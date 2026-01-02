//! Project-centric graph insights for 3D visualization
//!
//! Detects git repositories and normalizes directories to projects,
//! then clusters sessions around their projects for visualization.
//! Adds cross-project links for temporal proximity and similar sessions.

use anyhow::Result;
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use std::collections::{HashMap, HashSet};
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
    pub session_name: Option<String>,
    pub git_dirty: Option<bool>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,
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
    active_dates: HashSet<String>,
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

    let git_dir_check = Command::new("git")
        .args(["-C", dir, "rev-parse", "--git-dir"])
        .output()
        .ok()?;

    if !git_dir_check.status.success() {
        return None;
    }

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
    let url = url.trim_end_matches(".git");

    if let Some(path) = url.strip_prefix("git@") {
        if let Some((_host, path)) = path.split_once(':') {
            return path.to_string();
        }
    }

    if url.starts_with("https://") || url.starts_with("http://") {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 2 {
            return parts[parts.len() - 2..].join("/");
        }
    }

    url.split('/').last().unwrap_or(url).to_string()
}

fn get_directory_project_name(dir: &str) -> String {
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
            let colors = [
                "#6366F1", "#8B5CF6", "#A855F7", "#3B82F6", "#0EA5E9", "#06B6D4",
            ];
            colors[index % colors.len()].to_string()
        }
        ProjectType::Directory => {
            let colors = ["#10B981", "#14B8A6", "#22C55E", "#84CC16"];
            colors[index % colors.len()].to_string()
        }
    }
}

/// Check if a git repo has uncommitted changes (dirty)
fn is_git_dirty(dir: &str) -> Option<bool> {
    if !Path::new(dir).exists() {
        return None;
    }

    let output = Command::new("git")
        .args(["-C", dir, "status", "--porcelain"])
        .output()
        .ok()?;

    if output.status.success() {
        let status = String::from_utf8_lossy(&output.stdout);
        Some(!status.trim().is_empty())
    } else {
        None
    }
}

/// Extract keywords from session name for similarity matching
fn extract_keywords(name: &str) -> HashSet<String> {
    let stopwords: HashSet<&str> = [
        "session", "new", "the", "a", "an", "and", "or", "for", "to", "in", "on", "at", "of",
        "goose", "term", "cli", "1", "2", "3", "4", "5",
    ]
    .into_iter()
    .collect();

    name.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2 && !stopwords.contains(w))
        .map(|s| s.to_string())
        .collect()
}

/// Check if two session names are similar (share meaningful keywords)
fn sessions_are_similar(name1: &str, name2: &str) -> bool {
    let kw1 = extract_keywords(name1);
    let kw2 = extract_keywords(name2);

    if kw1.is_empty() || kw2.is_empty() {
        return false;
    }

    let intersection: HashSet<_> = kw1.intersection(&kw2).collect();
    !intersection.is_empty()
}

/// Query and build project-centric graph insights
pub async fn build_graph_insights(pool: &Pool<Sqlite>) -> Result<GraphInsights> {
    // Get all directories with their stats and active dates
    let directory_rows = sqlx::query_as::<_, (String, i64, i64, Option<i64>, String, String)>(
        r#"SELECT s.working_dir, COUNT(DISTINCT s.id), COUNT(m.id), SUM(s.total_tokens),
           MIN(date(s.created_at)), MAX(date(s.updated_at))
           FROM sessions s LEFT JOIN messages m ON s.id = m.session_id
           GROUP BY s.working_dir ORDER BY COUNT(m.id) DESC"#,
    )
    .fetch_all(pool)
    .await?;

    // Get active dates per directory (for temporal links)
    let date_rows = sqlx::query_as::<_, (String, String)>(
        r#"SELECT working_dir, date(created_at) FROM sessions
           WHERE created_at >= date('now', '-30 days')
           GROUP BY working_dir, date(created_at)"#,
    )
    .fetch_all(pool)
    .await?;

    let mut dir_dates: HashMap<String, HashSet<String>> = HashMap::new();
    for (dir, date) in date_rows {
        dir_dates.entry(dir).or_default().insert(date);
    }

    // Build project map
    let mut project_map: HashMap<String, ProjectInfo> = HashMap::new();

    for row in &directory_rows {
        let dir = &row.0;
        let session_count = row.1;
        let message_count = row.2;
        let token_count = row.3.unwrap_or(0);
        let first_activity = row.4.clone();
        let last_activity = row.5.clone();
        let dates = dir_dates.get(dir).cloned().unwrap_or_default();

        let (project_name, project_type) = if let Some(git_name) = get_git_project_name(dir) {
            (git_name, ProjectType::GitRepo)
        } else {
            (get_directory_project_name(dir), ProjectType::Directory)
        };

        project_map
            .entry(project_name.clone())
            .and_modify(|p| {
                p.directories.push(dir.clone());
                p.session_count += session_count;
                p.message_count += message_count;
                p.token_count += token_count;
                p.active_dates.extend(dates.clone());
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
                active_dates: dates,
            });
    }

    let mut projects: Vec<ProjectInfo> = project_map.into_values().collect();
    projects.sort_by(|a, b| b.message_count.cmp(&a.message_count));
    let projects: Vec<ProjectInfo> = projects.into_iter().take(30).collect();

    let mut nodes = Vec::new();
    let mut links = Vec::new();

    // Create project nodes
    for (i, project) in projects.iter().enumerate() {
        let project_type_str = match project.project_type {
            ProjectType::GitRepo => "git",
            ProjectType::Directory => "dir",
        };

        // Check git dirty status for git repos (check first existing directory)
        let git_dirty = if project.project_type == ProjectType::GitRepo {
            project
                .directories
                .iter()
                .find_map(|d| is_git_dirty(d))
        } else {
            None
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
                session_name: None,
                git_dirty,
            }),
        });
    }

    // Add temporal links between projects (worked on same day)
    let mut temporal_links_added: HashSet<(usize, usize)> = HashSet::new();
    for (i, proj_a) in projects.iter().enumerate() {
        for (j, proj_b) in projects.iter().enumerate().skip(i + 1) {
            let shared_dates: HashSet<_> = proj_a
                .active_dates
                .intersection(&proj_b.active_dates)
                .collect();
            if !shared_dates.is_empty() && !temporal_links_added.contains(&(i, j)) {
                links.push(GraphLink {
                    source: format!("project_{}", i),
                    target: format!("project_{}", j),
                    value: (shared_dates.len() as f64).sqrt() * 0.5,
                    link_type: Some("temporal".to_string()),
                });
                temporal_links_added.insert((i, j));
            }
        }
    }

    // Get recent sessions with names for visualization
    let session_rows = sqlx::query_as::<_, (String, String, String, i64, String)>(
        r#"SELECT s.id, s.working_dir, COALESCE(NULLIF(s.name, ''), s.description, ''),
           COUNT(m.id), date(s.created_at)
           FROM sessions s LEFT JOIN messages m ON s.id = m.session_id
           WHERE s.created_at >= date('now', '-30 days')
           GROUP BY s.id
           ORDER BY s.created_at DESC
           LIMIT 100"#,
    )
    .fetch_all(pool)
    .await?;

    // Track session info for cross-project similarity links
    struct SessionInfo {
        node_id: String,
        name: String,
        project_idx: usize,
    }
    let mut session_infos: Vec<SessionInfo> = Vec::new();

    // Create session nodes and link to projects
    for (sess_idx, sess_row) in session_rows.iter().enumerate() {
        let session_id = &sess_row.0;
        let working_dir = &sess_row.1;
        let session_name = &sess_row.2;
        let msg_count = sess_row.3;
        let created_date = &sess_row.4;

        let project_idx = projects
            .iter()
            .position(|p| p.directories.contains(working_dir));

        if let Some(proj_idx) = project_idx {
            let node_id = format!("session_{}", sess_idx);
            let display_name = if session_name.is_empty() {
                format!("Session {}", &session_id[..8.min(session_id.len())])
            } else {
                session_name.chars().take(30).collect()
            };

            nodes.push(GraphNode {
                id: node_id.clone(),
                name: display_name.clone(),
                node_type: "session".to_string(),
                val: (msg_count as f64).sqrt().max(1.0),
                color: Some("#9CA3AF".to_string()),
                metadata: Some(NodeMetadata {
                    session_count: None,
                    message_count: Some(msg_count),
                    token_count: None,
                    first_activity: Some(created_date.clone()),
                    last_activity: None,
                    directories: None,
                    project_type: None,
                    session_name: Some(session_name.clone()),
                    git_dirty: None,
                }),
            });

            // Link session to project
            links.push(GraphLink {
                source: node_id.clone(),
                target: format!("project_{}", proj_idx),
                value: 1.0,
                link_type: None,
            });

            // Track for similarity matching
            if !session_name.is_empty() {
                session_infos.push(SessionInfo {
                    node_id,
                    name: session_name.clone(),
                    project_idx: proj_idx,
                });
            }
        }
    }

    // Add cross-project links for similar sessions
    let mut similarity_links_added: HashSet<(String, String)> = HashSet::new();
    for (i, sess_a) in session_infos.iter().enumerate() {
        for sess_b in session_infos.iter().skip(i + 1) {
            // Only link sessions from DIFFERENT projects
            if sess_a.project_idx != sess_b.project_idx
                && sessions_are_similar(&sess_a.name, &sess_b.name)
            {
                let key = if sess_a.node_id < sess_b.node_id {
                    (sess_a.node_id.clone(), sess_b.node_id.clone())
                } else {
                    (sess_b.node_id.clone(), sess_a.node_id.clone())
                };

                if !similarity_links_added.contains(&key) {
                    links.push(GraphLink {
                        source: sess_a.node_id.clone(),
                        target: sess_b.node_id.clone(),
                        value: 0.3,
                        link_type: Some("similar".to_string()),
                    });
                    similarity_links_added.insert(key);
                }
            }
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
