use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::Pool;
use sqlx::Sqlite;
use utoipa::ToSchema;

/// Analytics for tool usage extracted from stored messages
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToolAnalytics {
    pub total_tool_calls: i64,
    pub total_tool_errors: i64,
    pub success_rate: f64,
    pub tool_usage: Vec<ToolUsageStat>,
    pub daily_tool_activity: Vec<DailyToolActivity>,
    pub extension_usage: Vec<ExtensionUsageStat>,
    pub session_tool_summary: Vec<SessionToolSummary>,
}

/// Per-tool usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToolUsageStat {
    pub tool_name: String,
    pub extension: String,
    pub call_count: i64,
    pub error_count: i64,
    pub success_rate: f64,
}

/// Daily tool activity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DailyToolActivity {
    pub date: String,
    pub tool_calls: i64,
    pub tool_errors: i64,
    pub unique_tools: i64,
}

/// Per-extension usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExtensionUsageStat {
    pub extension: String,
    pub tool_count: i64,
    pub total_calls: i64,
    pub total_errors: i64,
    pub success_rate: f64,
}

/// Tool summary for a specific session
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionToolSummary {
    pub session_id: String,
    pub session_name: String,
    pub tool_calls: i64,
    pub tool_errors: i64,
    pub unique_tools: i64,
    pub most_used_tool: String,
    pub created_at: String,
}

/// Agent performance metrics extracted from session data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentPerformanceMetrics {
    pub sessions_by_provider: Vec<ProviderSessionStat>,
    pub avg_messages_per_session: f64,
    pub avg_tools_per_session: f64,
    pub avg_tokens_per_session: f64,
    pub session_duration_stats: DurationStats,
    pub active_extensions: Vec<ActiveExtensionStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderSessionStat {
    pub provider: String,
    pub session_count: i64,
    pub avg_tokens: f64,
    pub avg_messages: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DurationStats {
    pub avg_seconds: f64,
    pub median_seconds: f64,
    pub p90_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActiveExtensionStat {
    pub extension: String,
    pub session_count: i64,
}

/// Version info for correlation tracking
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VersionInfo {
    pub goose_version: String,
    pub active_extensions: Vec<String>,
}

pub struct ToolAnalyticsStore<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> ToolAnalyticsStore<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Extract tool call analytics from stored messages using SQLite JSON functions.
    ///
    /// Messages are stored with content_json containing arrays of MessageContent variants.
    /// ToolRequest items have: {"type":"toolRequest","id":"...","toolCall":{"Ok":{"name":"...","arguments":{...}}}}
    /// ToolResponse items have: {"type":"toolResponse","id":"...","toolResult":{"Ok":{"content":[...],"isError":false}}}
    pub async fn get_tool_analytics(&self, days: i32) -> Result<ToolAnalytics> {
        // Extract tool calls from content_json using json_each to iterate array elements
        // ToolRequest has type="toolRequest" and toolCall.Ok.name for the tool name
        // ToolResponse has type="toolResponse" and toolResult.Ok.isError for error status

        let tool_usage = self.get_tool_usage_stats(days).await?;
        let daily_activity = self.get_daily_tool_activity(days).await?;
        let extension_usage = self.get_extension_usage(days).await?;
        let session_summaries = self.get_session_tool_summaries(days).await?;

        let total_calls: i64 = tool_usage.iter().map(|t| t.call_count).sum();
        let total_errors: i64 = tool_usage.iter().map(|t| t.error_count).sum();
        let success_rate = if total_calls > 0 {
            ((total_calls - total_errors) as f64 / total_calls as f64) * 100.0
        } else {
            100.0
        };

        Ok(ToolAnalytics {
            total_tool_calls: total_calls,
            total_tool_errors: total_errors,
            success_rate,
            tool_usage,
            daily_tool_activity: daily_activity,
            extension_usage,
            session_tool_summary: session_summaries,
        })
    }

    /// Get per-tool usage stats by parsing content_json
    async fn get_tool_usage_stats(&self, days: i32) -> Result<Vec<ToolUsageStat>> {
        // Query: Extract tool names from ToolRequest messages and match with ToolResponse for error status
        // Tool names follow the pattern "extension__tool_name" (double underscore separator)
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT
                json_extract(je.value, '$.toolCall.value.name') as tool_name,
                COUNT(*) as call_count
            FROM messages m, json_each(m.content_json) je
            WHERE m.role = 'assistant'
              AND json_extract(je.value, '$.type') = 'toolRequest'
              AND json_extract(je.value, '$.toolCall.value.name') IS NOT NULL
              AND m.created_timestamp > unixepoch() - (? * 86400)
            GROUP BY tool_name
            ORDER BY call_count DESC
            "#,
        )
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        // Get error counts from ToolResponse messages
        let error_rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT
                json_extract(je.value, '$.id') as response_id,
                CASE
                    WHEN json_extract(je.value, '$.toolResult.value.isError') = 1 THEN 1
                    WHEN json_extract(je.value, '$.toolResult.value.isError') = true THEN 1
                    WHEN json_extract(je.value, '$.toolResult.error') IS NOT NULL THEN 1
                    ELSE 0
                END as is_error
            FROM messages m, json_each(m.content_json) je
            WHERE m.role = 'user'
              AND json_extract(je.value, '$.type') = 'toolResponse'
              AND m.created_timestamp > unixepoch() - (? * 86400)
            "#,
        )
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        let total_errors: i64 = error_rows.iter().map(|(_, e)| *e).sum();
        let total_responses = error_rows.len() as i64;

        // Match tool request IDs with response IDs to get per-tool error counts
        // For now, distribute errors proportionally if we can't match 1:1
        let error_map = self.get_per_tool_errors(days).await.unwrap_or_default();

        let stats: Vec<ToolUsageStat> = rows
            .into_iter()
            .map(|(name, count)| {
                let error_count = error_map.get(&name).copied().unwrap_or(0);
                let success_rate = if count > 0 {
                    ((count - error_count) as f64 / count as f64) * 100.0
                } else {
                    100.0
                };
                let extension = name.split("__").next().unwrap_or("unknown").to_string();
                ToolUsageStat {
                    tool_name: name,
                    extension,
                    call_count: count,
                    error_count,
                    success_rate,
                }
            })
            .collect();

        // Use total_errors and total_responses to avoid unused variable warnings
        let _ = (total_errors, total_responses);

        Ok(stats)
    }

    /// Get per-tool error counts by joining request IDs with response IDs
    async fn get_per_tool_errors(
        &self,
        days: i32,
    ) -> Result<std::collections::HashMap<String, i64>> {
        // Join tool requests with their responses via matching IDs
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            WITH tool_requests AS (
                SELECT
                    m.session_id,
                    json_extract(je.value, '$.id') as request_id,
                    json_extract(je.value, '$.toolCall.value.name') as tool_name
                FROM messages m, json_each(m.content_json) je
                WHERE m.role = 'assistant'
                  AND json_extract(je.value, '$.type') = 'toolRequest'
                  AND json_extract(je.value, '$.toolCall.value.name') IS NOT NULL
                  AND m.created_timestamp > unixepoch() - (? * 86400)
            ),
            tool_responses AS (
                SELECT
                    m.session_id,
                    json_extract(je.value, '$.id') as response_id,
                    CASE
                        WHEN json_extract(je.value, '$.toolResult.value.isError') = 1 THEN 1
                        WHEN json_extract(je.value, '$.toolResult.error') IS NOT NULL THEN 1
                        ELSE 0
                    END as is_error
                FROM messages m, json_each(m.content_json) je
                WHERE m.role = 'user'
                  AND json_extract(je.value, '$.type') = 'toolResponse'
                  AND m.created_timestamp > unixepoch() - (? * 86400)
            )
            SELECT
                tr.tool_name,
                SUM(tres.is_error) as error_count
            FROM tool_requests tr
            JOIN tool_responses tres ON tr.request_id = tres.response_id AND tr.session_id = tres.session_id
            WHERE tres.is_error > 0
            GROUP BY tr.tool_name
            "#,
        )
        .bind(days)
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().collect())
    }

    /// Get daily tool activity
    async fn get_daily_tool_activity(&self, days: i32) -> Result<Vec<DailyToolActivity>> {
        let rows: Vec<(String, i64, i64)> = sqlx::query_as(
            r#"
            WITH daily_tools AS (
                SELECT
                    date(m.created_timestamp, 'unixepoch') as day,
                    json_extract(je.value, '$.toolCall.value.name') as tool_name,
                    1 as is_call
                FROM messages m, json_each(m.content_json) je
                WHERE m.role = 'assistant'
                  AND json_extract(je.value, '$.type') = 'toolRequest'
                  AND json_extract(je.value, '$.toolCall.value.name') IS NOT NULL
                  AND m.created_timestamp > unixepoch() - (? * 86400)
            )
            SELECT
                day,
                COUNT(*) as tool_calls,
                COUNT(DISTINCT tool_name) as unique_tools
            FROM daily_tools
            GROUP BY day
            ORDER BY day ASC
            "#,
        )
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        // Get daily errors separately
        let error_rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT
                date(m.created_timestamp, 'unixepoch') as day,
                COUNT(*) as error_count
            FROM messages m, json_each(m.content_json) je
            WHERE m.role = 'user'
              AND json_extract(je.value, '$.type') = 'toolResponse'
              AND (
                  json_extract(je.value, '$.toolResult.value.isError') = 1
                  OR json_extract(je.value, '$.toolResult.error') IS NOT NULL
              )
              AND m.created_timestamp > unixepoch() - (? * 86400)
            GROUP BY day
            "#,
        )
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        let error_map: std::collections::HashMap<String, i64> = error_rows.into_iter().collect();

        Ok(rows
            .into_iter()
            .map(|(date, calls, unique)| DailyToolActivity {
                date: date.clone(),
                tool_calls: calls,
                tool_errors: *error_map.get(&date).unwrap_or(&0),
                unique_tools: unique,
            })
            .collect())
    }

    /// Get per-extension usage stats
    async fn get_extension_usage(&self, days: i32) -> Result<Vec<ExtensionUsageStat>> {
        let rows: Vec<(String, i64, i64)> = sqlx::query_as(
            r#"
            WITH tool_calls AS (
                SELECT
                    json_extract(je.value, '$.toolCall.value.name') as tool_name
                FROM messages m, json_each(m.content_json) je
                WHERE m.role = 'assistant'
                  AND json_extract(je.value, '$.type') = 'toolRequest'
                  AND json_extract(je.value, '$.toolCall.value.name') IS NOT NULL
                  AND m.created_timestamp > unixepoch() - (? * 86400)
            ),
            ext_tools AS (
                SELECT
                    CASE
                        WHEN instr(tool_name, '__') > 0
                            THEN substr(tool_name, 1, instr(tool_name, '__') - 1)
                        ELSE 'unknown'
                    END as extension,
                    tool_name
                FROM tool_calls
            )
            SELECT
                extension,
                COUNT(DISTINCT tool_name) as tool_count,
                COUNT(*) as total_calls
            FROM ext_tools
            GROUP BY extension
            ORDER BY total_calls DESC
            "#,
        )
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(ext, tool_count, total_calls)| ExtensionUsageStat {
                extension: ext,
                tool_count,
                total_calls,
                total_errors: 0, // TODO: join with error data
                success_rate: 100.0,
            })
            .collect())
    }

    /// Get tool summaries per session (last N sessions)
    async fn get_session_tool_summaries(&self, days: i32) -> Result<Vec<SessionToolSummary>> {
        let rows: Vec<(String, String, i64, i64, String)> = sqlx::query_as(
            r#"
            WITH session_tools AS (
                SELECT
                    m.session_id,
                    json_extract(je.value, '$.toolCall.value.name') as tool_name
                FROM messages m, json_each(m.content_json) je
                WHERE m.role = 'assistant'
                  AND json_extract(je.value, '$.type') = 'toolRequest'
                  AND json_extract(je.value, '$.toolCall.value.name') IS NOT NULL
                  AND m.created_timestamp > unixepoch() - (? * 86400)
            ),
            session_tool_counts AS (
                SELECT
                    session_id,
                    COUNT(*) as tool_calls,
                    COUNT(DISTINCT tool_name) as unique_tools,
                    (SELECT tool_name FROM session_tools st2
                     WHERE st2.session_id = session_tools.session_id
                     GROUP BY tool_name ORDER BY COUNT(*) DESC LIMIT 1) as most_used
                FROM session_tools
                GROUP BY session_id
            )
            SELECT
                stc.session_id,
                COALESCE(s.name, 'Unnamed Session') as session_name,
                stc.tool_calls,
                stc.unique_tools,
                COALESCE(stc.most_used, '') as most_used_tool
            FROM session_tool_counts stc
            LEFT JOIN sessions s ON stc.session_id = s.id
            ORDER BY s.updated_at DESC
            LIMIT 20
            "#,
        )
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, name, calls, unique, most_used)| SessionToolSummary {
                session_id: id,
                session_name: name,
                tool_calls: calls,
                tool_errors: 0,
                unique_tools: unique,
                most_used_tool: most_used,
                created_at: String::new(),
            })
            .collect())
    }

    /// Get agent performance metrics from session data
    pub async fn get_agent_performance(&self, days: i32) -> Result<AgentPerformanceMetrics> {
        let provider_stats: Vec<(String, i64, f64, f64)> = sqlx::query_as(
            r#"
            SELECT
                COALESCE(provider_name, 'unknown') as provider,
                COUNT(*) as session_count,
                AVG(COALESCE(total_tokens, 0)) as avg_tokens,
                AVG(COALESCE(message_count, 0)) as avg_messages
            FROM sessions
            WHERE created_at > datetime('now', '-' || ? || ' days')
            GROUP BY provider
            ORDER BY session_count DESC
            "#,
        )
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        let avg_stats: (f64, f64) = sqlx::query_as(
            r#"
            SELECT
                AVG(COALESCE(message_count, 0)),
                AVG(COALESCE(total_tokens, 0))
            FROM sessions
            WHERE created_at > datetime('now', '-' || ? || ' days')
            "#,
        )
        .bind(days)
        .fetch_one(self.pool)
        .await?;

        // Avg tool calls per session
        let avg_tools: (f64,) = sqlx::query_as(
            r#"
            WITH session_tool_counts AS (
                SELECT
                    m.session_id,
                    COUNT(*) as tool_count
                FROM messages m, json_each(m.content_json) je
                WHERE m.role = 'assistant'
                  AND json_extract(je.value, '$.type') = 'toolRequest'
                  AND m.created_timestamp > unixepoch() - (? * 86400)
                GROUP BY m.session_id
            )
            SELECT COALESCE(AVG(tool_count), 0.0) FROM session_tool_counts
            "#,
        )
        .bind(days)
        .fetch_one(self.pool)
        .await?;

        // Session duration stats (time between first and last message)
        let duration_stats: Vec<(f64,)> = sqlx::query_as(
            r#"
            SELECT
                COALESCE(MAX(created_timestamp) - MIN(created_timestamp), 0) as duration_seconds
            FROM messages
            WHERE created_timestamp > unixepoch() - (? * 86400)
            GROUP BY session_id
            HAVING COUNT(*) > 1
            ORDER BY duration_seconds
            "#,
        )
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        let durations: Vec<f64> = duration_stats.iter().map(|(d,)| *d).collect();
        let duration = compute_duration_stats(&durations);

        Ok(AgentPerformanceMetrics {
            sessions_by_provider: provider_stats
                .into_iter()
                .map(|(provider, count, tokens, messages)| ProviderSessionStat {
                    provider,
                    session_count: count,
                    avg_tokens: tokens,
                    avg_messages: messages,
                })
                .collect(),
            avg_messages_per_session: avg_stats.0,
            avg_tools_per_session: avg_tools.0,
            avg_tokens_per_session: avg_stats.1,
            session_duration_stats: duration,
            active_extensions: Vec::new(),
        })
    }
}

fn compute_duration_stats(durations: &[f64]) -> DurationStats {
    if durations.is_empty() {
        return DurationStats {
            avg_seconds: 0.0,
            median_seconds: 0.0,
            p90_seconds: 0.0,
        };
    }
    let sum: f64 = durations.iter().sum();
    let avg = sum / durations.len() as f64;
    let median = if durations.len().is_multiple_of(2) {
        (durations[durations.len() / 2 - 1] + durations[durations.len() / 2]) / 2.0
    } else {
        durations[durations.len() / 2]
    };
    let p90_idx = ((durations.len() as f64) * 0.9).ceil() as usize;
    let p90 = durations[p90_idx.min(durations.len() - 1)];

    DurationStats {
        avg_seconds: avg,
        median_seconds: median,
        p90_seconds: p90,
    }
}
