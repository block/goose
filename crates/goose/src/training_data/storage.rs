use super::schema::TrainingExample;
use sqlx::Row;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Trait for training data storage backends
#[async_trait]
pub trait TrainingDataStorage: Send + Sync {
    /// Store a new training example
    async fn store_example(&self, example: TrainingExample) -> Result<()>;

    /// Retrieve a training example by ID
    async fn get_example(&self, id: Uuid) -> Result<Option<TrainingExample>>;

    /// Update an existing training example
    async fn update_example(&self, example: TrainingExample) -> Result<()>;

    /// Delete a training example
    async fn delete_example(&self, id: Uuid) -> Result<bool>;

    /// Get examples suitable for training with optional filters
    async fn get_examples_for_training(
        &self,
        limit: Option<usize>,
        min_quality_score: Option<f32>,
        domain_tags: Option<Vec<String>>,
    ) -> Result<Vec<TrainingExample>>;

    /// Delete old examples based on retention policy
    async fn delete_old_examples(&self, retention_days: u32) -> Result<usize>;

    /// Get training statistics
    async fn get_statistics(&self) -> Result<TrainingDataStatistics>;
}

/// Statistics about training data
#[derive(Debug, Clone)]
pub struct TrainingDataStatistics {
    pub total_examples: usize,
    pub high_quality_examples: usize,
    pub examples_with_feedback: usize,
    pub average_quality_score: f32,
    pub examples_by_domain: HashMap<String, usize>,
    pub examples_by_provider: HashMap<String, usize>,
}

/// In-memory storage implementation for testing and development
pub struct InMemoryTrainingDataStorage {
    examples: Arc<RwLock<HashMap<Uuid, TrainingExample>>>,
}

impl InMemoryTrainingDataStorage {
    pub fn new() -> Self {
        Self {
            examples: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl TrainingDataStorage for InMemoryTrainingDataStorage {
    async fn store_example(&self, example: TrainingExample) -> Result<()> {
        let mut examples = self.examples.write().await;
        examples.insert(example.id, example);
        Ok(())
    }

    async fn get_example(&self, id: Uuid) -> Result<Option<TrainingExample>> {
        let examples = self.examples.read().await;
        Ok(examples.get(&id).cloned())
    }

    async fn update_example(&self, example: TrainingExample) -> Result<()> {
        let mut examples = self.examples.write().await;
        examples.insert(example.id, example);
        Ok(())
    }

    async fn delete_example(&self, id: Uuid) -> Result<bool> {
        let mut examples = self.examples.write().await;
        Ok(examples.remove(&id).is_some())
    }

    async fn get_examples_for_training(
        &self,
        limit: Option<usize>,
        min_quality_score: Option<f32>,
        domain_tags: Option<Vec<String>>,
    ) -> Result<Vec<TrainingExample>> {
        let examples = self.examples.read().await;
        let mut filtered_examples: Vec<TrainingExample> = examples
            .values()
            .filter(|example| {
                // Filter by quality score
                if let Some(min_score) = min_quality_score {
                    if example.quality_metrics.overall_score < min_score {
                        return false;
                    }
                }

                // Filter by domain tags
                if let Some(ref tags) = domain_tags {
                    if !tags.iter().any(|tag| example.domain_tags.contains(tag)) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort by quality score (highest first)
        filtered_examples.sort_by(|a, b| {
            b.quality_metrics
                .overall_score
                .partial_cmp(&a.quality_metrics.overall_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        if let Some(limit) = limit {
            filtered_examples.truncate(limit);
        }

        Ok(filtered_examples)
    }

    async fn delete_old_examples(&self, retention_days: u32) -> Result<usize> {
        let cutoff_date = Utc::now() - Duration::days(retention_days as i64);
        let mut examples = self.examples.write().await;

        let old_example_ids: Vec<Uuid> = examples
            .values()
            .filter(|example| example.created_at < cutoff_date)
            .map(|example| example.id)
            .collect();

        let deleted_count = old_example_ids.len();
        for id in old_example_ids {
            examples.remove(&id);
        }

        Ok(deleted_count)
    }

    async fn get_statistics(&self) -> Result<TrainingDataStatistics> {
        let examples = self.examples.read().await;

        let total_examples = examples.len();
        let high_quality_examples = examples
            .values()
            .filter(|e| e.quality_metrics.overall_score >= 0.8)
            .count();
        let examples_with_feedback = examples
            .values()
            .filter(|e| e.user_feedback.is_some())
            .count();

        let average_quality_score = if total_examples > 0 {
            examples
                .values()
                .map(|e| e.quality_metrics.overall_score)
                .sum::<f32>()
                / total_examples as f32
        } else {
            0.0
        };

        let mut examples_by_domain = HashMap::new();
        let mut examples_by_provider = HashMap::new();

        for example in examples.values() {
            for tag in &example.domain_tags {
                *examples_by_domain.entry(tag.clone()).or_insert(0) += 1;
            }
            *examples_by_provider
                .entry(example.metadata.provider_used.clone())
                .or_insert(0) += 1;
        }

        Ok(TrainingDataStatistics {
            total_examples,
            high_quality_examples,
            examples_with_feedback,
            average_quality_score,
            examples_by_domain,
            examples_by_provider,
        })
    }
}

/// SQLite-based storage implementation for production use
pub struct SqliteTrainingDataStorage {
    db_path: String,
    pool: Option<sqlx::SqlitePool>,
}

impl SqliteTrainingDataStorage {
    pub fn new(db_path: String) -> Self {
        Self {
            db_path,
            pool: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        let pool = sqlx::SqlitePool::connect(&format!("sqlite:{}", self.db_path)).await?;

        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS training_examples (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                session_id TEXT,
                messages TEXT NOT NULL,
                user_feedback TEXT,
                quality_metrics TEXT NOT NULL,
                domain_tags TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Create indexes for better query performance
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_conversation_id ON training_examples(conversation_id)",
        )
        .execute(&pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_quality_score ON training_examples(json_extract(quality_metrics, '$.overall_score'))")
            .execute(&pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_created_at ON training_examples(created_at)")
            .execute(&pool)
            .await?;

        self.pool = Some(pool);
        Ok(())
    }

    fn get_pool(&self) -> Result<&sqlx::SqlitePool> {
        self.pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized"))
    }
}

#[async_trait]
impl TrainingDataStorage for SqliteTrainingDataStorage {
    async fn store_example(&self, example: TrainingExample) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            INSERT INTO training_examples (
                id, conversation_id, session_id, messages, user_feedback,
                quality_metrics, domain_tags, metadata, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(example.id.to_string())
        .bind(&example.conversation_id)
        .bind(&example.session_id)
        .bind(serde_json::to_string(&example.messages)?)
        .bind(serde_json::to_string(&example.user_feedback)?)
        .bind(serde_json::to_string(&example.quality_metrics)?)
        .bind(serde_json::to_string(&example.domain_tags)?)
        .bind(serde_json::to_string(&example.metadata)?)
        .bind(example.created_at.to_rfc3339())
        .bind(example.updated_at.to_rfc3339())
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn get_example(&self, id: Uuid) -> Result<Option<TrainingExample>> {
        let pool = self.get_pool()?;

        let row = sqlx::query("SELECT * FROM training_examples WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            let example = TrainingExample {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                conversation_id: row.get("conversation_id"),
                session_id: row.get("session_id"),
                messages: serde_json::from_str(&row.get::<String, _>("messages"))?,
                user_feedback: serde_json::from_str(&row.get::<String, _>("user_feedback"))?,
                quality_metrics: serde_json::from_str(&row.get::<String, _>("quality_metrics"))?,
                domain_tags: serde_json::from_str(&row.get::<String, _>("domain_tags"))?,
                metadata: serde_json::from_str(&row.get::<String, _>("metadata"))?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?
                    .with_timezone(&Utc),
            };
            Ok(Some(example))
        } else {
            Ok(None)
        }
    }

    async fn update_example(&self, example: TrainingExample) -> Result<()> {
        let pool = self.get_pool()?;

        sqlx::query(
            r#"
            UPDATE training_examples SET
                conversation_id = ?, session_id = ?, messages = ?, user_feedback = ?,
                quality_metrics = ?, domain_tags = ?, metadata = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&example.conversation_id)
        .bind(&example.session_id)
        .bind(serde_json::to_string(&example.messages)?)
        .bind(serde_json::to_string(&example.user_feedback)?)
        .bind(serde_json::to_string(&example.quality_metrics)?)
        .bind(serde_json::to_string(&example.domain_tags)?)
        .bind(serde_json::to_string(&example.metadata)?)
        .bind(example.updated_at.to_rfc3339())
        .bind(example.id.to_string())
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn delete_example(&self, id: Uuid) -> Result<bool> {
        let pool = self.get_pool()?;

        let result = sqlx::query("DELETE FROM training_examples WHERE id = ?")
            .bind(id.to_string())
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_examples_for_training(
        &self,
        limit: Option<usize>,
        min_quality_score: Option<f32>,
        domain_tags: Option<Vec<String>>,
    ) -> Result<Vec<TrainingExample>> {
        let pool = self.get_pool()?;

        let mut query = "SELECT * FROM training_examples WHERE 1=1".to_string();
        let mut bindings = Vec::new();

        if let Some(min_score) = min_quality_score {
            query.push_str(" AND json_extract(quality_metrics, '$.overall_score') >= ?");
            bindings.push(min_score.to_string());
        }

        if let Some(tags) = domain_tags {
            for tag in tags {
                query.push_str(" AND json_extract(domain_tags, '$') LIKE ?");
                bindings.push(format!("%\"{}\"%%", tag));
            }
        }

        query.push_str(" ORDER BY json_extract(quality_metrics, '$.overall_score') DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let mut query_builder = sqlx::query(&query);
        for binding in bindings {
            query_builder = query_builder.bind(binding);
        }

        let rows = query_builder.fetch_all(pool).await?;

        let mut examples = Vec::new();
        for row in rows {
            let example = TrainingExample {
                id: Uuid::parse_str(&row.get::<String, _>("id"))?,
                conversation_id: row.get("conversation_id"),
                session_id: row.get("session_id"),
                messages: serde_json::from_str(&row.get::<String, _>("messages"))?,
                user_feedback: serde_json::from_str(&row.get::<String, _>("user_feedback"))?,
                quality_metrics: serde_json::from_str(&row.get::<String, _>("quality_metrics"))?,
                domain_tags: serde_json::from_str(&row.get::<String, _>("domain_tags"))?,
                metadata: serde_json::from_str(&row.get::<String, _>("metadata"))?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at"))?
                    .with_timezone(&Utc),
            };
            examples.push(example);
        }

        Ok(examples)
    }

    async fn delete_old_examples(&self, retention_days: u32) -> Result<usize> {
        let pool = self.get_pool()?;
        let cutoff_date = (Utc::now() - Duration::days(retention_days as i64)).to_rfc3339();

        let result = sqlx::query("DELETE FROM training_examples WHERE created_at < ?")
            .bind(cutoff_date)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() as usize)
    }

    async fn get_statistics(&self) -> Result<TrainingDataStatistics> {
        let pool = self.get_pool()?;

        // Get basic counts
        let total_examples: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM training_examples")
            .fetch_one(pool)
            .await?;

        let high_quality_examples: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM training_examples WHERE json_extract(quality_metrics, '$.overall_score') >= 0.8"
        )
        .fetch_one(pool)
        .await?;

        let examples_with_feedback: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM training_examples WHERE user_feedback IS NOT NULL",
        )
        .fetch_one(pool)
        .await?;

        // Get average quality score
        let average_quality_score: Option<f64> = sqlx::query_scalar(
            "SELECT AVG(json_extract(quality_metrics, '$.overall_score')) FROM training_examples",
        )
        .fetch_one(pool)
        .await?;

        // For simplicity, we'll return empty maps for domain and provider statistics
        // In a full implementation, these would require more complex JSON queries
        let examples_by_domain = HashMap::new();
        let examples_by_provider = HashMap::new();

        Ok(TrainingDataStatistics {
            total_examples: total_examples as usize,
            high_quality_examples: high_quality_examples as usize,
            examples_with_feedback: examples_with_feedback as usize,
            average_quality_score: average_quality_score.unwrap_or(0.0) as f32,
            examples_by_domain,
            examples_by_provider,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use crate::training_data::schema::{
        PrivacyLevel, QualityMetrics, TrainingMetadata, TrainingSource,
    };
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryTrainingDataStorage::new();

        // Create a test example
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there!"),
        ];

        let example = TrainingExample::new(
            "conv_123".to_string(),
            messages,
            "native".to_string(),
            "llama-3.2-3b".to_string(),
        );

        let example_id = example.id;

        // Test store
        storage.store_example(example).await.unwrap();

        // Test retrieve
        let retrieved = storage.get_example(example_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().conversation_id, "conv_123");

        // Test get examples for training
        let examples = storage
            .get_examples_for_training(None, None, None)
            .await
            .unwrap();
        assert_eq!(examples.len(), 1);

        // Test statistics
        let stats = storage.get_statistics().await.unwrap();
        assert_eq!(stats.total_examples, 1);
    }
}
