use super::message::{
    PersistedChainSummary, ToolNameParts, ToolRequest, TOOL_META_CHAIN_SUMMARY_KEY,
    TOOL_META_EXTERNAL_DISPATCH_KEY, TOOL_META_TITLE_KEY,
};

impl<'a> From<&'a str> for ToolNameParts<'a> {
    fn from(name: &'a str) -> Self {
        match name.split_once("__") {
            Some((extension_name, tool_name)) => Self {
                extension_name: Some(extension_name),
                tool_name,
            },
            None => Self {
                extension_name: None,
                tool_name: name,
            },
        }
    }
}

impl ToolRequest {
    pub fn tool_name_parts(&self) -> Option<ToolNameParts<'_>> {
        let tool_call = self.tool_call.as_ref().ok()?;
        Some(ToolNameParts::from(tool_call.name.as_ref()))
    }

    pub fn to_readable_string(&self) -> String {
        match &self.tool_call {
            Ok(tool_call) => {
                format!(
                    "Tool: {}, Args: {}",
                    tool_call.name,
                    serde_json::to_string_pretty(&tool_call.arguments)
                        .unwrap_or_else(|_| "<<invalid json>>".to_string())
                )
            }
            Err(e) => format!("Invalid tool call: {}", e),
        }
    }

    /// Returns true if this tool request was already executed externally
    /// (e.g. by an ACP provider's underlying SDK) and the agent loop must
    /// not redispatch it. See [`TOOL_META_EXTERNAL_DISPATCH_KEY`].
    pub fn is_externally_dispatched(&self) -> bool {
        self.tool_meta
            .as_ref()
            .and_then(|v| v.get(TOOL_META_EXTERNAL_DISPATCH_KEY))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Returns the persisted LLM-generated title for this tool call, if any.
    /// Set asynchronously by [`crate::acp::server`] after `provider.complete_fast`
    /// resolves; survives session reload via SQLite. Falls back to `None` for
    /// older sessions that predate persistence — callers should use a deterministic
    /// title in that case.
    pub fn persisted_title(&self) -> Option<&str> {
        self.tool_meta
            .as_ref()
            .and_then(|v| v.get(TOOL_META_TITLE_KEY))
            .and_then(|v| v.as_str())
    }

    /// Returns the persisted per-chain summary anchored on this tool request,
    /// if any. Only the FIRST tool request in a chain (a run of consecutive
    /// tool blocks within one assistant message) carries this. See
    /// [`crate::acp::server`] for how chains are detected and summarized.
    pub fn persisted_chain_summary(&self) -> Option<PersistedChainSummary> {
        let obj = self
            .tool_meta
            .as_ref()
            .and_then(|v| v.get(TOOL_META_CHAIN_SUMMARY_KEY))?;
        let summary = obj.get("summary").and_then(|v| v.as_str())?.to_string();
        let count = obj.get("count").and_then(|v| v.as_u64())?;
        if count == 0 {
            return None;
        }
        Some(PersistedChainSummary {
            summary,
            count: count as usize,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::CallToolRequestParams;

    fn make_tool_request(meta: Option<serde_json::Value>) -> ToolRequest {
        ToolRequest {
            id: "id-1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("test_tool")),
            metadata: None,
            tool_meta: meta,
        }
    }

    mod tool_name_parts {
        use super::*;
        use rmcp::model::ErrorData;

        fn make_named_tool_request(name: &str) -> ToolRequest {
            ToolRequest {
                id: "id-1".to_string(),
                tool_call: Ok(CallToolRequestParams::new(name.to_string())),
                metadata: None,
                tool_meta: None,
            }
        }

        #[test]
        fn splits_prefixed_name() {
            let request = make_named_tool_request("developer__shell");

            assert_eq!(
                request.tool_name_parts(),
                Some(ToolNameParts {
                    extension_name: Some("developer"),
                    tool_name: "shell",
                })
            );
        }

        #[test]
        fn preserves_unprefixed_name() {
            let request = make_named_tool_request("read");

            assert_eq!(
                request.tool_name_parts(),
                Some(ToolNameParts {
                    extension_name: None,
                    tool_name: "read",
                })
            );
        }

        #[test]
        fn splits_at_first_separator() {
            let request = make_named_tool_request("calendar__events__list");

            assert_eq!(
                request.tool_name_parts(),
                Some(ToolNameParts {
                    extension_name: Some("calendar"),
                    tool_name: "events__list",
                })
            );
        }

        #[test]
        fn returns_none_for_invalid_call() {
            let request = ToolRequest {
                id: "id-1".to_string(),
                tool_call: Err(ErrorData::invalid_request("invalid tool call", None)),
                metadata: None,
                tool_meta: None,
            };

            assert_eq!(request.tool_name_parts(), None);
        }
    }

    mod persisted_title {
        use super::*;

        #[test]
        fn returns_none_when_meta_missing() {
            let req = make_tool_request(None);
            assert_eq!(req.persisted_title(), None);
        }

        #[test]
        fn returns_value_when_present() {
            let meta = serde_json::json!({
                TOOL_META_TITLE_KEY: "reading project configuration",
            });
            let req = make_tool_request(Some(meta));
            assert_eq!(req.persisted_title(), Some("reading project configuration"));
        }

        #[test]
        fn returns_none_for_non_string_value() {
            let meta = serde_json::json!({ TOOL_META_TITLE_KEY: 42 });
            let req = make_tool_request(Some(meta));
            assert_eq!(req.persisted_title(), None);
        }

        #[test]
        fn does_not_collide_with_external_dispatch() {
            let meta = serde_json::json!({
                TOOL_META_EXTERNAL_DISPATCH_KEY: true,
                TOOL_META_TITLE_KEY: "running commands",
            });
            let req = make_tool_request(Some(meta));
            assert!(req.is_externally_dispatched());
            assert_eq!(req.persisted_title(), Some("running commands"));
        }
    }

    mod persisted_chain_summary {
        use super::*;

        #[test]
        fn round_trips() {
            let meta = serde_json::json!({
                TOOL_META_CHAIN_SUMMARY_KEY: {
                    "summary": "applied dark mode polish",
                    "count": 4,
                },
            });
            let req = make_tool_request(Some(meta));
            let summary = req.persisted_chain_summary().expect("summary present");
            assert_eq!(summary.summary, "applied dark mode polish");
            assert_eq!(summary.count, 4);
        }

        #[test]
        fn returns_none_for_missing_or_zero_count() {
            let req = make_tool_request(None);
            assert!(req.persisted_chain_summary().is_none());

            let meta_zero = serde_json::json!({
                TOOL_META_CHAIN_SUMMARY_KEY: { "summary": "x", "count": 0 },
            });
            let req_zero = make_tool_request(Some(meta_zero));
            assert!(req_zero.persisted_chain_summary().is_none());

            let meta_no_summary = serde_json::json!({
                TOOL_META_CHAIN_SUMMARY_KEY: { "count": 3 },
            });
            let req_no_summary = make_tool_request(Some(meta_no_summary));
            assert!(req_no_summary.persisted_chain_summary().is_none());
        }
    }
}
