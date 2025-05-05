use crate::agents::extension::ExtensionConfig;
use crate::agents::extension_manager::ExtensionManager;
use crate::agents::platform_tools;

use etcetera::{choose_app_strategy, AppStrategy};
use mcp_core::tool::Tool;
use std::fs::OpenOptions; // TODO: remove this
use std::io::Write;
use std::path::PathBuf;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::TantivyDocument;
use tantivy::schema::*;
use tantivy::{Index, ReloadPolicy, Term};
use uuid::Uuid; // TODO: remove this

pub struct ToolRouterV2 {
    index: Index,
    reader: tantivy::IndexReader,
    schema: Schema,
    name_field: Field,
    description_field: Field,
    extension_name_field: Field,
    _index_dir: PathBuf, // Keep the path alive for the lifetime of the index
}

impl ToolRouterV2 {
    pub fn new() -> tantivy::Result<Self> {
        // Create schema
        let schema = Self::build_schema();
        let name_field = schema.get_field("name").unwrap();
        let description_field = schema.get_field("description").unwrap();
        let extension_name_field = schema.get_field("extension_name").unwrap();

        // Create persistent index directory in goose config
        // - macOS/Linux: ~/.config/goose/toolrouter/
        // - Windows:     ~\AppData\Roaming\Block\goose\config\toolrouter\
        let mut index_dir = choose_app_strategy(crate::config::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_config_dir("toolrouter"))
            .unwrap_or_else(|_| PathBuf::from(".config/goose/toolrouter"));

        // Append a random UUID to create a unique subdirectory
        let unique_dir = Uuid::new_v4().to_string();
        index_dir.push(unique_dir);

        // Ensure the directory exists
        std::fs::create_dir_all(&index_dir).expect("Failed to create toolrouter directory");

        // Create index in the persistent directory
        let index = Index::create_in_dir(&index_dir, schema.clone())?;

        // Create reader
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            schema,
            name_field,
            description_field,
            extension_name_field,
            _index_dir: index_dir,
        })
    }

    fn build_schema() -> Schema {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("name", TEXT | STORED);
        schema_builder.add_text_field("description", TEXT);
        schema_builder.add_text_field("extension_name", TEXT);
        schema_builder.build()
    }

    pub fn remove_document(&self, title: &str) -> tantivy::Result<()> {
        // TODO: check implementation, document not removed in time
        let mut writer = self.index.writer::<TantivyDocument>(50_000_000)?;
        let title_field = self.schema.get_field("title").unwrap();

        // Create a term for the title we want to delete
        let term = Term::from_field_text(title_field, title);

        // Delete all documents matching this term
        writer.delete_term(term);

        // Commit the changes
        writer.commit()?;

        // Reload the reader to reflect the changes
        self.reader.reload()?;

        Ok(())
    }

    pub async fn match_tools(
        &self,
        user_query: &str,
        _tools: &[Tool],
        top_k: usize,
    ) -> anyhow::Result<Vec<Tool>> {
        self.reader.reload()?; // Refresh index state
        let searcher = self.reader.searcher();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.extension_name_field,
                self.description_field,
                self.name_field,
            ],
        );
        // Return all tools if query is too long to avoid performance issues
        if user_query.chars().count() > 2000 {
            return Ok(_tools.to_vec());
        }
        let query = query_parser.parse_query(user_query)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(top_k))?;
        // Log top_docs to debug file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/goose_debug_v2.log")
            .unwrap();

        writeln!(file, "user_query: {:?}", user_query).unwrap();

        let mut matched_tools = Vec::new();

        // Always include platform tools if present in _tools
        let platform_tools = [
            platform_tools::search_available_extensions_tool(),
            platform_tools::manage_extensions_tool(),
            platform_tools::read_resource_tool(),
            platform_tools::list_resources_tool(),
        ];

        for platform_tool in platform_tools {
            if _tools.iter().any(|t| t.name == platform_tool.name) {
                matched_tools.push(platform_tool);
            }
        }

        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            writeln!(
                file,
                "retrieved_docs: {:?}  === score: {:?}",
                retrieved_doc.to_json(&self.schema),
                _score
            )
            .unwrap();

            // Get the tool name from the retrieved document
            if let Some(name_values) = retrieved_doc.get_first(self.name_field) {
                if let Some(tool_name) = name_values.as_str() {
                    // Find the matching tool from _tools
                    if let Some(tool) = _tools.iter().find(|t| t.name == tool_name) {
                        // Only append tools with a score >= 1
                        if _score >= 1.0 {
                            matched_tools.push(tool.clone());
                        }
                    }
                }
            }
        }
        writeln!(
            file,
            "original tool size: {} - matched_tools: {}",
            _tools.len(),
            matched_tools.len()
        )
        .unwrap();
        Ok(matched_tools)
    }

    pub async fn write_documents(
        &self,
        extension_manager: &ExtensionManager,
        extension: &ExtensionConfig,
    ) -> anyhow::Result<()> {
        let mut writer = self.index.writer::<TantivyDocument>(50_000_000)?;

        let extension_name = extension.name();
        let extension_tools = extension_manager
            .get_prefixed_tools(Some(extension_name.clone()))
            .await?;

        for tool in extension_tools {
            let mut doc = TantivyDocument::default();
            doc.add_text(self.name_field, tool.name);
            doc.add_text(self.description_field, tool.description);
            doc.add_text(self.extension_name_field, extension_name.clone());
            writer.add_document(doc)?;
        }
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }
}
