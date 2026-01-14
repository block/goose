use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing::info;

pub const CURRENT_SCHEMA_VERSION: i32 = 6;

pub async fn run_migrations(pool: &Pool<Sqlite>) -> Result<()> {
    let current_version = get_schema_version(pool).await?;

    if current_version < CURRENT_SCHEMA_VERSION {
        info!(
            "Running database migrations from v{} to v{}...",
            current_version, CURRENT_SCHEMA_VERSION
        );

        for version in (current_version + 1)..=CURRENT_SCHEMA_VERSION {
            info!("  Applying migration v{}...", version);
            apply_migration(pool, version).await?;
            update_schema_version(pool, version).await?;
            info!("  ✓ Migration v{} complete", version);
        }

        info!("All migrations complete");
    }

    Ok(())
}

async fn get_schema_version(pool: &Pool<Sqlite>) -> Result<i32> {
    let table_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT name FROM sqlite_master
            WHERE type='table' AND name='schema_version'
        )
    "#,
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        return Ok(0);
    }

    let version = sqlx::query_scalar::<_, i32>("SELECT MAX(version) FROM schema_version")
        .fetch_one(pool)
        .await?;

    Ok(version)
}

async fn update_schema_version(pool: &Pool<Sqlite>, version: i32) -> Result<()> {
    sqlx::query("INSERT INTO schema_version (version) VALUES (?)")
        .bind(version)
        .execute(pool)
        .await?;
    Ok(())
}

async fn apply_migration(pool: &Pool<Sqlite>, version: i32) -> Result<()> {
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
            .execute(pool)
            .await?;
        }
        2 => {
            sqlx::query(
                r#"
                ALTER TABLE sessions ADD COLUMN user_recipe_values_json TEXT
            "#,
            )
            .execute(pool)
            .await?;
        }
        3 => {
            sqlx::query(
                r#"
                ALTER TABLE messages ADD COLUMN metadata_json TEXT
            "#,
            )
            .execute(pool)
            .await?;
        }
        4 => {
            sqlx::query(
                r#"
                ALTER TABLE sessions ADD COLUMN name TEXT DEFAULT ''
            "#,
            )
            .execute(pool)
            .await?;

            sqlx::query(
                r#"
                ALTER TABLE sessions ADD COLUMN user_set_name BOOLEAN DEFAULT FALSE
            "#,
            )
            .execute(pool)
            .await?;
        }
        5 => {
            sqlx::query(
                r#"
                ALTER TABLE sessions ADD COLUMN session_type TEXT NOT NULL DEFAULT 'user'
            "#,
            )
            .execute(pool)
            .await?;

            sqlx::query("CREATE INDEX idx_sessions_type ON sessions(session_type)")
                .execute(pool)
                .await?;
        }
        6 => {
            sqlx::query(
                r#"
                ALTER TABLE sessions ADD COLUMN provider_name TEXT
            "#,
            )
            .execute(pool)
            .await?;

            sqlx::query(
                r#"
                ALTER TABLE sessions ADD COLUMN model_config_json TEXT
            "#,
            )
            .execute(pool)
            .await?;
        }
        _ => {
            anyhow::bail!("Unknown migration version: {}", version);
        }
    }

    Ok(())
}
