use goose::config::paths::Paths;
use regex::Regex;
use std::collections::HashMap;

const MAIN_CSS: &str = include_str!("../../../ui/desktop/src/styles/main.css");

/// Theme CSS Generation
/// ====================
///
/// Both main.css and user's theme.css use MCP-compliant variable names.
/// This module simply:
/// 1. Parses main.css to get default MCP color variables from :root and .dark
/// 2. Parses user's theme.css (if exists) which also uses MCP names
/// 3. Merges them (user overrides defaults)
/// 4. Returns variables in light-dark(light_value, dark_value) format for frontend injection

fn parse_css_variables(css: &str) -> (HashMap<String, String>, HashMap<String, String>) {
    let mut root_vars = HashMap::new();
    let mut dark_vars = HashMap::new();

    let var_regex = Regex::new(r"--([a-z0-9-]+):\s*([^;]+);").unwrap();

    let mut in_root = false;
    let mut in_dark = false;
    let mut brace_depth = 0;

    for line in css.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with(":root") {
            in_root = true;
            in_dark = false;
            brace_depth = 0;
        } else if trimmed.starts_with(".dark") {
            in_dark = true;
            in_root = false;
            brace_depth = 0;
        }

        brace_depth += trimmed.chars().filter(|&c| c == '{').count() as i32;
        brace_depth -= trimmed.chars().filter(|&c| c == '}').count() as i32;

        if brace_depth <= 0 {
            in_root = false;
            in_dark = false;
        }

        if let Some(caps) = var_regex.captures(trimmed) {
            let name = caps.get(1).unwrap().as_str().to_string();
            let value = caps.get(2).unwrap().as_str().trim().to_string();

            if in_root {
                root_vars.insert(name, value);
            } else if in_dark {
                dark_vars.insert(name, value);
            }
        }
    }

    (root_vars, dark_vars)
}

fn resolve_var_reference(value: &str, vars: &HashMap<String, String>) -> String {
    let var_ref_regex = Regex::new(r"var\(--([a-z0-9-]+)\)").unwrap();

    let mut result = value.to_string();
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 10;

    while iterations < MAX_ITERATIONS {
        if let Some(caps) = var_ref_regex.captures(&result.clone()) {
            let var_name = caps.get(1).unwrap().as_str();
            if let Some(resolved) = vars.get(var_name) {
                let full_match = caps.get(0).unwrap().as_str();
                result = result.replace(full_match, resolved);
                iterations += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    result
}

pub fn generate_mcp_theme_variables() -> HashMap<String, String> {
    let (main_root, main_dark) = parse_css_variables(MAIN_CSS);
    let mut merged_root = main_root.clone();

    let mut merged_dark = main_dark.clone();

    let theme_path = Paths::in_data_dir("theme.css");
    if theme_path.exists() {
        if let Ok(theme_css) = std::fs::read_to_string(&theme_path) {
            let (theme_root, theme_dark) = parse_css_variables(&theme_css);
            merged_root.extend(theme_root);
            merged_dark.extend(theme_dark);
        }
    };


    let resolved_root: HashMap<String, String> = merged_root
        .iter()
        .map(|(k, v)| (k.clone(), resolve_var_reference(v, &merged_root)))
        .collect();

    let resolved_dark: HashMap<String, String> = merged_dark
        .iter()
        .map(|(k, v)| (k.clone(), resolve_var_reference(v, &merged_dark)))
        .collect();

    let mut result = HashMap::new();

    for (name, light_value) in &resolved_root {
        if name.starts_with("color-") {
            let dark_value = resolved_dark.get(name).unwrap_or(light_value);

            let formatted = format!("light-dark({}, {})", light_value, dark_value);
            result.insert(format!("--{}", name), formatted);
        }
    }

    for (name, dark_value) in &resolved_dark {
        if name.starts_with("color-") && !result.contains_key(&format!("--{}", name)) {
            let light_value = resolved_root.get(name).unwrap_or(dark_value);
            let formatted = format!("light-dark({}, {})", light_value, dark_value);
            result.insert(format!("--{}", name), formatted);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_css_variables() {
        let css = r#"
            :root {
                --color-background-primary: #ffffff;
                --color-text-primary: var(--color-neutral-800);
            }
            .dark {
                --color-background-primary: #000000;
            }
        "#;

        let (root, dark) = parse_css_variables(css);
        assert_eq!(root.get("color-background-primary"), Some(&"#ffffff".to_string()));
        assert_eq!(dark.get("color-background-primary"), Some(&"#000000".to_string()));
    }

    #[test]
    fn test_resolve_var_reference() {
        let mut vars = HashMap::new();
        vars.insert("color-red".to_string(), "#ff0000".to_string());
        vars.insert("color-text-danger".to_string(), "var(--color-red)".to_string());

        assert_eq!(
            resolve_var_reference("var(--color-red)", &vars),
            "#ff0000"
        );
        assert_eq!(
            resolve_var_reference("var(--color-text-danger)", &vars),
            "#ff0000"
        );
    }
}
