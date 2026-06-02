use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing::info;

pub const CURRENT_SCHEMA_VERSION: i32 = 13;

pub(crate) async fn run_migrations(pool: &Pool<Sqlite>) -> Result<()> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;

    let current_version = get_schema_version(&mut tx).await?;

    if current_version < CURRENT_SCHEMA_VERSION {
        info!(
            "Running database migrations from v{} to v{}...",
            current_version, CURRENT_SCHEMA_VERSION
        );

        for version in (current_version + 1)..=CURRENT_SCHEMA_VERSION {
            info!("  Applying migration v{}...", version);
            apply_migration(&mut tx, version).await?;
            update_schema_version(&mut tx, version).await?;
            info!("  ✓ Migration v{} complete", version);
        }

        info!("All migrations complete");
    }

    tx.commit().await?;
    Ok(())
}

pub(crate) async fn get_schema_version(tx: &mut sqlx::Transaction<'_, Sqlite>) -> Result<i32> {
    let table_exists = sqlx::query_scalar::<_, bool>(
        r#"
            SELECT EXISTS (
                SELECT name FROM sqlite_master
                WHERE type='table' AND name='schema_version'
            )
        "#,
    )
    .fetch_one(&mut **tx)
    .await?;

    if !table_exists {
        return Ok(0);
    }

    let version = sqlx::query_scalar::<_, i32>("SELECT MAX(version) FROM schema_version")
        .fetch_one(&mut **tx)
        .await?;

    Ok(version)
}

pub(crate) async fn update_schema_version(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    version: i32,
) -> Result<()> {
    sqlx::query("INSERT INTO schema_version (version) VALUES (?)")
        .bind(version)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[allow(clippy::too_many_lines)]
pub(crate) async fn apply_migration(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    version: i32,
) -> Result<()> {
    match version {
        1 => {
            sqlx::query(
                r#"
                    CREATE TABLE IF NOT EXISTS schema_version (
                        version INTEGER PRIMARY KEY,
                        applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    )
                "#,
            )
            .execute(&mut **tx)
            .await?;
        }
        2 => {
            sqlx::query(
                r#"
                    ALTER TABLE sessions ADD COLUMN user_recipe_values_json TEXT
                "#,
            )
            .execute(&mut **tx)
            .await?;
        }
        3 => {
            sqlx::query(
                r#"
                    ALTER TABLE messages ADD COLUMN metadata_json TEXT
                "#,
            )
            .execute(&mut **tx)
            .await?;
        }
        4 => {
            sqlx::query(
                r#"
                    ALTER TABLE sessions ADD COLUMN name TEXT DEFAULT ''
                "#,
            )
            .execute(&mut **tx)
            .await?;

            sqlx::query(
                r#"
                    ALTER TABLE sessions ADD COLUMN user_set_name BOOLEAN DEFAULT FALSE
                "#,
            )
            .execute(&mut **tx)
            .await?;
        }
        5 => {
            sqlx::query(
                r#"
                    ALTER TABLE sessions ADD COLUMN session_type TEXT NOT NULL DEFAULT 'user'
                "#,
            )
            .execute(&mut **tx)
            .await?;

            sqlx::query("CREATE INDEX idx_sessions_type ON sessions(session_type)")
                .execute(&mut **tx)
                .await?;
        }
        6 => {
            sqlx::query(
                r#"
                    ALTER TABLE sessions ADD COLUMN provider_name TEXT
                "#,
            )
            .execute(&mut **tx)
            .await?;

            sqlx::query(
                r#"
                    ALTER TABLE sessions ADD COLUMN model_config_json TEXT
                "#,
            )
            .execute(&mut **tx)
            .await?;
        }
        7 => {
            sqlx::query(
                r#"
                    ALTER TABLE messages ADD COLUMN message_id TEXT
                "#,
            )
            .execute(&mut **tx)
            .await?;

            sqlx::query(
                r#"
                    UPDATE messages
                    SET message_id = 'msg_' || session_id || '_' || id
                "#,
            )
            .execute(&mut **tx)
            .await?;

            sqlx::query("CREATE INDEX idx_messages_message_id ON messages(message_id)")
                .execute(&mut **tx)
                .await?;
        }
        8 => {
            sqlx::query(
                r#"
                    ALTER TABLE sessions ADD COLUMN goose_mode TEXT NOT NULL DEFAULT 'auto'
                "#,
            )
            .execute(&mut **tx)
            .await?;
        }
        9 => {
            sqlx::query(
                r#"
                    UPDATE sessions
                    SET session_type = 'acp'
                    WHERE session_type = 'user'
                      AND name = 'ACP Session'
                      AND user_set_name = FALSE
                "#,
            )
            .execute(&mut **tx)
            .await?;
        }
        10 => {
            // Check if thread_id column already exists (e.g. fresh schema)
            let has_thread_id = sqlx::query_scalar::<_, i32>(
                "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'thread_id'",
            )
            .fetch_one(&mut **tx)
            .await?
                > 0;
            if !has_thread_id {
                sqlx::query("ALTER TABLE sessions ADD COLUMN thread_id TEXT")
                    .execute(&mut **tx)
                    .await?;
            }
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_thread ON sessions(thread_id)")
                .execute(&mut **tx)
                .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS threads (
                        id TEXT PRIMARY KEY,
                        name TEXT NOT NULL DEFAULT 'New Chat',
                        user_set_name BOOLEAN DEFAULT FALSE,
                        working_dir TEXT,
                        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                        updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                        archived_at TIMESTAMP,
                        metadata_json TEXT DEFAULT '{}'
                    )",
            )
            .execute(&mut **tx)
            .await?;
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS thread_messages (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        thread_id TEXT NOT NULL REFERENCES threads(id),
                        session_id TEXT,
                        message_id TEXT,
                        role TEXT NOT NULL,
                        content_json TEXT NOT NULL,
                        created_timestamp INTEGER NOT NULL,
                        metadata_json TEXT DEFAULT '{}'
                    )",
            )
            .execute(&mut **tx)
            .await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_thread_messages_thread ON thread_messages(thread_id)")
                .execute(&mut **tx)
                .await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_thread_messages_message_id ON thread_messages(message_id)")
                .execute(&mut **tx)
                .await?;
        }
        11 => {
            crate::providers::inventory::create_tables_in_tx(tx).await?;
        }
        12 => {
            // Add archived_at, project_id columns to sessions.
            let has_archived_at = sqlx::query_scalar::<_, i32>(
                "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'archived_at'",
            )
            .fetch_one(&mut **tx)
            .await?
                > 0;
            if !has_archived_at {
                sqlx::query("ALTER TABLE sessions ADD COLUMN archived_at TIMESTAMP")
                    .execute(&mut **tx)
                    .await?;
            }

            let has_project_id = sqlx::query_scalar::<_, i32>(
                "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'project_id'",
            )
            .fetch_one(&mut **tx)
            .await?
                > 0;
            if !has_project_id {
                sqlx::query("ALTER TABLE sessions ADD COLUMN project_id TEXT")
                    .execute(&mut **tx)
                    .await?;
            }
        }
        13 => {
            let has_accumulated_cost = sqlx::query_scalar::<_, i32>(
                "SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name = 'accumulated_cost'",
            )
            .fetch_one(&mut **tx)
            .await?
                > 0;
            if !has_accumulated_cost {
                sqlx::query("ALTER TABLE sessions ADD COLUMN accumulated_cost REAL")
                    .execute(&mut **tx)
                    .await?;
            }
        }
        _ => {
            anyhow::bail!("Unknown migration version: {}", version);
        }
    }

    Ok(())
}
