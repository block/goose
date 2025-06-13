use std::collections::HashMap;

pub fn preprocess_template(main_template: &str, templates: &HashMap<String, String>) -> String {
    main_template
        .lines()
        .map(|line| process_line(line, templates))
        .collect::<Vec<_>>()
        .join("\n")
}

fn process_line(line: &str, templates: &HashMap<String, String>) -> String {
    if let Some(include_name) = extract_include_name(line.trim()) {
        let indent = get_line_indent(line);
        return expand_include(&indent, include_name, templates)
            .unwrap_or_else(|| line.to_string());
    }

    line.to_string()
}

fn extract_include_name(trimmed: &str) -> Option<&str> {
    if trimmed.starts_with("{% include \"") && trimmed.ends_with("\" %}") {
        let start = "{% include \"".len();
        let end = trimmed.len() - "\" %}".len();
        Some(&trimmed[start..end])
    } else {
        None
    }
}

fn get_line_indent(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

fn expand_include(indent: &str, name: &str, templates: &HashMap<String, String>) -> Option<String> {
    templates.get(name).map(|content| {
        content
            .lines()
            .map(|l| format!("{indent}{l}"))
            .collect::<Vec<_>>()
            .join("\n")
    })
}
