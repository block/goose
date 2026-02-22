use crate::config::paths::Paths;
use include_dir::{include_dir, Dir};
use minijinja::{Environment, Error as MiniJinjaError, Value as MJValue};
use serde::Serialize;
use std::path::PathBuf;

static CORE_PROMPTS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/prompts");

static TEMPLATE_REGISTRY: &[(&str, &str)] = &[
    (
        "system.md",
        "Main system prompt that defines goose's personality and behavior",
    ),
    (
        "compaction.md",
        "Prompt for summarizing conversation history when context limits are reached",
    ),
    (
        "specialist.md",
        "System prompt for specialists spawned to handle specific tasks",
    ),
    (
        "recipe.md",
        "Prompt for generating recipe files from conversations",
    ),
    (
        "apps_create.md",
        "Prompt for generating new Goose apps based on the user instructions",
    ),
    (
        "apps_iterate.md",
        "Prompt for updating existing Goose apps based on feedback",
    ),
    (
        "permission_judge.md",
        "Prompt for analyzing tool operations for read-only detection",
    ),
    (
        "plan.md",
        "Prompt used when goose creates step-by-step plans. CLI only",
    ),
    (
        "genui.md",
        "Prompt for generating inline json-render dashboards and visualizations",
    ),
    // Goose Agent (universal public modes)
    (
        "goose/ask.md",
        "Goose Ask — read-only exploration, search, and Q&A",
    ),
    (
        "goose/plan.md",
        "Goose Plan — strategy, architecture, and step-by-step plans",
    ),
    (
        "goose/write.md",
        "Goose Write — create/edit files, run commands, build artifacts",
    ),
    (
        "goose/review.md",
        "Goose Review — evaluate code, docs, and artifacts for quality",
    ),
    // Developer Agent (universal modes)
    (
        "developer/ask.md",
        "Developer Ask — read-only exploration, search, and Q&A",
    ),
    (
        "developer/plan.md",
        "Developer Plan — design, architecture, ADRs, and implementation plans",
    ),
    (
        "developer/write.md",
        "Developer Write — implement code, configs, tests with full tool access",
    ),
    (
        "developer/review.md",
        "Developer Review — code review, audit, and quality assessment",
    ),
    (
        "developer/debug.md",
        "Developer Debug — systematic diagnosis and root-cause analysis",
    ),
    // QA Agent — universal modes
    (
        "qa/ask.md",
        "QA Agent Ask — read-only exploration of testing and quality",
    ),
    (
        "qa/plan.md",
        "QA Agent Plan — test strategy and test plan design",
    ),
    (
        "qa/write.md",
        "QA Agent Write — implement tests and quality infrastructure",
    ),
    (
        "qa/review.md",
        "QA Agent Review — evaluate test adequacy and code quality",
    ),
    // PM Agent — universal modes
    (
        "pm/ask.md",
        "PM Agent Ask — product questions, requirements exploration",
    ),
    (
        "pm/plan.md",
        "PM Agent Plan — PRDs, roadmaps, prioritization",
    ),
    (
        "pm/write.md",
        "PM Agent Write — produce product documents and specs",
    ),
    (
        "pm/review.md",
        "PM Agent Review — evaluate requirements and specs quality",
    ),
    // Security Agent — universal modes
    (
        "security/ask.md",
        "Security Agent Ask — security questions and analysis",
    ),
    (
        "security/plan.md",
        "Security Agent Plan — threat models and remediation plans",
    ),
    (
        "security/write.md",
        "Security Agent Write — security patches and hardening",
    ),
    (
        "security/review.md",
        "Security Agent Review — security code review and audit",
    ),
    // Research Agent — universal modes
    (
        "research/ask.md",
        "Research Agent Ask — answer questions with evidence",
    ),
    (
        "research/plan.md",
        "Research Agent Plan — research strategy and investigation design",
    ),
    (
        "research/write.md",
        "Research Agent Write — produce research reports and comparisons",
    ),
    (
        "research/review.md",
        "Research Agent Review — evaluate research quality and sources",
    ),
    (
        "orchestrator/system.md",
        "Orchestrator system prompt — meta-coordinator for routing to agents/modes",
    ),
    (
        "orchestrator/routing.md",
        "Orchestrator routing prompt — structured output for agent/mode selection",
    ),
    (
        "orchestrator/splitting.md",
        "Orchestrator splitting prompt — detect and decompose compound requests into sub-tasks",
    ),
];

/// Information about a template including its content and customization status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Template {
    pub name: String,
    pub description: String,
    pub default_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_content: Option<String>,
    pub is_customized: bool,
}

fn user_prompts_dir() -> PathBuf {
    Paths::config_dir().join("prompts")
}

fn is_registered(name: &str) -> bool {
    TEMPLATE_REGISTRY.iter().any(|(n, _)| *n == name)
}

pub fn render_string<T: Serialize>(
    template_str: &str,
    context: &T,
) -> Result<String, MiniJinjaError> {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);

    env.set_loader(|name| {
        let is_safe_path = !name.starts_with('/')
            && !name.starts_with('\\')
            && !name.contains("..")
            && !name.contains(':');
        if !is_safe_path {
            return Ok(None);
        }

        let user_path = user_prompts_dir().join(name);
        if user_path.exists() {
            let content = std::fs::read_to_string(&user_path).map_err(|e| {
                MiniJinjaError::new(
                    minijinja::ErrorKind::InvalidOperation,
                    format!("Failed to read user template: {e}"),
                )
            })?;
            return Ok(Some(content));
        }

        let file = CORE_PROMPTS_DIR.get_file(name);
        Ok(file.map(|f| String::from_utf8_lossy(f.contents()).to_string()))
    });

    env.add_template("template", template_str)?;
    let tmpl = env.get_template("template")?;
    let ctx = MJValue::from_serialize(context);
    let rendered = tmpl.render(ctx)?;
    Ok(rendered.trim().to_string())
}

pub fn render_template<T: Serialize>(name: &str, context: &T) -> Result<String, MiniJinjaError> {
    if !is_registered(name) {
        return Err(MiniJinjaError::new(
            minijinja::ErrorKind::TemplateNotFound,
            format!("Template '{}' is not registered", name),
        ));
    }

    let user_path = user_prompts_dir().join(name);
    let template_str = if user_path.exists() {
        std::fs::read_to_string(&user_path).map_err(|e| {
            MiniJinjaError::new(
                minijinja::ErrorKind::InvalidOperation,
                format!("Failed to read user template: {}", e),
            )
        })?
    } else {
        let file = CORE_PROMPTS_DIR.get_file(name).ok_or_else(|| {
            MiniJinjaError::new(
                minijinja::ErrorKind::TemplateNotFound,
                format!("Built-in template '{}' not found", name),
            )
        })?;
        String::from_utf8_lossy(file.contents()).to_string()
    };

    render_string(&template_str, context)
}

pub fn get_template(name: &str) -> Option<Template> {
    let (_, description) = TEMPLATE_REGISTRY.iter().find(|(n, _)| *n == name)?;

    let default_content = CORE_PROMPTS_DIR
        .get_file(name)
        .map(|file| String::from_utf8_lossy(file.contents()).to_string())?;

    let user_path = user_prompts_dir().join(name);
    let user_content = if user_path.exists() {
        std::fs::read_to_string(&user_path).ok()
    } else {
        None
    };

    let is_customized = user_content.is_some();

    Some(Template {
        name: name.to_string(),
        description: description.to_string(),
        default_content,
        user_content,
        is_customized,
    })
}

pub fn save_template(name: &str, content: &str) -> std::io::Result<()> {
    if !is_registered(name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Template '{}' is not registered", name),
        ));
    }

    let prompts_dir = user_prompts_dir();
    std::fs::create_dir_all(&prompts_dir)?;
    let path = prompts_dir.join(name);
    std::fs::write(path, content)
}

/// Reset a template to its default by removing the user customization.
pub fn reset_template(name: &str) -> std::io::Result<()> {
    if !is_registered(name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Template '{}' is not registered", name),
        ));
    }

    let path = user_prompts_dir().join(name);
    if path.exists() {
        std::fs::remove_file(path)
    } else {
        Ok(())
    }
}

pub fn list_templates() -> Vec<Template> {
    TEMPLATE_REGISTRY
        .iter()
        .filter_map(|(name, description)| {
            let default_content = CORE_PROMPTS_DIR
                .get_file(name)
                .map(|file| String::from_utf8_lossy(file.contents()).to_string())?;

            let user_path = user_prompts_dir().join(name);
            let user_content = if user_path.exists() {
                std::fs::read_to_string(&user_path).ok()
            } else {
                None
            };

            let is_customized = user_content.is_some();

            Some(Template {
                name: name.to_string(),
                description: description.to_string(),
                default_content,
                user_content,
                is_customized,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_get_template() {
        let template = get_template("system.md");
        assert!(template.is_some(), "system.md should be registered");

        let template = template.unwrap();
        assert_eq!(template.name, "system.md");
        assert!(!template.description.is_empty());
        assert!(!template.default_content.is_empty());
        assert!(!template.is_customized);
    }

    #[test]
    fn test_render_template() {
        let context: HashMap<String, String> = HashMap::new();
        let result = render_template("system.md", &context);
        assert!(result.is_ok(), "Should be able to render system.md");
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_list_templates() {
        let templates = list_templates();
        assert_eq!(templates.len(), TEMPLATE_REGISTRY.len());

        let has_system = templates.iter().any(|t| t.name == "system.md");
        assert!(has_system, "system.md should be in the template list");

        for template in templates {
            assert!(
                !template.description.is_empty(),
                "Each template should have a description"
            );
            assert!(
                !template.default_content.is_empty(),
                "Each template should have content"
            );
        }
    }

    #[test]
    fn test_render_string_supports_includes() {
        let context: HashMap<String, String> = HashMap::new();
        let result = render_string(
            "{% include \"partials/genui_output_contract.md\" %}",
            &context,
        );
        assert!(result.is_ok(), "Should be able to render includes");
        let text = result.unwrap();
        assert!(text.contains("Output contract"));
        assert!(text.contains("json-render"));
    }

    #[test]
    fn test_render_template_with_includes() {
        let context: HashMap<String, String> = HashMap::new();
        let rendered = render_template("genui.md", &context).expect("Should render genui.md");
        assert!(rendered.contains("Output contract"));
        assert!(rendered.contains("json-render"));
    }
}
