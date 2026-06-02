use super::*;

pub async fn create_tables(pool: &Pool<Sqlite>) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS provider_inventory_entries (
            inventory_key TEXT PRIMARY KEY,
            provider_id TEXT NOT NULL,
            provider_family TEXT NOT NULL,
            last_updated_at TEXT,
            last_refresh_attempt_at TEXT,
            last_refresh_error TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS provider_inventory_models (
            inventory_key TEXT NOT NULL REFERENCES provider_inventory_entries(inventory_key) ON DELETE CASCADE,
            ordinal INTEGER NOT NULL,
            model_id TEXT NOT NULL,
            name TEXT NOT NULL,
            family TEXT,
            context_limit INTEGER,
            reasoning BOOLEAN,
            recommended BOOLEAN,
            PRIMARY KEY (inventory_key, ordinal)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_provider_inventory_provider_id ON provider_inventory_entries(provider_id)",
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_tables_in_tx(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS provider_inventory_entries (
            inventory_key TEXT PRIMARY KEY,
            provider_id TEXT NOT NULL,
            provider_family TEXT NOT NULL,
            last_updated_at TEXT,
            last_refresh_attempt_at TEXT,
            last_refresh_error TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS provider_inventory_models (
            inventory_key TEXT NOT NULL REFERENCES provider_inventory_entries(inventory_key) ON DELETE CASCADE,
            ordinal INTEGER NOT NULL,
            model_id TEXT NOT NULL,
            name TEXT NOT NULL,
            family TEXT,
            context_limit INTEGER,
            reasoning BOOLEAN,
            recommended BOOLEAN,
            PRIMARY KEY (inventory_key, ordinal)
        )
        "#,
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_provider_inventory_provider_id ON provider_inventory_entries(provider_id)",
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}
