//! GraphRAG-lite knowledge extraction from conversation history.
//!
//! Extracts structured entities and relations from conversation text using an LLM
//! and a knowledge extraction prompt template. The output can be fed into a
//! knowledge graph (MCP memory server) for persistent, queryable memory.

use crate::providers::base::Provider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// A knowledge graph entity extracted from conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KgEntity {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub name: String,
    pub description: String,
    pub confidence: f64,
}

/// A relation between two entities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KgRelation {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub description: String,
}

/// The result of a knowledge extraction.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KnowledgeGraph {
    pub entities: Vec<KgEntity>,
    pub relations: Vec<KgRelation>,
}

impl KnowledgeGraph {
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.relations.is_empty()
    }

    /// Merge another graph into this one, deduplicating by entity name.
    pub fn merge(&mut self, other: KnowledgeGraph) {
        let existing: std::collections::HashSet<String> =
            self.entities.iter().map(|e| e.name.clone()).collect();

        for entity in other.entities {
            if !existing.contains(&entity.name) {
                self.entities.push(entity);
            }
        }

        for relation in other.relations {
            let key = format!(
                "{}-{}-{}",
                relation.source, relation.relation, relation.target
            );
            let already = self
                .relations
                .iter()
                .any(|r| format!("{}-{}-{}", r.source, r.relation, r.target) == key);
            if !already {
                self.relations.push(relation);
            }
        }
    }

    /// Filter to only high-confidence entities.
    pub fn high_confidence(&self, threshold: f64) -> KnowledgeGraph {
        let entities: Vec<KgEntity> = self
            .entities
            .iter()
            .filter(|e| e.confidence >= threshold)
            .cloned()
            .collect();
        let entity_names: std::collections::HashSet<&str> =
            entities.iter().map(|e| e.name.as_str()).collect();
        let relations: Vec<KgRelation> = self
            .relations
            .iter()
            .filter(|r| {
                entity_names.contains(r.source.as_str()) || entity_names.contains(r.target.as_str())
            })
            .cloned()
            .collect();
        KnowledgeGraph {
            entities,
            relations,
        }
    }
}

/// Extract knowledge from conversation text using an LLM.
pub async fn extract_knowledge(
    provider: &Arc<dyn Provider>,
    session_id: &str,
    conversation_text: &str,
) -> anyhow::Result<KnowledgeGraph> {
    let mut context = HashMap::new();
    context.insert(
        "conversation_text".to_string(),
        conversation_text.to_string(),
    );

    let prompt_text = crate::prompt_template::render_template("knowledge_extraction.md", &context)?;

    let user_msg = crate::conversation::message::Message::user().with_text(&prompt_text);

    let system = "You are a knowledge extraction engine. Output valid JSON only.".to_string();

    let (message, _usage) = provider
        .complete(session_id, &system, &[user_msg], &[])
        .await?;

    let text = message
        .content
        .iter()
        .filter_map(|c| match c {
            crate::conversation::message::MessageContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    parse_knowledge_graph(&text)
}

/// Parse a knowledge graph from JSON text (with optional markdown fencing).
pub fn parse_knowledge_graph(text: &str) -> anyhow::Result<KnowledgeGraph> {
    let json_str = extract_json_block(text);
    let kg: KnowledgeGraph = serde_json::from_str(json_str)?;

    // Validate: cap at 20 entities, 20 relations
    let entities = kg.entities.into_iter().take(20).collect();
    let relations = kg.relations.into_iter().take(20).collect();

    Ok(KnowledgeGraph {
        entities,
        relations,
    })
}

/// Extract JSON from text, handling markdown code fences.
#[allow(clippy::string_slice)] // All indexed chars are ASCII (backticks, braces)
fn extract_json_block(text: &str) -> &str {
    let trimmed = text.trim();

    // Try ```json ... ``` blocks
    if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
    }

    // Try ``` ... ``` blocks
    if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
    }

    // Try raw { ... } JSON
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return &trimmed[start..=end];
        }
    }

    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_knowledge_graph_basic() {
        let json = r#"{
            "entities": [
                {"type": "Component", "name": "semantic_router", "description": "TF-IDF router", "confidence": 0.9}
            ],
            "relations": [
                {"source": "semantic_router", "target": "intent_router", "relation": "implements", "description": "Layer 2"}
            ]
        }"#;

        let kg = parse_knowledge_graph(json).unwrap();
        assert_eq!(kg.entities.len(), 1);
        assert_eq!(kg.relations.len(), 1);
        assert_eq!(kg.entities[0].name, "semantic_router");
        assert_eq!(kg.relations[0].relation, "implements");
    }

    #[test]
    fn test_parse_with_markdown_fence() {
        let text =
            "Here's the knowledge:\n```json\n{\"entities\": [], \"relations\": []}\n```\nDone.";
        let kg = parse_knowledge_graph(text).unwrap();
        assert!(kg.is_empty());
    }

    #[test]
    fn test_parse_with_bare_fence() {
        let text = "```\n{\"entities\": [{\"type\": \"Concept\", \"name\": \"A2A\", \"description\": \"Agent-to-Agent protocol\", \"confidence\": 0.8}], \"relations\": []}\n```";
        let kg = parse_knowledge_graph(text).unwrap();
        assert_eq!(kg.entities.len(), 1);
        assert_eq!(kg.entities[0].name, "A2A");
    }

    #[test]
    fn test_merge_deduplicates() {
        let mut kg1 = KnowledgeGraph {
            entities: vec![KgEntity {
                entity_type: "Component".into(),
                name: "router".into(),
                description: "Routes stuff".into(),
                confidence: 0.9,
            }],
            relations: vec![],
        };

        let kg2 = KnowledgeGraph {
            entities: vec![
                KgEntity {
                    entity_type: "Component".into(),
                    name: "router".into(),
                    description: "Duplicate".into(),
                    confidence: 0.8,
                },
                KgEntity {
                    entity_type: "Concept".into(),
                    name: "tfidf".into(),
                    description: "Term frequency".into(),
                    confidence: 0.7,
                },
            ],
            relations: vec![],
        };

        kg1.merge(kg2);
        assert_eq!(kg1.entities.len(), 2); // router + tfidf (no duplicate)
    }

    #[test]
    fn test_high_confidence_filter() {
        let kg = KnowledgeGraph {
            entities: vec![
                KgEntity {
                    entity_type: "Component".into(),
                    name: "strong".into(),
                    description: "High conf".into(),
                    confidence: 0.9,
                },
                KgEntity {
                    entity_type: "Concept".into(),
                    name: "weak".into(),
                    description: "Low conf".into(),
                    confidence: 0.3,
                },
            ],
            relations: vec![KgRelation {
                source: "strong".into(),
                target: "weak".into(),
                relation: "related_to".into(),
                description: "test".into(),
            }],
        };

        let filtered = kg.high_confidence(0.5);
        assert_eq!(filtered.entities.len(), 1);
        assert_eq!(filtered.entities[0].name, "strong");
        assert_eq!(filtered.relations.len(), 1); // kept because source is high-conf
    }

    #[test]
    fn test_caps_at_20_entities() {
        let entities: Vec<KgEntity> = (0..30)
            .map(|i| KgEntity {
                entity_type: "Concept".into(),
                name: format!("entity_{i}"),
                description: format!("Entity {i}"),
                confidence: 0.5,
            })
            .collect();

        let json = serde_json::json!({
            "entities": entities,
            "relations": []
        })
        .to_string();

        let kg = parse_knowledge_graph(&json).unwrap();
        assert_eq!(kg.entities.len(), 20);
    }

    #[test]
    fn test_extract_json_block_raw() {
        let text = "some prefix {\"entities\": [], \"relations\": []} some suffix";
        let block = extract_json_block(text);
        assert!(block.starts_with('{'));
        assert!(block.ends_with('}'));
    }
}
