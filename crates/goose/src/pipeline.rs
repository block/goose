use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use utoipa::ToSchema;

use crate::config::paths::Paths;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Pipeline {
    #[serde(default = "default_api_version")]
    pub api_version: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub nodes: Vec<PipelineNode>,
    #[serde(default)]
    pub edges: Vec<PipelineEdge>,
    #[serde(default)]
    pub layout: Option<PipelineLayout>,
}

fn default_api_version() -> String {
    "goose/v1".to_string()
}

fn default_kind() -> String {
    "Pipeline".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PipelineNode {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub position: Option<NodePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Trigger,
    Agent,
    Tool,
    Condition,
    Transform,
    Human,
    A2a,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PipelineEdge {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PipelineLayout {
    #[serde(default)]
    pub viewport: Option<Viewport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PipelineManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub tags: Vec<String>,
    pub node_count: usize,
    pub edge_count: usize,
    pub file_path: String,
    pub last_modified: String,
}

impl Pipeline {
    pub fn from_yaml(content: &str) -> Result<Self> {
        serde_yaml::from_str(content).map_err(|e| anyhow!("Failed to parse pipeline YAML: {}", e))
    }

    pub fn from_json(content: &str) -> Result<Self> {
        serde_json::from_str(content).map_err(|e| anyhow!("Failed to parse pipeline JSON: {}", e))
    }

    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self)
            .map_err(|e| anyhow!("Failed to serialize pipeline to YAML: {}", e))
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to serialize pipeline to JSON: {}", e))
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        match path.extension().and_then(|e| e.to_str()) {
            Some("json") => Self::from_json(&content),
            _ => Self::from_yaml(&content),
        }
    }

    pub fn validate(&self) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        if self.name.is_empty() {
            return Err(anyhow!("Pipeline name is required"));
        }
        if self.nodes.is_empty() {
            return Err(anyhow!("Pipeline must have at least one node"));
        }

        let node_ids: Vec<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();
        let has_trigger = self
            .nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Trigger));
        if !has_trigger {
            warnings.push("Pipeline has no trigger node — it won't auto-start".to_string());
        }

        for edge in &self.edges {
            if !node_ids.contains(&edge.source.as_str()) {
                return Err(anyhow!(
                    "Edge source '{}' references unknown node",
                    edge.source
                ));
            }
            if !node_ids.contains(&edge.target.as_str()) {
                return Err(anyhow!(
                    "Edge target '{}' references unknown node",
                    edge.target
                ));
            }
        }

        // Check for cycles (simple DFS)
        if self.has_cycle() {
            return Err(anyhow!("Pipeline contains a cycle — DAGs must be acyclic"));
        }

        Ok(warnings)
    }

    fn has_cycle(&self) -> bool {
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.nodes {
            adj.entry(node.id.as_str()).or_default();
        }
        for edge in &self.edges {
            adj.entry(edge.source.as_str())
                .or_default()
                .push(edge.target.as_str());
        }

        let mut visited: HashMap<&str, u8> = HashMap::new(); // 0=unvisited, 1=in-stack, 2=done
        for node in &self.nodes {
            if self.dfs_cycle(node.id.as_str(), &adj, &mut visited) {
                return true;
            }
        }
        false
    }

    fn dfs_cycle<'a>(
        &self,
        node: &'a str,
        adj: &HashMap<&'a str, Vec<&'a str>>,
        visited: &mut HashMap<&'a str, u8>,
    ) -> bool {
        match visited.get(node) {
            Some(1) => return true,  // back edge = cycle
            Some(2) => return false, // already done
            _ => {}
        }
        visited.insert(node, 1);
        if let Some(neighbors) = adj.get(node) {
            for &next in neighbors {
                if self.dfs_cycle(next, adj, visited) {
                    return true;
                }
            }
        }
        visited.insert(node, 2);
        false
    }
}

pub fn get_pipeline_dir() -> PathBuf {
    Paths::config_dir().join("pipelines")
}

pub fn list_pipelines() -> Result<Vec<PipelineManifest>> {
    let dir = get_pipeline_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut manifests = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == "yaml" || ext == "yml" || ext == "json" {
                    match Pipeline::from_file(&path) {
                        Ok(pipeline) => {
                            let metadata = fs::metadata(&path)?;
                            let modified = metadata
                                .modified()
                                .ok()
                                .and_then(|t| {
                                    t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| {
                                        chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                                            .map(|dt| dt.to_rfc3339())
                                            .unwrap_or_default()
                                    })
                                })
                                .unwrap_or_default();

                            let id = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();

                            manifests.push(PipelineManifest {
                                id,
                                name: pipeline.name,
                                description: pipeline.description,
                                version: pipeline.version,
                                tags: pipeline.tags,
                                node_count: pipeline.nodes.len(),
                                edge_count: pipeline.edges.len(),
                                file_path: path.to_string_lossy().to_string(),
                                last_modified: modified,
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to load pipeline {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    manifests.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    Ok(manifests)
}

pub fn save_pipeline(pipeline: &Pipeline, file_path: Option<PathBuf>) -> Result<PathBuf> {
    let dir = get_pipeline_dir();
    fs::create_dir_all(&dir)?;

    let path = match file_path {
        Some(p) => p,
        None => generate_pipeline_filename(&pipeline.name, &dir),
    };

    let yaml = pipeline.to_yaml()?;
    fs::write(&path, yaml)?;
    Ok(path)
}

pub fn load_pipeline(id: &str) -> Result<(Pipeline, PathBuf)> {
    let dir = get_pipeline_dir();
    for ext in &["yaml", "yml", "json"] {
        let path = dir.join(format!("{}.{}", id, ext));
        if path.exists() {
            let pipeline = Pipeline::from_file(&path)?;
            return Ok((pipeline, path));
        }
    }
    Err(anyhow!("Pipeline '{}' not found", id))
}

pub fn delete_pipeline(id: &str) -> Result<()> {
    let dir = get_pipeline_dir();
    for ext in &["yaml", "yml", "json"] {
        let path = dir.join(format!("{}.{}", id, ext));
        if path.exists() {
            fs::remove_file(&path)?;
            return Ok(());
        }
    }
    Err(anyhow!("Pipeline '{}' not found", id))
}

fn generate_pipeline_filename(name: &str, dir: &Path) -> PathBuf {
    let base = name
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("-");

    let base = if base.is_empty() {
        "untitled-pipeline".to_string()
    } else {
        base
    };

    let mut candidate = dir.join(format!("{}.yaml", base));
    if !candidate.exists() {
        return candidate;
    }

    let mut counter = 1;
    loop {
        candidate = dir.join(format!("{}-{}.yaml", base, counter));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_yaml_roundtrip() {
        let yaml = r#"
apiVersion: goose/v1
kind: Pipeline
name: test-pipeline
description: A test pipeline
nodes:
  - id: trigger-1
    kind: trigger
    label: Start
    config:
      type: manual
  - id: agent-1
    kind: agent
    label: Code Review
    config:
      persona: code-reviewer
edges:
  - source: trigger-1
    target: agent-1
"#;
        let pipeline = Pipeline::from_yaml(yaml).unwrap();
        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.nodes.len(), 2);
        assert_eq!(pipeline.edges.len(), 1);

        let yaml_out = pipeline.to_yaml().unwrap();
        let pipeline2 = Pipeline::from_yaml(&yaml_out).unwrap();
        assert_eq!(pipeline2.name, pipeline.name);
        assert_eq!(pipeline2.nodes.len(), pipeline.nodes.len());
    }

    #[test]
    fn test_pipeline_validation() {
        let pipeline = Pipeline {
            api_version: "goose/v1".to_string(),
            kind: "Pipeline".to_string(),
            name: "valid".to_string(),
            description: String::new(),
            version: "1.0".to_string(),
            tags: vec![],
            nodes: vec![
                PipelineNode {
                    id: "t1".to_string(),
                    kind: NodeKind::Trigger,
                    label: "Start".to_string(),
                    config: HashMap::new(),
                    condition: None,
                    position: None,
                },
                PipelineNode {
                    id: "a1".to_string(),
                    kind: NodeKind::Agent,
                    label: "Agent".to_string(),
                    config: HashMap::new(),
                    condition: None,
                    position: None,
                },
            ],
            edges: vec![PipelineEdge {
                source: "t1".to_string(),
                target: "a1".to_string(),
                label: None,
                condition: None,
            }],
            layout: None,
        };

        let warnings = pipeline.validate().unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_pipeline_cycle_detection() {
        let pipeline = Pipeline {
            api_version: "goose/v1".to_string(),
            kind: "Pipeline".to_string(),
            name: "cyclic".to_string(),
            description: String::new(),
            version: "1.0".to_string(),
            tags: vec![],
            nodes: vec![
                PipelineNode {
                    id: "a".to_string(),
                    kind: NodeKind::Agent,
                    label: "A".to_string(),
                    config: HashMap::new(),
                    condition: None,
                    position: None,
                },
                PipelineNode {
                    id: "b".to_string(),
                    kind: NodeKind::Agent,
                    label: "B".to_string(),
                    config: HashMap::new(),
                    condition: None,
                    position: None,
                },
            ],
            edges: vec![
                PipelineEdge {
                    source: "a".to_string(),
                    target: "b".to_string(),
                    label: None,
                    condition: None,
                },
                PipelineEdge {
                    source: "b".to_string(),
                    target: "a".to_string(),
                    label: None,
                    condition: None,
                },
            ],
            layout: None,
        };

        let result = pipeline.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cycle"));
    }
}
