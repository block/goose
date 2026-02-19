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
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
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

/// Live monitoring metrics — recent activity snapshot
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LiveMetrics {
    /// Active sessions in the last hour
    pub active_sessions_1h: i32,
    /// Active sessions in the last 24 hours
    pub active_sessions_24h: i32,
    /// Tool calls in the last hour
    pub tool_calls_1h: i32,
    /// Tool errors in the last hour
    pub tool_errors_1h: i32,
    /// Tool calls in the last 24 hours
    pub tool_calls_24h: i32,
    /// Tool errors in the last 24 hours
    pub tool_errors_24h: i32,
    /// Success rate in the last hour (0.0-1.0)
    pub success_rate_1h: f64,
    /// Messages processed in the last hour
    pub messages_1h: i32,
    /// Most active tools in the last hour
    pub hot_tools: Vec<HotTool>,
    /// Recent errors (last 10)
    pub recent_errors: Vec<RecentError>,
    /// Per-minute activity for the last 60 minutes
    pub activity_timeline: Vec<MinuteActivity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HotTool {
    pub tool_name: String,
    pub calls_1h: i32,
    pub errors_1h: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecentError {
    pub tool_name: String,
    pub session_id: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MinuteActivity {
    pub minute: String,
    pub tool_calls: i32,
    pub messages: i32,
}

/// Response quality proxy metrics derived from session/message patterns
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ResponseQualityMetrics {
    pub total_sessions: i64,
    pub avg_session_duration_secs: f64,
    pub avg_messages_per_session: f64,
    pub avg_user_messages_per_session: f64,
    pub retry_rate: f64,
    pub avg_tool_errors_per_session: f64,
    pub avg_tokens_per_session: f64,
    pub completion_rate: f64,
    pub sessions_with_errors: i64,
    pub daily_quality: Vec<DailyQuality>,
    pub quality_by_provider: Vec<ProviderQuality>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DailyQuality {
    pub date: String,
    pub sessions: i64,
    pub avg_duration_secs: f64,
    pub avg_messages: f64,
    pub retry_rate: f64,
    pub error_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderQuality {
    pub provider: String,
    pub sessions: i64,
    pub avg_duration_secs: f64,
    pub avg_messages: f64,
    pub avg_tokens: f64,
    pub retry_rate: f64,
    pub error_rate: f64,
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

    /// Get per-tool error counts using a two-step approach to avoid O(n²) CTE cross-joins.
    /// Step 1: Collect error response IDs (small set).
    /// Step 2: Look up those IDs in tool requests to get tool names.
    async fn get_per_tool_errors(
        &self,
        days: i32,
    ) -> Result<std::collections::HashMap<String, i64>> {
        // Step 1: Get (session_id, response_id) pairs for error responses — typically small
        let error_ids: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT
                m.session_id,
                json_extract(je.value, '$.id') as response_id
            FROM messages m, json_each(m.content_json) je
            WHERE m.role = 'user'
              AND json_extract(je.value, '$.type') = 'toolResponse'
              AND (
                  json_extract(je.value, '$.toolResult.value.isError') = 1
                  OR json_extract(je.value, '$.toolResult.error') IS NOT NULL
              )
              AND m.created_timestamp > unixepoch() - (? * 86400)
            "#,
        )
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        if error_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        // Step 2: Look up tool names for those specific IDs
        // Build a set of (session_id, request_id) to match against
        let id_set: std::collections::HashSet<(&str, &str)> = error_ids
            .iter()
            .map(|(sid, rid)| (sid.as_str(), rid.as_str()))
            .collect();

        // Fetch all tool requests from sessions that had errors
        let session_ids: Vec<&str> = error_ids.iter().map(|(s, _)| s.as_str()).collect();
        let session_ids_dedup: std::collections::HashSet<&str> =
            session_ids.into_iter().collect();
        let placeholders: String = session_ids_dedup.iter().map(|_| "?").collect::<Vec<_>>().join(",");

        let query = format!(
            r#"
            SELECT
                m.session_id,
                json_extract(je.value, '$.id') as request_id,
                json_extract(je.value, '$.toolCall.value.name') as tool_name
            FROM messages m, json_each(m.content_json) je
            WHERE m.role = 'assistant'
              AND json_extract(je.value, '$.type') = 'toolRequest'
              AND json_extract(je.value, '$.toolCall.value.name') IS NOT NULL
              AND m.session_id IN ({})
              AND m.created_timestamp > unixepoch() - (? * 86400)
            "#,
            placeholders
        );

        let mut qb = sqlx::query_as::<_, (String, String, String)>(&query);
        for sid in &session_ids_dedup {
            qb = qb.bind(*sid);
        }
        qb = qb.bind(days);

        let requests: Vec<(String, String, String)> = qb.fetch_all(self.pool).await?;

        // Match error response IDs to request IDs in Rust (O(n) with HashSet)
        let mut error_map: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        for (session_id, request_id, tool_name) in &requests {
            if id_set.contains(&(session_id.as_str(), request_id.as_str())) {
                *error_map.entry(tool_name.clone()).or_insert(0) += 1;
            }
        }

        Ok(error_map)
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
                COALESCE(s.provider_name, 'unknown') as provider,
                COUNT(DISTINCT s.id) as session_count,
                CAST(AVG(COALESCE(s.total_tokens, 0)) AS REAL) as avg_tokens,
                CAST(AVG(COALESCE(mc.msg_count, 0)) AS REAL) as avg_messages
            FROM sessions s
            LEFT JOIN (
                SELECT session_id, COUNT(*) as msg_count
                FROM messages
                GROUP BY session_id
            ) mc ON mc.session_id = s.id
            WHERE s.created_at > datetime('now', '-' || ? || ' days')
            GROUP BY provider
            ORDER BY session_count DESC
            "#,
        )
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        let avg_stats: (Option<f64>, Option<f64>) = sqlx::query_as(
            r#"
            SELECT
                CAST(AVG(COALESCE(mc.msg_count, 0)) AS REAL),
                CAST(AVG(COALESCE(s.total_tokens, 0)) AS REAL)
            FROM sessions s
            LEFT JOIN (
                SELECT session_id, COUNT(*) as msg_count
                FROM messages
                GROUP BY session_id
            ) mc ON mc.session_id = s.id
            WHERE s.created_at > datetime('now', '-' || ? || ' days')
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
                CAST(COALESCE(MAX(created_timestamp) - MIN(created_timestamp), 0) AS REAL) as duration_seconds
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
            avg_messages_per_session: avg_stats.0.unwrap_or(0.0),
            avg_tools_per_session: avg_tools.0,
            avg_tokens_per_session: avg_stats.1.unwrap_or(0.0),
            session_duration_stats: duration,
            active_extensions: Vec::new(),
        })
    }

    /// Get live monitoring metrics — recent activity snapshot
    pub async fn get_live_metrics(&self) -> Result<LiveMetrics> {
        // Active sessions in last 1h and 24h
        let (sessions_1h,): (i32,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT session_id) FROM messages WHERE created_timestamp > unixepoch() - 3600",
        )
        .fetch_one(self.pool)
        .await?;

        let (sessions_24h,): (i32,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT session_id) FROM messages WHERE created_timestamp > unixepoch() - 86400",
        )
        .fetch_one(self.pool)
        .await?;

        // Tool calls in last 1h
        let (calls_1h,): (i32,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM messages m, json_each(m.content_json) je
            WHERE m.role = 'assistant'
              AND json_extract(je.value, '$.type') = 'toolRequest'
              AND json_extract(je.value, '$.toolCall.value.name') IS NOT NULL
              AND m.created_timestamp > unixepoch() - 3600
            "#,
        )
        .fetch_one(self.pool)
        .await?;

        // Tool errors in last 1h
        let (errors_1h,): (i32,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM messages m, json_each(m.content_json) je
            WHERE m.role = 'user'
              AND json_extract(je.value, '$.type') = 'toolResponse'
              AND (json_extract(je.value, '$.toolResult.value.isError') = 1
                   OR json_extract(je.value, '$.toolResult.error') IS NOT NULL)
              AND m.created_timestamp > unixepoch() - 3600
            "#,
        )
        .fetch_one(self.pool)
        .await?;

        // Tool calls/errors in last 24h
        let (calls_24h,): (i32,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM messages m, json_each(m.content_json) je
            WHERE m.role = 'assistant'
              AND json_extract(je.value, '$.type') = 'toolRequest'
              AND json_extract(je.value, '$.toolCall.value.name') IS NOT NULL
              AND m.created_timestamp > unixepoch() - 86400
            "#,
        )
        .fetch_one(self.pool)
        .await?;

        let (errors_24h,): (i32,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM messages m, json_each(m.content_json) je
            WHERE m.role = 'user'
              AND json_extract(je.value, '$.type') = 'toolResponse'
              AND (json_extract(je.value, '$.toolResult.value.isError') = 1
                   OR json_extract(je.value, '$.toolResult.error') IS NOT NULL)
              AND m.created_timestamp > unixepoch() - 86400
            "#,
        )
        .fetch_one(self.pool)
        .await?;

        // Messages in last 1h
        let (messages_1h,): (i32,) = sqlx::query_as(
            "SELECT COUNT(*) FROM messages WHERE created_timestamp > unixepoch() - 3600",
        )
        .fetch_one(self.pool)
        .await?;

        let success_rate_1h = if calls_1h > 0 {
            (calls_1h - errors_1h) as f64 / calls_1h as f64
        } else {
            1.0
        };

        // Hot tools (most active in last 1h)
        let hot_tools: Vec<(String, i32)> = sqlx::query_as(
            r#"
            SELECT json_extract(je.value, '$.toolCall.value.name'), COUNT(*)
            FROM messages m, json_each(m.content_json) je
            WHERE m.role = 'assistant'
              AND json_extract(je.value, '$.type') = 'toolRequest'
              AND json_extract(je.value, '$.toolCall.value.name') IS NOT NULL
              AND m.created_timestamp > unixepoch() - 3600
            GROUP BY 1 ORDER BY 2 DESC LIMIT 10
            "#,
        )
        .fetch_all(self.pool)
        .await?;

        // Recent errors (last 10)
        let recent_errors: Vec<(String, String, i64)> = sqlx::query_as(
            r#"
            SELECT
                COALESCE(json_extract(je.value, '$.id'), '') as response_id,
                m.session_id,
                m.created_timestamp
            FROM messages m, json_each(m.content_json) je
            WHERE m.role = 'user'
              AND json_extract(je.value, '$.type') = 'toolResponse'
              AND (json_extract(je.value, '$.toolResult.value.isError') = 1
                   OR json_extract(je.value, '$.toolResult.error') IS NOT NULL)
            ORDER BY m.created_timestamp DESC
            LIMIT 10
            "#,
        )
        .fetch_all(self.pool)
        .await?;

        // Per-minute activity for last 60 minutes
        let timeline: Vec<(String, i32, i32)> = sqlx::query_as(
            r#"
            SELECT
                strftime('%H:%M', created_timestamp, 'unixepoch') as minute,
                SUM(CASE WHEN role = 'assistant' THEN 1 ELSE 0 END) as tool_calls,
                COUNT(*) as messages
            FROM messages
            WHERE created_timestamp > unixepoch() - 3600
            GROUP BY minute
            ORDER BY minute ASC
            "#,
        )
        .fetch_all(self.pool)
        .await?;

        Ok(LiveMetrics {
            active_sessions_1h: sessions_1h,
            active_sessions_24h: sessions_24h,
            tool_calls_1h: calls_1h,
            tool_errors_1h: errors_1h,
            tool_calls_24h: calls_24h,
            tool_errors_24h: errors_24h,
            success_rate_1h,
            messages_1h,
            hot_tools: hot_tools
                .into_iter()
                .map(|(name, calls)| HotTool {
                    tool_name: name,
                    calls_1h: calls,
                    errors_1h: 0,
                })
                .collect(),
            recent_errors: recent_errors
                .into_iter()
                .map(|(id, session, ts)| RecentError {
                    tool_name: id,
                    session_id: session,
                    timestamp: chrono::DateTime::from_timestamp(ts, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_default(),
                })
                .collect(),
            activity_timeline: timeline
                .into_iter()
                .map(|(minute, calls, msgs)| MinuteActivity {
                    minute,
                    tool_calls: calls,
                    messages: msgs,
                })
                .collect(),
        })
    }

    /// Get response quality proxy metrics from session patterns
    pub async fn get_response_quality(&self, days: i32) -> Result<ResponseQualityMetrics> {
        // Overall session metrics (message_count computed via subquery since it's not a column)
        let overall: (i64, Option<f64>, Option<f64>, Option<f64>, f64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total_sessions,
                AVG(COALESCE(
                    julianday(s.updated_at) - julianday(s.created_at), 0
                ) * 86400) as avg_duration_secs,
                CAST(AVG(mc.msg_count) AS REAL) as avg_messages,
                CAST(AVG(COALESCE(s.total_tokens, 0)) AS REAL) as avg_tokens,
                0.0 as placeholder,
                0 as placeholder2
            FROM sessions s
            INNER JOIN (
                SELECT session_id, COUNT(*) as msg_count
                FROM messages
                GROUP BY session_id
                HAVING COUNT(*) > 0
            ) mc ON mc.session_id = s.id
            WHERE s.created_at > datetime('now', '-' || ? || ' days')
            "#,
        )
        .bind(days)
        .fetch_one(self.pool)
        .await?;

        // Retry rate: consecutive user messages (user sent >1 message before assistant responded)
        let retry_data: (f64,) = sqlx::query_as(
            r#"
            WITH msg_pairs AS (
                SELECT
                    session_id,
                    role,
                    LAG(role) OVER (PARTITION BY session_id ORDER BY created_timestamp) as prev_role
                FROM messages
                WHERE created_timestamp > unixepoch() - (? * 86400)
            )
            SELECT
                COALESCE(
                    CAST(SUM(CASE WHEN role = 'user' AND prev_role = 'user' THEN 1 ELSE 0 END) AS REAL) /
                    NULLIF(CAST(SUM(CASE WHEN role = 'user' THEN 1 ELSE 0 END) AS REAL), 0),
                    0.0
                ) as retry_rate
            FROM msg_pairs
            "#,
        )
        .bind(days)
        .fetch_one(self.pool)
        .await?;

        // Average user messages per session
        let user_msgs: (f64,) = sqlx::query_as(
            r#"
            SELECT COALESCE(AVG(user_count), 0.0) FROM (
                SELECT session_id, COUNT(*) as user_count
                FROM messages
                WHERE role = 'user'
                  AND created_timestamp > unixepoch() - (? * 86400)
                GROUP BY session_id
            )
            "#,
        )
        .bind(days)
        .fetch_one(self.pool)
        .await?;

        // Tool errors per session
        let tool_errors: (f64,) = sqlx::query_as(
            r#"
            SELECT COALESCE(AVG(err_count), 0.0) FROM (
                SELECT m.session_id, COUNT(*) as err_count
                FROM messages m, json_each(m.content_json) je
                WHERE m.role = 'user'
                  AND json_extract(je.value, '$.type') = 'toolResponse'
                  AND (json_extract(je.value, '$.toolResult.value.isError') = 1
                       OR json_extract(je.value, '$.toolResult.error') IS NOT NULL)
                  AND m.created_timestamp > unixepoch() - (? * 86400)
                GROUP BY m.session_id
            )
            "#,
        )
        .bind(days)
        .fetch_one(self.pool)
        .await?;

        // Sessions with errors
        let (sessions_with_errors,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT m.session_id)
            FROM messages m, json_each(m.content_json) je
            WHERE m.role = 'user'
              AND json_extract(je.value, '$.type') = 'toolResponse'
              AND (json_extract(je.value, '$.toolResult.value.isError') = 1
                   OR json_extract(je.value, '$.toolResult.error') IS NOT NULL)
              AND m.created_timestamp > unixepoch() - (? * 86400)
            "#,
        )
        .bind(days)
        .fetch_one(self.pool)
        .await?;

        // Completion rate: sessions with >2 messages and last message from assistant
        let completion: (f64,) = sqlx::query_as(
            r#"
            WITH session_last AS (
                SELECT
                    session_id,
                    role as last_role,
                    ROW_NUMBER() OVER (PARTITION BY session_id ORDER BY created_timestamp DESC) as rn
                FROM messages
                WHERE created_timestamp > unixepoch() - (? * 86400)
            ),
            completed AS (
                SELECT session_id, last_role
                FROM session_last WHERE rn = 1
            )
            SELECT COALESCE(
                CAST(SUM(CASE WHEN last_role = 'assistant' THEN 1 ELSE 0 END) AS REAL) /
                NULLIF(CAST(COUNT(*) AS REAL), 0),
                0.0
            ) FROM completed
            "#,
        )
        .bind(days)
        .fetch_one(self.pool)
        .await?;

        // Daily quality trend (join messages to compute message count)
        let daily: Vec<(String, i64, f64, f64)> = sqlx::query_as(
            r#"
            SELECT
                date(s.created_at) as day,
                COUNT(DISTINCT s.id) as sessions,
                AVG(COALESCE(julianday(s.updated_at) - julianday(s.created_at), 0) * 86400) as avg_duration,
                CAST(AVG(mc.msg_count) AS REAL) as avg_messages
            FROM sessions s
            INNER JOIN (
                SELECT session_id, COUNT(*) as msg_count
                FROM messages
                GROUP BY session_id
                HAVING COUNT(*) > 0
            ) mc ON mc.session_id = s.id
            WHERE s.created_at > datetime('now', '-' || ? || ' days')
            GROUP BY day
            ORDER BY day ASC
            "#,
        )
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        // Quality by provider (join messages to compute message count)
        let by_provider: Vec<(String, i64, f64, f64, f64)> = sqlx::query_as(
            r#"
            SELECT
                COALESCE(s.provider_name, 'unknown') as provider,
                COUNT(DISTINCT s.id) as sessions,
                AVG(COALESCE(julianday(s.updated_at) - julianday(s.created_at), 0) * 86400) as avg_duration,
                CAST(AVG(mc.msg_count) AS REAL) as avg_messages,
                CAST(AVG(COALESCE(s.total_tokens, 0)) AS REAL) as avg_tokens
            FROM sessions s
            INNER JOIN (
                SELECT session_id, COUNT(*) as msg_count
                FROM messages
                GROUP BY session_id
                HAVING COUNT(*) > 0
            ) mc ON mc.session_id = s.id
            WHERE s.created_at > datetime('now', '-' || ? || ' days')
            GROUP BY provider
            ORDER BY sessions DESC
            "#,
        )
        .bind(days)
        .fetch_all(self.pool)
        .await?;

        Ok(ResponseQualityMetrics {
            total_sessions: overall.0,
            avg_session_duration_secs: overall.1.unwrap_or(0.0),
            avg_messages_per_session: overall.2.unwrap_or(0.0),
            avg_user_messages_per_session: user_msgs.0,
            retry_rate: retry_data.0,
            avg_tool_errors_per_session: tool_errors.0,
            avg_tokens_per_session: overall.3.unwrap_or(0.0),
            completion_rate: completion.0,
            sessions_with_errors,
            daily_quality: daily
                .into_iter()
                .map(|(date, sessions, dur, msgs)| DailyQuality {
                    date,
                    sessions,
                    avg_duration_secs: dur,
                    avg_messages: msgs,
                    retry_rate: 0.0,
                    error_rate: 0.0,
                })
                .collect(),
            quality_by_provider: by_provider
                .into_iter()
                .map(|(provider, sessions, dur, msgs, tokens)| ProviderQuality {
                    provider,
                    sessions,
                    avg_duration_secs: dur,
                    avg_messages: msgs,
                    avg_tokens: tokens,
                    retry_rate: 0.0,
                    error_rate: 0.0,
                })
                .collect(),
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
