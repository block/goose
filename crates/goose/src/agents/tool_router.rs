use crate::agents::extension::ExtensionConfig;
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

pub struct ToolRouter {
    index: Index,
    reader: tantivy::IndexReader,
    schema: Schema,
    id_field: Field,
    description_field: Field,
    _index_dir: PathBuf, // Keep the path alive for the lifetime of the index
}

impl ToolRouter {
    pub fn new(tool_descriptions: Vec<(&str, &str)>) -> tantivy::Result<Self> {
        // Create schema
        let schema = Self::build_schema();
        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();

        // Create persistent index directory in goose config
        // - macOS/Linux: ~/.config/goose/toolrouter/
        // - Windows:     ~\AppData\Roaming\Block\goose\config\toolrouter\
        let mut index_dir = choose_app_strategy(crate::config::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_config_dir("toolrouter"))
            .unwrap_or_else(|_| PathBuf::from(".config/goose/toolrouter"));

        // Append a random UUID to create a unique subdirectory
        // let unique_dir = Uuid::new_v4().to_string();
        let unique_dir = "wendy_test";
        index_dir.push(unique_dir);

        // Ensure the directory exists
        std::fs::create_dir_all(&index_dir).expect("Failed to create toolrouter directory");

        // Create index in the persistent directory
        let index = Index::create_in_dir(&index_dir, schema.clone())?;

        // Write documents to index
        Self::write_documents(&index, &schema, &tool_descriptions)?;

        // Create reader
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            schema,
            id_field: title_field,
            description_field: body_field,
            _index_dir: index_dir,
        })
    }

    fn build_schema() -> Schema {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("body", TEXT);
        schema_builder.build()
    }

    fn write_documents(
        index: &Index,
        schema: &Schema,
        tool_descriptions: &[(&str, &str)],
    ) -> tantivy::Result<()> {
        let mut writer = index.writer::<TantivyDocument>(50_000_000)?;
        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();

        for (title, body) in tool_descriptions {
            let mut doc = TantivyDocument::default();
            doc.add_text(title_field, title);
            doc.add_text(body_field, body);
            writer.add_document(doc)?;
        }
        writer.commit()?;
        Ok(())
    }

    pub fn remove_document(&self, title: &str) -> tantivy::Result<()> {
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
        tools: &[Tool],
        top_k: usize,
    ) -> anyhow::Result<Vec<Tool>> {
        self.reader.reload()?; // Refresh index state
        let searcher = self.reader.searcher();

        let query_parser = QueryParser::for_index(&self.index, vec![self.description_field]);
        let query = query_parser.parse_query(user_query)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(top_k))?;
        // Log top_docs to debug file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/goose_debug2.log")
            .unwrap();
        writeln!(file, "top_docs: {:?}", &top_docs).unwrap();

        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            writeln!(file, "Retrieved doc: {:?}", retrieved_doc).unwrap();

            if let Some(id_val) = retrieved_doc.get_first(self.id_field) {
                writeln!(file, "ID value: {:?}", id_val).unwrap();

                if let Some(id_text) = id_val.as_str() {
                    writeln!(file, "ID text: {}", id_text).unwrap();

                    // Find matching tools in tools array using prefix matching
                    writeln!(
                        file,
                        "Available tools: {:?}",
                        tools.iter().map(|t| &t.name).collect::<Vec<_>>()
                    )
                    .unwrap();

                    // Create the prefix pattern (e.g., "developer__")
                    let prefix = format!("{}__", id_text);

                    // Find all tools that start with the prefix
                    let matching_tools: Vec<Tool> = tools
                        .iter()
                        .filter(|t| t.name.starts_with(&prefix))
                        .cloned()
                        .collect();

                    if !matching_tools.is_empty() {
                        writeln!(
                            file,
                            "Found matching tools: {}",
                            matching_tools
                                .iter()
                                .map(|t| t.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                        .unwrap();
                        results.extend(matching_tools);
                    } else {
                        writeln!(file, "No matching tools found for prefix: {}", prefix).unwrap();
                    }
                }
            } else {
                writeln!(file, "No ID field found in document").unwrap();
            }
        }

        Ok(results)
    }

    fn extension_to_document(schema: &Schema, extension: &ExtensionConfig) -> TantivyDocument {
        let mut doc = TantivyDocument::default();
        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();

        match extension {
            ExtensionConfig::Sse {
                name,
                uri,
                description,
                ..
            } => {
                doc.add_text(title_field, name);
                let body = format!(
                    "SSE Extension: {}\nURI: {}\nDescription: {}",
                    name,
                    uri,
                    description.as_deref().unwrap_or("No description")
                );
                doc.add_text(body_field, &body);
            }
            ExtensionConfig::Stdio {
                name,
                cmd,
                args,
                description,
                ..
            } => {
                doc.add_text(title_field, name);
                let body = format!(
                    "Stdio Extension: {}\nCommand: {}\nArgs: {}\nDescription: {}",
                    name,
                    cmd,
                    args.join(" "),
                    description.as_deref().unwrap_or("No description")
                );
                doc.add_text(body_field, &body);
            }
            ExtensionConfig::Builtin {
                name, display_name, ..
            } => {
                doc.add_text(title_field, name);
                let body = format!(
                    "Builtin Extension: {}\nDisplay Name: {}",
                    name,
                    display_name.as_deref().unwrap_or("No display name")
                );
                doc.add_text(body_field, &body);
            }
            ExtensionConfig::Frontend {
                name,
                tools,
                instructions,
                ..
            } => {
                doc.add_text(title_field, name);
                let body = format!(
                    "Frontend Extension: {}\nTools: {}\nInstructions: {}",
                    name,
                    tools.len(),
                    instructions.as_deref().unwrap_or("No instructions")
                );
                doc.add_text(body_field, &body);
            }
        }
        doc
    }

    pub async fn write_extension(&self, extension: &ExtensionConfig) -> tantivy::Result<()> {
        let mut writer = self.index.writer::<TantivyDocument>(50_000_000)?;
        let doc = Self::extension_to_document(&self.schema, extension);
        writer.add_document(doc)?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }
}
