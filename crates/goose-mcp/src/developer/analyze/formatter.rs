use crate::developer::analyze::types::{
    AnalysisMode, AnalysisResult, CallChain, EntryType, FocusedAnalysisData,
};
use crate::developer::lang;
use rmcp::model::{Content, Role};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

fn safe_truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

pub struct Formatter;

impl Formatter {
    pub fn format_results(output: String) -> Vec<Content> {
        vec![
            Content::text(output.clone()).with_audience(vec![Role::Assistant]),
            Content::text(output)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ]
    }

    /// Format analysis result based on mode
    pub fn format_analysis_result(
        path: &Path,
        result: &AnalysisResult,
        mode: &AnalysisMode,
    ) -> String {
        tracing::debug!("Formatting result for {:?} in {:?} mode", path, mode);

        match mode {
            AnalysisMode::Structure => Self::format_structure_overview(path, result),
            AnalysisMode::Semantic => Self::format_semantic_result(path, result),
            AnalysisMode::Focused => {
                // Focused mode is handled separately
                tracing::warn!("format_analysis_result called with Focused mode");
                String::new()
            }
        }
    }

    /// Format structure overview (compact format)
    pub fn format_structure_overview(path: &Path, result: &AnalysisResult) -> String {
        let mut output = String::new();

        // Format as: path [LOC, FUNCTIONS, CLASSES] <FLAGS>
        output.push_str(&format!("{} [{}L", path.display(), result.line_count));

        if result.function_count > 0 {
            output.push_str(&format!(", {}F", result.function_count));
        }

        if result.class_count > 0 {
            output.push_str(&format!(", {}C", result.class_count));
        }

        output.push(']');

        // Add FLAGS if any
        if let Some(main_line) = result.main_line {
            output.push_str(&format!(" main:{}", main_line));
        }

        output.push('\n');
        output
    }

    /// Format semantic analysis result (dense matrix format)
    pub fn format_semantic_result(path: &Path, result: &AnalysisResult) -> String {
        let mut output = format!(
            "FILE: {} [{}L, {}F, {}C]\n\n",
            path.display(),
            result.line_count,
            result.function_count,
            result.class_count
        );

        // Classes on single/multiple lines with colon-separated line numbers
        if !result.classes.is_empty() {
            output.push_str("C: ");
            let class_strs: Vec<String> = result
                .classes
                .iter()
                .map(|c| format!("{}:{}", c.name, c.line))
                .collect();
            output.push_str(&class_strs.join(" "));
            output.push_str("\n\n");
        }

        // Functions with call counts where significant
        if !result.functions.is_empty() {
            output.push_str("F: ");

            // Count how many times each function is called
            let mut call_counts: HashMap<String, usize> = HashMap::new();
            for call in &result.calls {
                *call_counts.entry(call.callee_name.clone()).or_insert(0) += 1;
            }

            let func_strs: Vec<String> = result
                .functions
                .iter()
                .map(|f| {
                    let count = call_counts.get(&f.name).unwrap_or(&0);
                    if *count > 3 {
                        format!("{}:{}•{}", f.name, f.line, count)
                    } else {
                        format!("{}:{}", f.name, f.line)
                    }
                })
                .collect();

            // Format functions, wrapping at reasonable line length
            let mut line_len = 3; // "F: "
            for (i, func_str) in func_strs.iter().enumerate() {
                if i > 0 && line_len + func_str.len() + 1 > 100 {
                    output.push_str("\n   ");
                    line_len = 3;
                }
                if i > 0 {
                    output.push(' ');
                    line_len += 1;
                }
                output.push_str(func_str);
                line_len += func_str.len();
            }
            output.push_str("\n\n");
        }

        // Condensed imports
        if !result.imports.is_empty() {
            output.push_str("I: ");

            // Group imports by module/package
            let mut grouped_imports: HashMap<String, Vec<String>> = HashMap::new();
            for import in &result.imports {
                // Simple heuristic: first word/module is the group
                let group = if import.starts_with("use ") {
                    import.split("::").next().unwrap_or("use").to_string()
                } else if import.starts_with("import ") {
                    import
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("import")
                        .to_string()
                } else if import.starts_with("from ") {
                    import
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("from")
                        .to_string()
                } else {
                    import.split_whitespace().next().unwrap_or("").to_string()
                };
                grouped_imports
                    .entry(group)
                    .or_default()
                    .push(import.clone());
            }

            // Show condensed import summary
            let import_summary: Vec<String> = grouped_imports
                .iter()
                .map(|(group, imports)| {
                    if imports.len() > 1 {
                        format!("{}({})", group, imports.len())
                    } else {
                        safe_truncate(&imports[0], 40)
                    }
                })
                .collect();

            output.push_str(&import_summary.join("; "));
            output.push('\n');
        }

        // References (type tracking) - only show if present
        if !result.references.is_empty() {
            Self::append_references(&mut output, result);
        }

        output
    }

    /// Append reference tracking information (method-to-type associations, type usage)
    fn append_references(output: &mut String, result: &AnalysisResult) {
        use crate::developer::analyze::types::ReferenceType;

        // Group references by type
        let mut method_defs = Vec::new();
        let mut type_inst = Vec::new();
        let mut field_types = Vec::new();
        let mut var_types = Vec::new();
        let mut param_types = Vec::new();

        for ref_info in &result.references {
            match ref_info.ref_type {
                ReferenceType::MethodDefinition => method_defs.push(ref_info),
                ReferenceType::TypeInstantiation => type_inst.push(ref_info),
                ReferenceType::FieldType => field_types.push(ref_info),
                ReferenceType::VariableType => var_types.push(ref_info),
                ReferenceType::ParameterType => param_types.push(ref_info),
                ReferenceType::Call | ReferenceType::Definition | ReferenceType::Import => {}
            }
        }

        // Only show section if we have non-call references
        if method_defs.is_empty()
            && type_inst.is_empty()
            && field_types.is_empty()
            && var_types.is_empty()
            && param_types.is_empty()
        {
            return;
        }

        output.push_str("\nR: ");

        let mut sections = Vec::new();

        // Method definitions (methods associated with types)
        if !method_defs.is_empty() {
            let mut method_strs: Vec<String> = method_defs
                .iter()
                .map(|r| {
                    if let Some(type_name) = &r.associated_type {
                        format!("{}({})", r.symbol, type_name)
                    } else {
                        r.symbol.clone()
                    }
                })
                .collect();
            method_strs.sort();
            method_strs.dedup();
            sections.push(format!("methods[{}]", method_strs.join(" ")));
        }

        // Type instantiations (struct literals)
        if !type_inst.is_empty() {
            let mut type_names: Vec<String> = type_inst.iter().map(|r| r.symbol.clone()).collect();
            type_names.sort();
            type_names.dedup();
            sections.push(format!("types[{}]", type_names.join(" ")));
        }

        // Field types (only show unique types, not all occurrences)
        if !field_types.is_empty() {
            let mut field_type_names: Vec<String> =
                field_types.iter().map(|r| r.symbol.clone()).collect();
            field_type_names.sort();
            field_type_names.dedup();
            sections.push(format!("fields[{}]", field_type_names.join(" ")));
        }

        // Variable types (only show unique types)
        if !var_types.is_empty() {
            let mut var_type_names: Vec<String> =
                var_types.iter().map(|r| r.symbol.clone()).collect();
            var_type_names.sort();
            var_type_names.dedup();
            sections.push(format!("vars[{}]", var_type_names.join(" ")));
        }

        // Parameter types (only show unique types)
        if !param_types.is_empty() {
            let mut param_type_names: Vec<String> =
                param_types.iter().map(|r| r.symbol.clone()).collect();
            param_type_names.sort();
            param_type_names.dedup();
            sections.push(format!("params[{}]", param_type_names.join(" ")));
        }

        output.push_str(&sections.join("; "));
        output.push('\n');
    }

    /// Format directory structure with summary
    pub fn format_directory_structure(
        base_path: &Path,
        results: &[(PathBuf, EntryType)],
        max_depth: u32,
    ) -> String {
        let mut output = String::new();

        // Add summary section
        Self::append_summary(&mut output, results, max_depth);

        output.push_str("\nPATH [LOC, FUNCTIONS, CLASSES] <FLAGS>\n");

        // Add tree structure
        Self::append_tree_structure(&mut output, base_path, results);

        output
    }

    /// Append summary section with statistics
    fn append_summary(output: &mut String, results: &[(PathBuf, EntryType)], max_depth: u32) {
        // Calculate totals (only from files)
        let files: Vec<&AnalysisResult> = results
            .iter()
            .filter_map(|(_, entry)| match entry {
                EntryType::File(result) => Some(result),
                _ => None,
            })
            .collect();

        let total_files = files.len();
        let total_lines: usize = files.iter().map(|r| r.line_count).sum();
        let total_functions: usize = files.iter().map(|r| r.function_count).sum();
        let total_classes: usize = files.iter().map(|r| r.class_count).sum();

        // Format summary with depth indicator
        output.push_str("SUMMARY:\n");
        if max_depth == 0 {
            output.push_str(&format!(
                "Shown: {} files, {}L, {}F, {}C (unlimited depth)\n",
                total_files, total_lines, total_functions, total_classes
            ));
        } else {
            output.push_str(&format!(
                "Shown: {} files, {}L, {}F, {}C (max_depth={})\n",
                total_files, total_lines, total_functions, total_classes, max_depth
            ));
        }

        // Add language distribution
        Self::append_language_stats(output, results, total_lines);
    }

    /// Append language statistics
    fn append_language_stats(
        output: &mut String,
        results: &[(PathBuf, EntryType)],
        total_lines: usize,
    ) {
        // Calculate language distribution
        let mut language_lines: HashMap<String, usize> = HashMap::new();
        for (path, entry) in results {
            if let EntryType::File(result) = entry {
                let lang = lang::get_language_identifier(path);
                if !lang.is_empty() && result.line_count > 0 {
                    *language_lines.entry(lang.to_string()).or_insert(0) += result.line_count;
                }
            }
        }

        // Format language percentages
        if !language_lines.is_empty() && total_lines > 0 {
            let mut languages: Vec<_> = language_lines.iter().collect();
            languages.sort_by(|a, b| b.1.cmp(a.1)); // Sort by lines descending

            let lang_str: Vec<String> = languages
                .iter()
                .map(|(lang, lines)| {
                    let percentage = (**lines as f64 / total_lines as f64 * 100.0) as u32;
                    format!("{} ({}%)", lang, percentage)
                })
                .collect();

            output.push_str(&format!("Languages: {}\n", lang_str.join(", ")));
        }
    }

    /// Append tree structure for directory contents
    fn append_tree_structure(
        output: &mut String,
        base_path: &Path,
        results: &[(PathBuf, EntryType)],
    ) {
        // Sort results by path for consistent output
        let mut sorted_results = results.to_vec();
        sorted_results.sort_by(|a, b| a.0.cmp(&b.0));

        // Track which directories we've already printed to avoid duplicates
        let mut printed_dirs = HashSet::new();

        // Format each entry with tree-style indentation
        for (path, entry) in sorted_results {
            Self::format_tree_entry(output, base_path, &path, &entry, &mut printed_dirs);
        }
    }

    /// Format a single tree entry
    fn format_tree_entry(
        output: &mut String,
        base_path: &Path,
        path: &Path,
        entry: &EntryType,
        printed_dirs: &mut HashSet<PathBuf>,
    ) {
        // Make path relative to base_path
        let relative_path = path.strip_prefix(base_path).unwrap_or(path);

        // Get path components for determining structure
        let components: Vec<_> = relative_path.components().collect();
        if components.is_empty() {
            return;
        }

        // Print parent directories if not already printed
        for i in 0..components.len().saturating_sub(1) {
            let parent_path: PathBuf = components[..=i].iter().collect();
            if !printed_dirs.contains(&parent_path) {
                let indent = "  ".repeat(i);
                let dir_name = components[i].as_os_str().to_string_lossy();
                output.push_str(&format!("{}{}/\n", indent, dir_name));
                printed_dirs.insert(parent_path);
            }
        }

        // Determine indentation level for this entry
        let indent_level = components.len().saturating_sub(1);
        let indent = "  ".repeat(indent_level);

        // Get the file/directory name (last component)
        let name = components
            .last()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_else(|| relative_path.display().to_string());

        // Format based on entry type
        Self::format_entry_line(
            output,
            &indent,
            &name,
            entry,
            base_path,
            relative_path,
            printed_dirs,
        );
    }

    /// Format the line for a specific entry type
    fn format_entry_line(
        output: &mut String,
        indent: &str,
        name: &str,
        entry: &EntryType,
        base_path: &Path,
        relative_path: &Path,
        printed_dirs: &mut HashSet<PathBuf>,
    ) {
        match entry {
            EntryType::File(result) => {
                output.push_str(&format!("{}{} [{}L", indent, name, result.line_count));
                if result.function_count > 0 {
                    output.push_str(&format!(", {}F", result.function_count));
                }
                if result.class_count > 0 {
                    output.push_str(&format!(", {}C", result.class_count));
                }
                output.push(']');
                if let Some(main_line) = result.main_line {
                    output.push_str(&format!(" main:{}", main_line));
                }
                output.push('\n');
            }
            EntryType::Directory => {
                // Only print if not already printed as a parent
                if !printed_dirs.contains(relative_path) {
                    output.push_str(&format!("{}{}/\n", indent, name));
                    printed_dirs.insert(relative_path.to_path_buf());
                }
            }
            EntryType::SymlinkDir(target) | EntryType::SymlinkFile(target) => {
                let is_dir = matches!(entry, EntryType::SymlinkDir(_));
                let target_display = if target.is_relative() {
                    target.display().to_string()
                } else if let Ok(rel) = target.strip_prefix(base_path) {
                    rel.display().to_string()
                } else {
                    target.display().to_string()
                };
                let suffix = if is_dir { "/" } else { "" };
                output.push_str(&format!(
                    "{}{}{} -> {}\n",
                    indent, name, suffix, target_display
                ));
            }
        }
    }

    /// Format focused analysis output with call chains
    pub fn format_focused_output(focus_data: &FocusedAnalysisData) -> String {
        let mut output = format!("FOCUSED ANALYSIS: {}\n\n", focus_data.focus_symbol);

        // Build file alias mapping
        let (file_map, sorted_files) = Self::build_file_aliases(
            focus_data.definitions,
            focus_data.incoming_chains,
            focus_data.outgoing_chains,
        );

        // Section 1: Definitions
        Self::append_definitions(
            &mut output,
            focus_data.definitions,
            &file_map,
            focus_data.focus_symbol,
        );

        // Section 2: Incoming Call Chains
        Self::append_call_chains(
            &mut output,
            focus_data.incoming_chains,
            &file_map,
            focus_data.follow_depth,
            true,
        );

        // Section 3: Outgoing Call Chains
        Self::append_call_chains(
            &mut output,
            focus_data.outgoing_chains,
            &file_map,
            focus_data.follow_depth,
            false,
        );

        // Section 4: Summary Statistics
        Self::append_statistics(
            &mut output,
            focus_data.files_analyzed,
            focus_data.definitions,
            focus_data.incoming_chains,
            focus_data.outgoing_chains,
            focus_data.follow_depth,
        );

        // Section 5: File Legend
        Self::append_file_legend(
            &mut output,
            &file_map,
            &sorted_files,
            focus_data.definitions,
            focus_data.incoming_chains,
            focus_data.outgoing_chains,
        );

        if focus_data.definitions.is_empty()
            && focus_data.incoming_chains.is_empty()
            && focus_data.outgoing_chains.is_empty()
        {
            output = format!(
                "Symbol '{}' not found in any analyzed files.\n",
                focus_data.focus_symbol
            );
        }

        output
    }

    /// Build file alias mapping for focused output
    fn build_file_aliases(
        definitions: &[(PathBuf, usize)],
        incoming_chains: &[CallChain],
        outgoing_chains: &[CallChain],
    ) -> (HashMap<PathBuf, String>, Vec<PathBuf>) {
        let mut all_files = HashSet::new();

        for (file, _) in definitions {
            all_files.insert(file.clone());
        }

        for chain in incoming_chains.iter().chain(outgoing_chains.iter()) {
            for (file, _, _, _) in &chain.path {
                all_files.insert(file.clone());
            }
        }

        let mut sorted_files: Vec<_> = all_files.into_iter().collect();
        sorted_files.sort();

        let mut file_map = HashMap::new();
        for (index, file) in sorted_files.iter().enumerate() {
            let alias = if sorted_files.len() == 1 {
                file.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            } else {
                format!("F{}", index + 1)
            };
            file_map.insert(file.clone(), alias);
        }

        (file_map, sorted_files)
    }

    /// Append definitions section to output
    fn append_definitions(
        output: &mut String,
        definitions: &[(PathBuf, usize)],
        file_map: &HashMap<PathBuf, String>,
        focus_symbol: &str,
    ) {
        if !definitions.is_empty() {
            output.push_str("DEFINITIONS:\n");
            for (file, line) in definitions {
                let alias = file_map.get(file).cloned().unwrap_or_else(|| {
                    file.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                });
                output.push_str(&format!("{}:{} - {}\n", alias, line, focus_symbol));
            }
            output.push('\n');
        }
    }

    /// Append call chains section to output
    fn append_call_chains(
        output: &mut String,
        chains: &[CallChain],
        file_map: &HashMap<PathBuf, String>,
        follow_depth: u32,
        is_incoming: bool,
    ) {
        if !chains.is_empty() {
            let chain_type = if is_incoming { "INCOMING" } else { "OUTGOING" };
            output.push_str(&format!(
                "{} CALL CHAINS (depth={}):\n",
                chain_type, follow_depth
            ));

            let mut unique_chains = HashSet::new();
            for chain in chains {
                let chain_str = Self::format_chain_path(&chain.path, file_map);
                unique_chains.insert(chain_str);
            }

            let mut sorted_chains: Vec<_> = unique_chains.into_iter().collect();
            sorted_chains.sort();

            for chain in sorted_chains {
                output.push_str(&format!("{}\n", chain));
            }
            output.push('\n');
        }
    }

    /// Format a single chain path
    fn format_chain_path(
        path: &[(PathBuf, usize, String, String)],
        file_map: &HashMap<PathBuf, String>,
    ) -> String {
        path.iter()
            .map(|(file, line, from, to)| {
                let alias = file_map.get(file).cloned().unwrap_or_else(|| {
                    file.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                });
                format!("{}:{} ({} -> {})", alias, line, from, to)
            })
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    /// Append statistics section to output
    fn append_statistics(
        output: &mut String,
        files_analyzed: &[PathBuf],
        definitions: &[(PathBuf, usize)],
        incoming_chains: &[CallChain],
        outgoing_chains: &[CallChain],
        follow_depth: u32,
    ) {
        output.push_str("STATISTICS:\n");
        output.push_str(&format!("  Files analyzed: {}\n", files_analyzed.len()));
        output.push_str(&format!("  Definitions found: {}\n", definitions.len()));
        output.push_str(&format!("  Incoming chains: {}\n", incoming_chains.len()));
        output.push_str(&format!("  Outgoing chains: {}\n", outgoing_chains.len()));
        output.push_str(&format!("  Follow depth: {}\n", follow_depth));
    }

    /// Append file legend section to output
    fn append_file_legend(
        output: &mut String,
        file_map: &HashMap<PathBuf, String>,
        sorted_files: &[PathBuf],
        definitions: &[(PathBuf, usize)],
        incoming_chains: &[CallChain],
        outgoing_chains: &[CallChain],
    ) {
        if !file_map.is_empty()
            && (sorted_files.len() > 1
                || !incoming_chains.is_empty()
                || !outgoing_chains.is_empty()
                || !definitions.is_empty())
        {
            output.push_str("\nFILES:\n");
            let mut legend_entries: Vec<_> = file_map.iter().collect();
            legend_entries.sort_by_key(|(_, alias)| alias.as_str());

            for (file_path, alias) in legend_entries {
                if sorted_files.len() == 1
                    && alias == file_path.file_name().and_then(|n| n.to_str()).unwrap_or("")
                {
                    continue;
                }
                output.push_str(&format!("  {}: {}\n", alias, file_path.display()));
            }
        }
    }

    /// Filter output by focus symbol
    pub fn filter_by_focus(output: &str, focus: &str) -> String {
        let mut filtered = String::new();
        let mut include_section = false;

        for line in output.lines() {
            if line.starts_with("##") {
                include_section = false;
            }

            if line.contains(focus) {
                include_section = true;
                // Include the file header
                if let Some(header_line) = output
                    .lines()
                    .rev()
                    .find(|l| l.starts_with("##") && l.get(3..).is_some_and(|s| line.contains(s)))
                {
                    if !filtered.contains(header_line) {
                        filtered.push_str(header_line);
                        filtered.push('\n');
                    }
                }
            }

            if include_section || line.starts_with('#') {
                filtered.push_str(line);
                filtered.push('\n');
            }
        }

        if filtered.is_empty() {
            format!("No results found for symbol: {}", focus)
        } else {
            filtered
        }
    }
}
