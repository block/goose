//! SQL triggers that keep the denormalized `message_count` /
//! `last_message_timestamp` columns on `sessions` in sync with the
//! `messages` table.
//!
//! Maintaining the columns at the SQL layer means every writer stays
//! consistent — including binaries built before migration 17 that know
//! nothing about the columns — so a downgrade followed by an upgrade can
//! never leave stale counters behind.
//!
//! Visibility mirrors the pre-migration query semantics: rows with NULL,
//! missing, or malformed metadata JSON count as user-visible
//! (`json_valid` guards against corrupt rows instead of erroring).

/// Incremental maintenance on insert: one indexed UPDATE on the session row.
const INSERT_TRIGGER: &str = r#"
CREATE TRIGGER IF NOT EXISTS trg_messages_insert_counters
AFTER INSERT ON messages
BEGIN
    UPDATE sessions SET
        message_count = message_count +
            (CASE WHEN COALESCE(CASE WHEN json_valid(NEW.metadata_json)
                                     THEN json_extract(NEW.metadata_json, '$.userVisible') END, 1) != 0
                  THEN 1 ELSE 0 END),
        last_message_timestamp = MAX(
            COALESCE(last_message_timestamp, 0),
            CASE WHEN NEW.created_timestamp > 10000000000 THEN NEW.created_timestamp / 1000 ELSE NEW.created_timestamp END
        )
    WHERE id = NEW.session_id;
END
"#;

/// Full recompute for the one affected session (MAX cannot be decremented
/// incrementally; the aggregate reads via idx_messages_session_created).
const DELETE_TRIGGER: &str = r#"
CREATE TRIGGER IF NOT EXISTS trg_messages_delete_counters
AFTER DELETE ON messages
BEGIN
    UPDATE sessions SET
        message_count = (SELECT COUNT(*) FROM messages m
                         WHERE m.session_id = OLD.session_id
                           AND COALESCE(CASE WHEN json_valid(m.metadata_json)
                                             THEN json_extract(m.metadata_json, '$.userVisible') END, 1) != 0),
        last_message_timestamp = (SELECT MAX(CASE WHEN m.created_timestamp > 10000000000
                                                  THEN m.created_timestamp / 1000
                                                  ELSE m.created_timestamp END)
                                  FROM messages m
                                  WHERE m.session_id = OLD.session_id)
    WHERE id = OLD.session_id;
END
"#;

/// Visibility flips (metadata_json) and timestamp rewrites (created_timestamp)
/// can change both aggregates; recompute like the delete path.
const UPDATE_TRIGGER: &str = r#"
CREATE TRIGGER IF NOT EXISTS trg_messages_update_counters
AFTER UPDATE OF metadata_json, created_timestamp ON messages
BEGIN
    UPDATE sessions SET
        message_count = (SELECT COUNT(*) FROM messages m
                         WHERE m.session_id = OLD.session_id
                           AND COALESCE(CASE WHEN json_valid(m.metadata_json)
                                             THEN json_extract(m.metadata_json, '$.userVisible') END, 1) != 0),
        last_message_timestamp = (SELECT MAX(CASE WHEN m.created_timestamp > 10000000000
                                                  THEN m.created_timestamp / 1000
                                                  ELSE m.created_timestamp END)
                                  FROM messages m
                                  WHERE m.session_id = OLD.session_id)
    WHERE id = OLD.session_id;
END
"#;

pub(crate) const SESSION_COUNTER_TRIGGERS: [&str; 3] =
    [INSERT_TRIGGER, DELETE_TRIGGER, UPDATE_TRIGGER];
