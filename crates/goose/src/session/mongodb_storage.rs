use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use mongodb::bson::{doc, Bson, Document};
use mongodb::options::{
    ClientOptions, FindOneAndUpdateOptions, FindOptions, IndexOptions, ReturnDocument,
};
use mongodb::{Client, Collection, Database, IndexModel};
use rmcp::model::Role;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::conversation::message::{Message, MessageMetadata};
use crate::conversation::Conversation;
use crate::session::chat_history_search::{ChatRecallMessage, ChatRecallResult, ChatRecallResults};
use crate::session::session_manager::{Session, SessionInsights, SessionType};
use crate::session::storage_backend::{SessionStorageBackend, SessionUpdate};

fn role_to_string(role: &Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

/// Convert a chrono DateTime to a BSON DateTime.
fn to_bson_dt(dt: &DateTime<Utc>) -> mongodb::bson::DateTime {
    mongodb::bson::DateTime::from_millis(dt.timestamp_millis())
}

/// Convert a BSON DateTime to a chrono DateTime.
fn from_bson_dt(dt: &mongodb::bson::DateTime) -> DateTime<Utc> {
    chrono::DateTime::<Utc>::from_timestamp_millis(dt.timestamp_millis()).unwrap_or_default()
}

/// Serialize a serde-serializable value to a BSON value for native document storage.
fn to_bson_value<T: serde::Serialize>(value: &T) -> Result<Bson> {
    let json_val = serde_json::to_value(value)?;
    Ok(mongodb::bson::to_bson(&json_val)?)
}

/// Deserialize a BSON value back to a typed Rust value.
fn from_bson_value<T: serde::de::DeserializeOwned>(bson: &Bson) -> Result<T> {
    Ok(mongodb::bson::from_bson(bson.clone())?)
}

pub struct MongoDbSessionStorage {
    client: Client,
    database: Database,
    sessions: Collection<Document>,
    messages: Collection<Document>,
}

impl MongoDbSessionStorage {
    /// Create from environment variables.
    pub async fn new() -> Result<Self> {
        let uri = std::env::var("GOOSE_MONGODB_URI")
            .map_err(|_| anyhow::anyhow!("GOOSE_MONGODB_URI not set"))?;

        let db_name =
            std::env::var("GOOSE_MONGODB_DATABASE").unwrap_or_else(|_| "goose".to_string());

        let sessions_collection = std::env::var("GOOSE_MONGODB_SESSIONS_COLLECTION")
            .unwrap_or_else(|_| "sessions".to_string());

        let messages_collection = std::env::var("GOOSE_MONGODB_MESSAGES_COLLECTION")
            .unwrap_or_else(|_| "messages".to_string());

        Self::connect(&uri, &db_name, &sessions_collection, &messages_collection).await
    }

    /// Create with explicit parameters.
    pub async fn connect(
        uri: &str,
        db_name: &str,
        sessions_collection: &str,
        messages_collection: &str,
    ) -> Result<Self> {
        let mut client_options = ClientOptions::parse(uri).await?;

        if let Ok(pool_size) = std::env::var("GOOSE_MONGODB_MAX_POOL_SIZE") {
            client_options.max_pool_size = Some(pool_size.parse()?);
        }

        if let Ok(timeout_ms) = std::env::var("GOOSE_MONGODB_CONNECT_TIMEOUT_MS") {
            client_options.connect_timeout =
                Some(std::time::Duration::from_millis(timeout_ms.parse()?));
        }

        if let Ok(timeout_ms) = std::env::var("GOOSE_MONGODB_SERVER_SELECTION_TIMEOUT_MS") {
            client_options.server_selection_timeout =
                Some(std::time::Duration::from_millis(timeout_ms.parse()?));
        }

        let client = Client::with_options(client_options)?;
        let database = client.database(&db_name);

        let storage = Self {
            client,
            database: database.clone(),
            sessions: database.collection(&sessions_collection),
            messages: database.collection(&messages_collection),
        };

        storage.ensure_indexes().await?;

        Ok(storage)
    }

    async fn ensure_indexes(&self) -> Result<()> {
        // Sessions: unique on id
        self.sessions
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "id": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
                None,
            )
            .await?;

        // Sessions: type + updated_at for listing
        self.sessions
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "session_type": 1, "updated_at": -1 })
                    .build(),
                None,
            )
            .await?;

        // Messages: session_id + created for ordered retrieval
        self.messages
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "session_id": 1, "created": 1 })
                    .build(),
                None,
            )
            .await?;

        // Messages: unique on id
        self.messages
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "id": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
                None,
            )
            .await?;

        // Messages: text index for chat history search
        self.messages
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "content_text": "text" })
                    .build(),
                None,
            )
            .await?;

        Ok(())
    }

    /// Generate a unique session ID using an atomic counter, matching SQLite's date-based pattern.
    async fn generate_session_id(&self) -> Result<String> {
        let today = chrono::Utc::now().format("%Y%m%d").to_string();

        let counters: Collection<Document> = self.database.collection("counters");
        let result = counters
            .find_one_and_update(
                doc! { "_id": &today },
                doc! { "$inc": { "seq": 1_i64 } },
                FindOneAndUpdateOptions::builder()
                    .upsert(true)
                    .return_document(ReturnDocument::After)
                    .build(),
            )
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to generate session ID"))?;

        let seq = result
            .get_i64("seq")
            .or_else(|_| result.get_i32("seq").map(|v| v as i64))
            .map_err(|_| anyhow::anyhow!("Invalid counter value"))?;

        Ok(format!("{}_{}", today, seq))
    }

    /// Convert a MongoDB session document to a Session struct.
    /// Field names match what `serde_json::to_string_pretty(&session)` produces in export_session.
    fn document_to_session(&self, doc: &Document) -> Result<Session> {
        let name = doc.get_str("name").unwrap_or("").to_string();
        let user_set_name = doc.get_bool("user_set_name").unwrap_or(false);

        let session_type_str = doc.get_str("session_type").unwrap_or("user");
        let session_type: SessionType = session_type_str.parse().unwrap_or_default();

        let created_at = doc
            .get_datetime("created_at")
            .map(from_bson_dt)
            .unwrap_or_default();
        let updated_at = doc
            .get_datetime("updated_at")
            .map(from_bson_dt)
            .unwrap_or_default();

        // extension_data: native BSON document → ExtensionData
        let extension_data = doc
            .get("extension_data")
            .and_then(|v| from_bson_value(v).ok())
            .unwrap_or_default();

        let provider_name = doc.get_str("provider_name").ok().map(|s| s.to_string());
        let schedule_id = doc.get_str("schedule_id").ok().map(|s| s.to_string());

        // recipe: native BSON document → Recipe
        let recipe = doc.get("recipe").and_then(|v| {
            if v == &Bson::Null {
                None
            } else {
                from_bson_value(v).ok()
            }
        });

        // user_recipe_values: native BSON document → HashMap<String, String>
        let user_recipe_values = doc.get("user_recipe_values").and_then(|v| {
            if v == &Bson::Null {
                None
            } else {
                from_bson_value(v).ok()
            }
        });

        // model_config: native BSON document → ModelConfig
        let model_config = doc.get("model_config").and_then(|v| {
            if v == &Bson::Null {
                None
            } else {
                from_bson_value(v).ok()
            }
        });

        Ok(Session {
            id: doc.get_str("id").unwrap_or("").to_string(),
            working_dir: PathBuf::from(doc.get_str("working_dir").unwrap_or(".")),
            name,
            user_set_name,
            session_type,
            created_at,
            updated_at,
            extension_data,
            total_tokens: doc.get_i32("total_tokens").ok(),
            input_tokens: doc.get_i32("input_tokens").ok(),
            output_tokens: doc.get_i32("output_tokens").ok(),
            accumulated_total_tokens: doc.get_i32("accumulated_total_tokens").ok(),
            accumulated_input_tokens: doc.get_i32("accumulated_input_tokens").ok(),
            accumulated_output_tokens: doc.get_i32("accumulated_output_tokens").ok(),
            schedule_id,
            recipe,
            user_recipe_values,
            conversation: None,
            message_count: 0,
            provider_name,
            model_config,
        })
    }

    /// Convert a MongoDB message document to a Message struct.
    /// Field names match the Message struct's serde serialization.
    fn document_to_message(&self, doc: &Document) -> Result<Message> {
        let role_str = doc.get_str("role").unwrap_or("user");
        let role = match role_str {
            "assistant" => Role::Assistant,
            _ => Role::User,
        };

        // content: native BSON array → Vec<MessageContent>
        let content = doc
            .get("content")
            .and_then(|v| from_bson_value(v).ok())
            .unwrap_or_default();

        // metadata: native BSON document → MessageMetadata
        let metadata: MessageMetadata = doc
            .get("metadata")
            .and_then(|v| from_bson_value(v).ok())
            .unwrap_or_default();

        let created = doc.get_i64("created").unwrap_or(0);

        let mut message = Message::new(role, created, content);
        message.metadata = metadata;

        if let Ok(id) = doc.get_str("id") {
            message = message.with_id(id.to_string());
        }

        Ok(message)
    }

    /// Build a BSON document for a message, matching the Message struct's serde field names.
    fn message_to_document(&self, session_id: &str, message: &Message) -> Result<Document> {
        let message_id = message
            .id
            .clone()
            .unwrap_or_else(|| format!("msg_{}_{}", session_id, uuid::Uuid::new_v4()));

        // content: native BSON array (matches export_session's "content" field)
        let content_bson = to_bson_value(&message.content)?;

        // metadata: native BSON document (matches export_session's "metadata" field)
        let metadata_bson = to_bson_value(&message.metadata)?;

        // Extract text for search index (extra field, not in export format)
        let content_text: String = message
            .content
            .iter()
            .filter_map(|c| c.as_text())
            .collect::<Vec<_>>()
            .join(" ");

        Ok(doc! {
            "id": &message_id,
            "session_id": session_id,
            "role": role_to_string(&message.role),
            "created": message.created,
            "content": content_bson,
            "metadata": metadata_bson,
            "content_text": content_text,
        })
    }

    async fn get_conversation(&self, session_id: &str) -> Result<Conversation> {
        let filter = doc! { "session_id": session_id };
        let options = FindOptions::builder()
            .sort(doc! { "created": 1 })
            .build();

        let mut cursor = self.messages.find(filter, options).await?;
        let mut messages = Vec::new();

        while let Some(doc) = cursor.try_next().await? {
            messages.push(self.document_to_message(&doc)?);
        }

        Ok(Conversation::new_unvalidated(messages))
    }
}

#[async_trait]
impl SessionStorageBackend for MongoDbSessionStorage {
    async fn create_session(
        &self,
        working_dir: PathBuf,
        name: String,
        session_type: SessionType,
    ) -> Result<Session> {
        let id = self.generate_session_id().await?;
        let now = mongodb::bson::DateTime::now();

        let doc = doc! {
            "id": &id,
            "name": &name,
            "user_set_name": false,
            "session_type": session_type.to_string(),
            "working_dir": working_dir.to_string_lossy().to_string(),
            "created_at": now,
            "updated_at": now,
            "extension_data": {},
            "message_count": 0_i32,
        };

        self.sessions.insert_one(doc, None).await?;
        crate::posthog::emit_session_started();

        self.get_session(&id, false).await
    }

    async fn get_session(&self, id: &str, include_messages: bool) -> Result<Session> {
        let filter = doc! { "id": id };
        let doc = self
            .sessions
            .find_one(filter, None)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        let mut session = self.document_to_session(&doc)?;

        if include_messages {
            let conv = self.get_conversation(id).await?;
            session.message_count = conv.messages().len();
            session.conversation = Some(conv);
        } else {
            let count = self
                .messages
                .count_documents(doc! { "session_id": id }, None)
                .await? as usize;
            session.message_count = count;
        }

        Ok(session)
    }

    async fn apply_update(&self, session_id: &str, update: SessionUpdate) -> Result<()> {
        let mut set_doc = doc! { "updated_at": mongodb::bson::DateTime::now() };

        if let Some(name) = update.name {
            set_doc.insert("name", name);
        }
        if let Some(usn) = update.user_set_name {
            set_doc.insert("user_set_name", usn);
        }
        if let Some(st) = update.session_type {
            set_doc.insert("session_type", st.to_string());
        }
        if let Some(wd) = update.working_dir {
            set_doc.insert("working_dir", wd.to_string_lossy().to_string());
        }
        if let Some(ed) = update.extension_data {
            set_doc.insert("extension_data", to_bson_value(&ed)?);
        }
        if let Some(tt) = update.total_tokens {
            match tt {
                Some(v) => set_doc.insert("total_tokens", v),
                None => set_doc.insert("total_tokens", Bson::Null),
            };
        }
        if let Some(it) = update.input_tokens {
            match it {
                Some(v) => set_doc.insert("input_tokens", v),
                None => set_doc.insert("input_tokens", Bson::Null),
            };
        }
        if let Some(ot) = update.output_tokens {
            match ot {
                Some(v) => set_doc.insert("output_tokens", v),
                None => set_doc.insert("output_tokens", Bson::Null),
            };
        }
        if let Some(att) = update.accumulated_total_tokens {
            match att {
                Some(v) => set_doc.insert("accumulated_total_tokens", v),
                None => set_doc.insert("accumulated_total_tokens", Bson::Null),
            };
        }
        if let Some(ait) = update.accumulated_input_tokens {
            match ait {
                Some(v) => set_doc.insert("accumulated_input_tokens", v),
                None => set_doc.insert("accumulated_input_tokens", Bson::Null),
            };
        }
        if let Some(aot) = update.accumulated_output_tokens {
            match aot {
                Some(v) => set_doc.insert("accumulated_output_tokens", v),
                None => set_doc.insert("accumulated_output_tokens", Bson::Null),
            };
        }
        if let Some(sid) = update.schedule_id {
            match sid {
                Some(v) => set_doc.insert("schedule_id", v),
                None => set_doc.insert("schedule_id", Bson::Null),
            };
        }
        if let Some(recipe) = update.recipe {
            match recipe {
                Some(r) => set_doc.insert("recipe", to_bson_value(&r)?),
                None => set_doc.insert("recipe", Bson::Null),
            };
        }
        if let Some(urv) = update.user_recipe_values {
            match urv {
                Some(v) => set_doc.insert("user_recipe_values", to_bson_value(&v)?),
                None => set_doc.insert("user_recipe_values", Bson::Null),
            };
        }
        if let Some(pn) = update.provider_name {
            match pn {
                Some(v) => set_doc.insert("provider_name", v),
                None => set_doc.insert("provider_name", Bson::Null),
            };
        }
        if let Some(mc) = update.model_config {
            match mc {
                Some(m) => set_doc.insert("model_config", to_bson_value(&m)?),
                None => set_doc.insert("model_config", Bson::Null),
            };
        }

        self.sessions
            .update_one(
                doc! { "id": session_id },
                doc! { "$set": set_doc },
                None,
            )
            .await?;

        Ok(())
    }

    async fn delete_session(&self, id: &str) -> Result<()> {
        let result = self
            .sessions
            .delete_one(doc! { "id": id }, None)
            .await?;

        if result.deleted_count == 0 {
            return Err(anyhow::anyhow!("Session not found"));
        }

        self.messages
            .delete_many(doc! { "session_id": id }, None)
            .await?;

        Ok(())
    }

    async fn list_sessions_by_types(&self, types: &[SessionType]) -> Result<Vec<Session>> {
        if types.is_empty() {
            return Ok(Vec::new());
        }

        let type_strings: Vec<String> = types.iter().map(|t| t.to_string()).collect();

        // Use aggregation to filter sessions that have messages and match types
        let pipeline = vec![
            doc! { "$match": { "session_type": { "$in": &type_strings } } },
            doc! {
                "$lookup": {
                    "from": self.messages.name(),
                    "localField": "id",
                    "foreignField": "session_id",
                    "as": "msgs"
                }
            },
            doc! { "$match": { "msgs.0": { "$exists": true } } },
            doc! { "$addFields": { "message_count_val": { "$size": "$msgs" } } },
            doc! { "$project": { "msgs": 0 } },
            doc! { "$sort": { "updated_at": -1 } },
        ];

        let mut cursor = self.sessions.aggregate(pipeline, None).await?;
        let mut sessions = Vec::new();

        while let Some(doc) = cursor.try_next().await? {
            let mut session = self.document_to_session(&doc)?;
            session.message_count = doc.get_i32("message_count_val").unwrap_or(0) as usize;
            sessions.push(session);
        }

        Ok(sessions)
    }

    async fn add_message(&self, session_id: &str, message: &Message) -> Result<()> {
        let doc = self.message_to_document(session_id, message)?;
        self.messages.insert_one(doc, None).await?;

        // Update session timestamp
        self.sessions
            .update_one(
                doc! { "id": session_id },
                doc! { "$set": { "updated_at": mongodb::bson::DateTime::now() } },
                None,
            )
            .await?;

        Ok(())
    }

    async fn replace_conversation(
        &self,
        session_id: &str,
        conversation: &Conversation,
    ) -> Result<()> {
        // Delete existing messages
        self.messages
            .delete_many(doc! { "session_id": session_id }, None)
            .await?;

        // Insert new messages
        for message in conversation.messages() {
            let doc = self.message_to_document(session_id, message)?;
            self.messages.insert_one(doc, None).await?;
        }

        Ok(())
    }

    async fn truncate_conversation(&self, session_id: &str, timestamp: i64) -> Result<()> {
        self.messages
            .delete_many(
                doc! {
                    "session_id": session_id,
                    "created": { "$gte": timestamp }
                },
                None,
            )
            .await?;

        Ok(())
    }

    async fn get_message_metadata(
        &self,
        session_id: &str,
        message_id: &str,
    ) -> Result<MessageMetadata> {
        let filter = doc! { "id": message_id, "session_id": session_id };
        let doc = self
            .messages
            .find_one(filter, None)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Message not found"))?;

        doc.get("metadata")
            .and_then(|v| from_bson_value(v).ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid metadata"))
    }

    async fn set_message_metadata(
        &self,
        session_id: &str,
        message_id: &str,
        metadata: MessageMetadata,
    ) -> Result<()> {
        let metadata_bson = to_bson_value(&metadata)?;

        self.messages
            .update_one(
                doc! { "id": message_id, "session_id": session_id },
                doc! { "$set": { "metadata": metadata_bson } },
                None,
            )
            .await?;

        Ok(())
    }

    async fn get_insights(&self) -> Result<SessionInsights> {
        let total_sessions = self.sessions.count_documents(doc! {}, None).await? as usize;

        // Aggregate total tokens
        let pipeline = vec![
            doc! {
                "$group": {
                    "_id": null,
                    "total_tokens": {
                        "$sum": {
                            "$ifNull": [
                                "$accumulated_total_tokens",
                                { "$ifNull": ["$total_tokens", 0] }
                            ]
                        }
                    }
                }
            },
        ];

        let mut cursor = self.sessions.aggregate(pipeline, None).await?;
        let total_tokens = if let Some(doc) = cursor.try_next().await? {
            doc.get_i64("total_tokens")
                .or_else(|_| doc.get_i32("total_tokens").map(|v| v as i64))
                .unwrap_or(0)
        } else {
            0
        };

        Ok(SessionInsights {
            total_sessions,
            total_tokens,
        })
    }

    async fn search_chat_history(
        &self,
        query: &str,
        limit: Option<usize>,
        after_date: Option<DateTime<Utc>>,
        before_date: Option<DateTime<Utc>>,
        exclude_session_id: Option<String>,
    ) -> Result<ChatRecallResults> {
        let limit = limit.unwrap_or(10);

        // Build filter using text search on denormalized content_text field
        let mut filter = doc! { "$text": { "$search": query } };

        if let Some(exclude_id) = exclude_session_id {
            filter.insert("session_id", doc! { "$ne": exclude_id });
        }

        let mut timestamp_filter = Document::new();
        if let Some(after) = after_date {
            timestamp_filter.insert("$gte", to_bson_dt(&after));
        }
        if let Some(before) = before_date {
            timestamp_filter.insert("$lte", to_bson_dt(&before));
        }
        if !timestamp_filter.is_empty() {
            filter.insert("created", timestamp_filter);
        }

        let options = FindOptions::builder()
            .sort(doc! { "created": -1 })
            .limit(limit as i64)
            .build();

        let mut cursor = self.messages.find(filter, options).await?;

        // Group messages by session
        let mut session_messages: HashMap<String, Vec<(String, String, DateTime<Utc>)>> =
            HashMap::new();

        while let Some(doc) = cursor.try_next().await? {
            let session_id = doc.get_str("session_id").unwrap_or("").to_string();
            let role = doc.get_str("role").unwrap_or("user").to_string();
            let content_text = doc.get_str("content_text").unwrap_or("").to_string();
            let created = doc.get_i64("created").unwrap_or(0);
            let timestamp =
                chrono::DateTime::<Utc>::from_timestamp_millis(created).unwrap_or_default();

            session_messages
                .entry(session_id)
                .or_default()
                .push((role, content_text, timestamp));
        }

        // Build results with session info
        let mut results = Vec::new();

        for (session_id, msgs) in &session_messages {
            // Get session info
            let session_doc = self
                .sessions
                .find_one(doc! { "id": session_id }, None)
                .await?;

            let (description, working_dir) = match session_doc {
                Some(ref doc) => (
                    doc.get_str("name").unwrap_or("").to_string(),
                    doc.get_str("working_dir").unwrap_or("").to_string(),
                ),
                None => (String::new(), String::new()),
            };

            // Get total message count for this session
            let total_messages_in_session = self
                .messages
                .count_documents(doc! { "session_id": session_id }, None)
                .await? as usize;

            let message_vec: Vec<ChatRecallMessage> = msgs
                .iter()
                .map(|(role, content, timestamp)| ChatRecallMessage {
                    role: role.clone(),
                    content: content.clone(),
                    timestamp: *timestamp,
                })
                .collect();

            let last_activity = message_vec
                .iter()
                .map(|m| m.timestamp)
                .max()
                .unwrap_or_else(Utc::now);

            results.push(ChatRecallResult {
                session_id: session_id.clone(),
                session_description: description,
                session_working_dir: working_dir,
                last_activity,
                total_messages_in_session,
                messages: message_vec,
            });
        }

        results.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));

        let total_matches = results.iter().map(|r| r.messages.len()).sum();

        Ok(ChatRecallResults {
            results,
            total_matches,
        })
    }

    async fn health_check(&self) -> Result<()> {
        self.client
            .database("admin")
            .run_command(doc! { "ping": 1 }, None)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::MessageContent;
    use crate::session::session_manager::Session;
    use crate::session::storage_backend::SessionUpdate;
    use std::sync::Arc;

    /// Create a test storage instance with a unique database name.
    /// Returns None if GOOSE_MONGODB_URI is not set (test will be skipped).
    async fn setup_test_storage() -> Option<MongoDbSessionStorage> {
        let uri = match std::env::var("GOOSE_MONGODB_URI") {
            Ok(uri) => uri,
            Err(_) => return None,
        };

        let db_name = format!(
            "goose_test_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "")
        );

        Some(
            MongoDbSessionStorage::connect(&uri, &db_name, "sessions", "messages")
                .await
                .expect("Failed to create test storage"),
        )
    }

    /// Drop the test database after use.
    async fn teardown(storage: &MongoDbSessionStorage) {
        storage.database.drop(None).await.ok();
    }

    #[tokio::test]
    async fn test_health_check() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        assert!(storage.health_check().await.is_ok());
        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_create_and_get_session() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let session = storage
            .create_session(
                PathBuf::from("/tmp/test"),
                "Test Session".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        assert!(!session.id.is_empty());
        assert_eq!(session.name, "Test Session");
        assert_eq!(session.working_dir, PathBuf::from("/tmp/test"));
        assert_eq!(session.session_type, SessionType::User);
        assert_eq!(session.message_count, 0);

        let retrieved = storage.get_session(&session.id, false).await.unwrap();
        assert_eq!(retrieved.id, session.id);
        assert_eq!(retrieved.name, "Test Session");
        assert_eq!(retrieved.working_dir, PathBuf::from("/tmp/test"));

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_session_id_format() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let session = storage
            .create_session(PathBuf::from("/tmp"), "S1".to_string(), SessionType::User)
            .await
            .unwrap();

        // ID should match YYYYMMDD_N pattern
        let parts: Vec<&str> = session.id.split('_').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 8); // YYYYMMDD
        assert!(parts[1].parse::<i64>().is_ok()); // sequence number

        // Second session on same day should increment
        let session2 = storage
            .create_session(PathBuf::from("/tmp"), "S2".to_string(), SessionType::User)
            .await
            .unwrap();

        let seq1: i64 = session.id.split('_').nth(1).unwrap().parse().unwrap();
        let seq2: i64 = session2.id.split('_').nth(1).unwrap().parse().unwrap();
        assert_eq!(seq2, seq1 + 1);

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_add_and_get_messages() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let session = storage
            .create_session(PathBuf::from("/tmp"), "Test".to_string(), SessionType::User)
            .await
            .unwrap();

        let msg1 = Message {
            id: Some("msg-1".to_string()),
            role: Role::User,
            created: 1000,
            content: vec![MessageContent::text("Hello")],
            metadata: MessageMetadata::default(),
        };

        let msg2 = Message {
            id: Some("msg-2".to_string()),
            role: Role::Assistant,
            created: 2000,
            content: vec![MessageContent::text("Hi there!")],
            metadata: MessageMetadata::default(),
        };

        storage.add_message(&session.id, &msg1).await.unwrap();
        storage.add_message(&session.id, &msg2).await.unwrap();

        let retrieved = storage.get_session(&session.id, true).await.unwrap();
        assert_eq!(retrieved.message_count, 2);

        let conv = retrieved.conversation.unwrap();
        let messages = conv.messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[0].created, 1000);
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[1].created, 2000);

        // Verify message IDs round-trip
        assert_eq!(messages[0].id.as_deref(), Some("msg-1"));
        assert_eq!(messages[1].id.as_deref(), Some("msg-2"));

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_message_content_roundtrip() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let session = storage
            .create_session(PathBuf::from("/tmp"), "Test".to_string(), SessionType::User)
            .await
            .unwrap();

        let msg = Message {
            id: Some("msg-content".to_string()),
            role: Role::User,
            created: 1000,
            content: vec![
                MessageContent::text("First part"),
                MessageContent::text("Second part"),
            ],
            metadata: MessageMetadata::default(),
        };

        storage.add_message(&session.id, &msg).await.unwrap();

        let retrieved = storage.get_session(&session.id, true).await.unwrap();
        let messages = retrieved.conversation.unwrap();
        let m = &messages.messages()[0];

        // Content should round-trip as native BSON and back
        assert_eq!(m.content.len(), 2);

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_apply_update() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let session = storage
            .create_session(
                PathBuf::from("/tmp"),
                "Original".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        storage
            .apply_update(
                &session.id,
                SessionUpdate {
                    name: Some("Updated".to_string()),
                    total_tokens: Some(Some(100)),
                    input_tokens: Some(Some(60)),
                    output_tokens: Some(Some(40)),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let updated = storage.get_session(&session.id, false).await.unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.total_tokens, Some(100));
        assert_eq!(updated.input_tokens, Some(60));
        assert_eq!(updated.output_tokens, Some(40));

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_apply_update_nullable_fields() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let session = storage
            .create_session(PathBuf::from("/tmp"), "Test".to_string(), SessionType::User)
            .await
            .unwrap();

        // Set a value
        storage
            .apply_update(
                &session.id,
                SessionUpdate {
                    total_tokens: Some(Some(500)),
                    provider_name: Some(Some("anthropic".to_string())),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let s = storage.get_session(&session.id, false).await.unwrap();
        assert_eq!(s.total_tokens, Some(500));
        assert_eq!(s.provider_name.as_deref(), Some("anthropic"));

        // Clear it back to null
        storage
            .apply_update(
                &session.id,
                SessionUpdate {
                    total_tokens: Some(None),
                    provider_name: Some(None),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let s = storage.get_session(&session.id, false).await.unwrap();
        assert_eq!(s.total_tokens, None);
        assert_eq!(s.provider_name, None);

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_delete_session() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let session = storage
            .create_session(
                PathBuf::from("/tmp"),
                "To Delete".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        storage
            .add_message(
                &session.id,
                &Message {
                    id: Some("msg-1".to_string()),
                    role: Role::User,
                    created: 1000,
                    content: vec![MessageContent::text("Hello")],
                    metadata: MessageMetadata::default(),
                },
            )
            .await
            .unwrap();

        storage.delete_session(&session.id).await.unwrap();

        let result = storage.get_session(&session.id, false).await;
        assert!(result.is_err());

        // Messages should also be deleted
        let msg_count = storage
            .messages
            .count_documents(doc! { "session_id": &session.id }, None)
            .await
            .unwrap();
        assert_eq!(msg_count, 0);

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_delete_nonexistent_session() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let result = storage.delete_session("nonexistent_id").await;
        assert!(result.is_err());

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_list_sessions_by_types() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        // Create sessions of different types
        let s1 = storage
            .create_session(PathBuf::from("/tmp"), "User 1".to_string(), SessionType::User)
            .await
            .unwrap();
        let s2 = storage
            .create_session(
                PathBuf::from("/tmp"),
                "Scheduled 1".to_string(),
                SessionType::Scheduled,
            )
            .await
            .unwrap();
        let _s3 = storage
            .create_session(
                PathBuf::from("/tmp"),
                "Hidden 1".to_string(),
                SessionType::Hidden,
            )
            .await
            .unwrap();

        // Add messages so sessions appear in listing (sessions without messages are filtered)
        for sid in [&s1.id, &s2.id] {
            storage
                .add_message(
                    sid,
                    &Message {
                        id: None,
                        role: Role::User,
                        created: 1000,
                        content: vec![MessageContent::text("test")],
                        metadata: MessageMetadata::default(),
                    },
                )
                .await
                .unwrap();
        }

        // List only User sessions
        let user_sessions = storage
            .list_sessions_by_types(&[SessionType::User])
            .await
            .unwrap();
        assert_eq!(user_sessions.len(), 1);
        assert_eq!(user_sessions[0].name, "User 1");

        // List User + Scheduled
        let mixed = storage
            .list_sessions_by_types(&[SessionType::User, SessionType::Scheduled])
            .await
            .unwrap();
        assert_eq!(mixed.len(), 2);

        // Empty types returns empty
        let empty = storage.list_sessions_by_types(&[]).await.unwrap();
        assert!(empty.is_empty());

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_replace_conversation() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let session = storage
            .create_session(PathBuf::from("/tmp"), "Test".to_string(), SessionType::User)
            .await
            .unwrap();

        // Add initial messages
        storage
            .add_message(
                &session.id,
                &Message {
                    id: Some("old-1".to_string()),
                    role: Role::User,
                    created: 1000,
                    content: vec![MessageContent::text("old message")],
                    metadata: MessageMetadata::default(),
                },
            )
            .await
            .unwrap();

        // Replace with new conversation
        let new_messages = vec![
            Message {
                id: Some("new-1".to_string()),
                role: Role::User,
                created: 2000,
                content: vec![MessageContent::text("new message 1")],
                metadata: MessageMetadata::default(),
            },
            Message {
                id: Some("new-2".to_string()),
                role: Role::Assistant,
                created: 3000,
                content: vec![MessageContent::text("new message 2")],
                metadata: MessageMetadata::default(),
            },
        ];
        let new_conv = Conversation::new_unvalidated(new_messages);

        storage
            .replace_conversation(&session.id, &new_conv)
            .await
            .unwrap();

        let retrieved = storage.get_session(&session.id, true).await.unwrap();
        assert_eq!(retrieved.message_count, 2);
        let msgs = retrieved.conversation.unwrap().messages().to_vec();
        assert_eq!(msgs[0].id.as_deref(), Some("new-1"));
        assert_eq!(msgs[1].id.as_deref(), Some("new-2"));

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_truncate_conversation() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let session = storage
            .create_session(PathBuf::from("/tmp"), "Test".to_string(), SessionType::User)
            .await
            .unwrap();

        for i in 0..5 {
            storage
                .add_message(
                    &session.id,
                    &Message {
                        id: Some(format!("msg-{}", i)),
                        role: Role::User,
                        created: (i + 1) * 1000,
                        content: vec![MessageContent::text(format!("Message {}", i))],
                        metadata: MessageMetadata::default(),
                    },
                )
                .await
                .unwrap();
        }

        // Truncate from timestamp 3000 onwards (should remove messages with created >= 3000)
        storage
            .truncate_conversation(&session.id, 3000)
            .await
            .unwrap();

        let retrieved = storage.get_session(&session.id, true).await.unwrap();
        assert_eq!(retrieved.message_count, 2);

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_message_metadata() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let session = storage
            .create_session(PathBuf::from("/tmp"), "Test".to_string(), SessionType::User)
            .await
            .unwrap();

        let msg = Message {
            id: Some("msg-meta".to_string()),
            role: Role::User,
            created: 1000,
            content: vec![MessageContent::text("Hello")],
            metadata: MessageMetadata::default(),
        };

        storage.add_message(&session.id, &msg).await.unwrap();

        // Default metadata
        let metadata = storage
            .get_message_metadata(&session.id, "msg-meta")
            .await
            .unwrap();
        assert!(metadata.user_visible);
        assert!(metadata.agent_visible);

        // Update to agent-only
        let new_metadata = MessageMetadata::agent_only();
        storage
            .set_message_metadata(&session.id, "msg-meta", new_metadata)
            .await
            .unwrap();

        let updated = storage
            .get_message_metadata(&session.id, "msg-meta")
            .await
            .unwrap();
        assert!(!updated.user_visible);
        assert!(updated.agent_visible);

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_get_insights() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        // Empty database
        let insights = storage.get_insights().await.unwrap();
        assert_eq!(insights.total_sessions, 0);
        assert_eq!(insights.total_tokens, 0);

        // Add sessions with tokens
        let s1 = storage
            .create_session(PathBuf::from("/tmp"), "S1".to_string(), SessionType::User)
            .await
            .unwrap();
        let s2 = storage
            .create_session(PathBuf::from("/tmp"), "S2".to_string(), SessionType::User)
            .await
            .unwrap();

        storage
            .apply_update(
                &s1.id,
                SessionUpdate {
                    total_tokens: Some(Some(100)),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        storage
            .apply_update(
                &s2.id,
                SessionUpdate {
                    accumulated_total_tokens: Some(Some(200)),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let insights = storage.get_insights().await.unwrap();
        assert_eq!(insights.total_sessions, 2);
        // s1 has total_tokens=100 (no accumulated), s2 has accumulated=200 (takes priority)
        assert_eq!(insights.total_tokens, 300);

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_concurrent_session_creation() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let storage = Arc::new(storage);
        let mut handles: Vec<tokio::task::JoinHandle<String>> = vec![];

        for i in 0..10_i32 {
            let storage = Arc::clone(&storage);
            handles.push(tokio::spawn(async move {
                let session: Session = storage
                    .create_session(
                        PathBuf::from(format!("/tmp/test_{}", i)),
                        format!("Session {}", i),
                        SessionType::User,
                    )
                    .await
                    .unwrap();

                storage
                    .add_message(
                        &session.id,
                        &Message {
                            id: None,
                            role: Role::User,
                            created: chrono::Utc::now().timestamp_millis(),
                            content: vec![MessageContent::text(format!("Hello from {}", i))],
                            metadata: MessageMetadata::default(),
                        },
                    )
                    .await
                    .unwrap();

                session.id
            }));
        }

        let mut ids: Vec<String> = Vec::new();
        for handle in handles {
            ids.push(handle.await.unwrap());
        }

        // All IDs should be unique
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 10);

        // All sessions should be listable
        let sessions: Vec<Session> = storage
            .list_sessions_by_types(&[SessionType::User])
            .await
            .unwrap();
        assert_eq!(sessions.len(), 10);

        // Use Arc::try_unwrap to get owned storage for teardown
        let storage = Arc::try_unwrap(storage).unwrap_or_else(|_| panic!("Arc still shared"));
        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_search_chat_history() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let session = storage
            .create_session(
                PathBuf::from("/tmp/search"),
                "Search Test".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        storage
            .add_message(
                &session.id,
                &Message {
                    id: Some("search-1".to_string()),
                    role: Role::User,
                    created: 1000,
                    content: vec![MessageContent::text(
                        "How do I configure kubernetes deployments?",
                    )],
                    metadata: MessageMetadata::default(),
                },
            )
            .await
            .unwrap();

        storage
            .add_message(
                &session.id,
                &Message {
                    id: Some("search-2".to_string()),
                    role: Role::Assistant,
                    created: 2000,
                    content: vec![MessageContent::text(
                        "You can use kubectl apply to configure deployments.",
                    )],
                    metadata: MessageMetadata::default(),
                },
            )
            .await
            .unwrap();

        // Search for "kubernetes"
        let results = storage
            .search_chat_history("kubernetes", Some(10), None, None, None)
            .await
            .unwrap();

        assert!(results.total_matches > 0);
        assert_eq!(results.results[0].session_id, session.id);

        // Search with exclude
        let results = storage
            .search_chat_history(
                "kubernetes",
                Some(10),
                None,
                None,
                Some(session.id.clone()),
            )
            .await
            .unwrap();
        assert_eq!(results.total_matches, 0);

        teardown(&storage).await;
    }

    #[tokio::test]
    async fn test_document_matches_export_format() {
        let Some(storage) = setup_test_storage().await else {
            eprintln!("Skipping: GOOSE_MONGODB_URI not set");
            return;
        };

        let session = storage
            .create_session(
                PathBuf::from("/tmp/export"),
                "Export Test".to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        storage
            .add_message(
                &session.id,
                &Message {
                    id: Some("export-msg-1".to_string()),
                    role: Role::User,
                    created: 1000,
                    content: vec![MessageContent::text("Hello")],
                    metadata: MessageMetadata::default(),
                },
            )
            .await
            .unwrap();

        // Read the raw message document from MongoDB
        let msg_doc = storage
            .messages
            .find_one(doc! { "id": "export-msg-1" }, None)
            .await
            .unwrap()
            .unwrap();

        // Verify field names match Message struct serde names
        assert!(msg_doc.get("id").is_some(), "should have 'id' field");
        assert!(msg_doc.get("role").is_some(), "should have 'role' field");
        assert!(msg_doc.get("created").is_some(), "should have 'created' field");
        assert!(msg_doc.get("content").is_some(), "should have 'content' field");
        assert!(msg_doc.get("metadata").is_some(), "should have 'metadata' field");

        // content should be a BSON array, not a string
        assert!(msg_doc.get_array("content").is_ok(), "content should be an array");

        // metadata should be a BSON document, not a string
        assert!(
            msg_doc.get_document("metadata").is_ok(),
            "metadata should be a document"
        );

        // Read the raw session document
        let sess_doc = storage
            .sessions
            .find_one(doc! { "id": &session.id }, None)
            .await
            .unwrap()
            .unwrap();

        // extension_data should be a BSON document, not a string
        assert!(
            sess_doc.get_document("extension_data").is_ok(),
            "extension_data should be a document"
        );

        teardown(&storage).await;
    }
}
