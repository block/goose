//! Graph-based session insights for 3D visualization
//!
//! Provides data structures and queries to generate force-graph compatible
//! data for visualizing session activity across directories, providers, and time.

use anyhow::Result;
use serde::Serialize;
use sqlx::{Pool, Sqlite};
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
    pub date_range_start: Option<String>,
    pub date_range_end: Option<String>,
}

fn get_node_color(node_type: &str, index: usize) -> String {
    match node_type {
        "directory" => {
            let colors = ["#4CAF50", "#8BC34A", "#CDDC39", "#009688", "#00BCD4", "#03A9F4"];
            colors[index % colors.len()].to_string()
        }
        "provider" => {
            let colors = ["#FF5722", "#FF9800", "#FFC107", "#E91E63", "#9C27B0"];
            colors[index % colors.len()].to_string()
        }
        "session_type" => {
            let colors = ["#2196F3", "#3F51B5", "#673AB7", "#607D8B"];
            colors[index % colors.len()].to_string()
        }
        _ => "#757575".to_string(),
    }
}

/// Query and build graph insights - called internally with pool reference
pub async fn build_graph_insights(pool: &Pool<Sqlite>) -> Result<GraphInsights> {
    let mut nodes = Vec::new();
    let mut links = Vec::new();

    // Summary statistics
    let summary_row = sqlx::query_as::<_, (i64, i64, Option<i64>, i64, Option<String>, Option<String>)>(
        r#"SELECT COUNT(DISTINCT s.id), COUNT(m.id), SUM(s.total_tokens),
           COUNT(DISTINCT s.working_dir), MIN(date(s.created_at)), MAX(date(s.updated_at))
           FROM sessions s LEFT JOIN messages m ON s.id = m.session_id"#,
    ).fetch_one(pool).await?;

    let summary = InsightsSummary {
        total_sessions: summary_row.0,
        total_messages: summary_row.1,
        total_tokens: summary_row.2.unwrap_or(0),
        unique_directories: summary_row.3,
        date_range_start: summary_row.4,
        date_range_end: summary_row.5,
    };

    // Directory statistics
    let directory_rows = sqlx::query_as::<_, (String, i64, i64, Option<i64>, String, String)>(
        r#"SELECT s.working_dir, COUNT(DISTINCT s.id), COUNT(m.id), SUM(s.total_tokens),
           MIN(date(s.created_at)), MAX(date(s.updated_at))
           FROM sessions s LEFT JOIN messages m ON s.id = m.session_id
           GROUP BY s.working_dir ORDER BY COUNT(m.id) DESC LIMIT 50"#,
    ).fetch_all(pool).await?;

    let directories: Vec<DirectoryStats> = directory_rows.iter().map(|r| DirectoryStats {
        directory: r.0.clone(), session_count: r.1, message_count: r.2,
        total_tokens: r.3.unwrap_or(0), first_activity: r.4.clone(), last_activity: r.5.clone(),
    }).collect();

    for (i, dir) in directories.iter().enumerate() {
        let short_name = dir.directory.split('/').last().unwrap_or(&dir.directory).to_string();
        nodes.push(GraphNode {
            id: format!("dir_{}", i), name: short_name, node_type: "directory".to_string(),
            val: (dir.message_count as f64).sqrt() * 2.0, color: Some(get_node_color("directory", i)),
            metadata: Some(NodeMetadata {
                session_count: Some(dir.session_count), message_count: Some(dir.message_count),
                token_count: Some(dir.total_tokens), first_activity: Some(dir.first_activity.clone()),
                last_activity: Some(dir.last_activity.clone()),
            }),
        });
    }

    // Provider statistics
    let provider_rows = sqlx::query_as::<_, (String, i64)>(
        r#"SELECT COALESCE(provider_name, 'unknown'), COUNT(*) FROM sessions
           GROUP BY provider_name ORDER BY COUNT(*) DESC"#,
    ).fetch_all(pool).await?;

    let providers: Vec<ProviderStats> = provider_rows.iter()
        .map(|r| ProviderStats { provider: r.0.clone(), session_count: r.1 }).collect();

    for (i, provider) in providers.iter().enumerate() {
        nodes.push(GraphNode {
            id: format!("provider_{}", provider.provider), name: provider.provider.clone(),
            node_type: "provider".to_string(), val: (provider.session_count as f64).sqrt() * 3.0,
            color: Some(get_node_color("provider", i)),
            metadata: Some(NodeMetadata {
                session_count: Some(provider.session_count), message_count: None,
                token_count: None, first_activity: None, last_activity: None,
            }),
        });
    }

    // Session type statistics
    let session_type_rows = sqlx::query_as::<_, (String, i64, Option<i64>)>(
        r#"SELECT session_type, COUNT(*), SUM(total_tokens) FROM sessions
           GROUP BY session_type ORDER BY COUNT(*) DESC"#,
    ).fetch_all(pool).await?;

    let session_types: Vec<SessionTypeStats> = session_type_rows.iter()
        .map(|r| SessionTypeStats { session_type: r.0.clone(), count: r.1, total_tokens: r.2.unwrap_or(0) }).collect();

    for (i, st) in session_types.iter().enumerate() {
        nodes.push(GraphNode {
            id: format!("type_{}", st.session_type), name: st.session_type.clone(),
            node_type: "session_type".to_string(), val: (st.count as f64).sqrt() * 2.5,
            color: Some(get_node_color("session_type", i)),
            metadata: Some(NodeMetadata {
                session_count: Some(st.count), message_count: None,
                token_count: Some(st.total_tokens), first_activity: None, last_activity: None,
            }),
        });
    }

    // Daily activity
    let daily_rows = sqlx::query_as::<_, (String, i64, i64)>(
        r#"SELECT date(s.updated_at), COUNT(DISTINCT s.id), COUNT(m.id)
           FROM sessions s LEFT JOIN messages m ON s.id = m.session_id
           WHERE s.updated_at >= date('now', '-30 days')
           GROUP BY date(s.updated_at) ORDER BY date(s.updated_at) DESC"#,
    ).fetch_all(pool).await?;

    let daily_activity: Vec<DailyActivity> = daily_rows.iter()
        .map(|r| DailyActivity { date: r.0.clone(), session_count: r.1, message_count: r.2 }).collect();

    // Links between directories and providers
    let dir_provider_links = sqlx::query_as::<_, (String, String, i64)>(
        r#"SELECT working_dir, COALESCE(provider_name, 'unknown'), COUNT(*)
           FROM sessions GROUP BY working_dir, provider_name
           HAVING COUNT(*) > 2 ORDER BY COUNT(*) DESC LIMIT 100"#,
    ).fetch_all(pool).await?;

    for (dir_idx, dir) in directories.iter().enumerate() {
        for link_row in &dir_provider_links {
            if link_row.0 == dir.directory {
                links.push(GraphLink {
                    source: format!("dir_{}", dir_idx),
                    target: format!("provider_{}", link_row.1),
                    value: (link_row.2 as f64).sqrt(),
                });
            }
        }
    }

    // Central hub
    nodes.push(GraphNode {
        id: "hub".to_string(), name: "Sessions".to_string(), node_type: "hub".to_string(),
        val: 10.0, color: Some("#FF6B35".to_string()), metadata: None,
    });

    for st in &session_types {
        links.push(GraphLink {
            source: "hub".to_string(), target: format!("type_{}", st.session_type),
            value: (st.count as f64).sqrt() * 0.5,
        });
    }

    Ok(GraphInsights { nodes, links, summary, directories, providers, session_types, daily_activity })
}
