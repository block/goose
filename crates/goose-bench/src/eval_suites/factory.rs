pub use super::Evaluation;
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

type EvaluationConstructor = fn() -> Box<dyn Evaluation>;
type Registry = &'static RwLock<HashMap<&'static str, EvaluationConstructor>>;

// Use std::sync::RwLock for interior mutability
static EVAL_REGISTRY: OnceLock<RwLock<HashMap<&'static str, EvaluationConstructor>>> =
    OnceLock::new();

/// Initialize the registry if it hasn't been initialized
fn eval_registry() -> Registry {
    EVAL_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register a new evaluation version
pub fn register_eval(selector: &'static str, constructor: fn() -> Box<dyn Evaluation>) {
    let registry = eval_registry();
    if let Ok(mut map) = registry.write() {
        map.insert(selector, constructor);
    }
}

pub struct EvaluationSuite;

impl EvaluationSuite {
    pub fn from(selector: &str) -> Option<Box<dyn Evaluation>> {
        let registry = eval_registry();
        let map = registry
            .read()
            .expect("Failed to read the benchmark evaluation registry.");

        let constructor = map.get(selector)?;
        let instance = constructor();

        Some(instance)
    }

    pub fn registered_evals() -> Vec<&'static str> {
        let registry = eval_registry();
        let map = registry
            .read()
            .expect("Failed to read the benchmark evaluation registry.");

        let evals: Vec<_> = map.keys().copied().collect();
        evals
    }
    pub fn select(selectors: Vec<String>) -> HashMap<String, Vec<&'static str>> {
        let eval_name_pattern = Regex::new(r":\w+$").unwrap();
        let grouped_by_suite: HashMap<String, Vec<&'static str>> =
            EvaluationSuite::registered_evals()
                .into_iter()
                .filter(|&eval| selectors.is_empty() || matches_any_selectors(eval, &selectors))
                .fold(HashMap::new(), |mut suites, eval| {
                    let suite = match eval_name_pattern.replace(eval, "") {
                        Cow::Borrowed(s) => s.to_string(),
                        Cow::Owned(s) => s,
                    };
                    suites.entry(suite).or_default().push(eval);
                    suites
                });

        grouped_by_suite
    }

    pub fn available_selectors() -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for selector in EvaluationSuite::registered_evals() {
            let parts = selector.split(":").collect::<Vec<_>>();
            for i in 0..parts.len() {
                let sel = parts[..i + 1].join(":");
                *counts.entry(sel).or_insert(0) += 1;
            }
        }
        counts
    }
}

fn matches_any_selectors(eval: &str, selectors: &Vec<String>) -> bool {
    // selectors must prefix match exactly, no matching half-way in a word
    // remove one level of nesting at a time and check exact match
    let nesting_pattern = Regex::new(r":\w+$").unwrap();
    for selector in selectors {
        let mut level_up = eval.to_string();
        while !level_up.is_empty() {
            if level_up == *selector {
                return true;
            }
            if !level_up.contains(":") {
                break;
            };
            level_up = match nesting_pattern.replace(&level_up, "") {
                Cow::Borrowed(s) => s.to_string(),
                Cow::Owned(s) => s,
            };
        }
    }
    false
}

#[macro_export]
macro_rules! register_evaluation {
    ($evaluation_type:ty) => {
        paste::paste! {
            #[ctor::ctor]
            #[allow(non_snake_case)]
            fn [<__register_evaluation_ $evaluation_type>]() {
                let mut path = std::path::PathBuf::from(file!());
                path.set_extension("");
                let eval_suites_dir = "eval_suites";
                let eval_selector = {
                    let s = path.components()
                        .skip_while(|comp| comp.as_os_str() != eval_suites_dir)
                        .skip(1)
                        .map(|comp| comp.as_os_str().to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                        .join(":");
                    Box::leak(s.into_boxed_str())
                };

                $crate::eval_suites::factory::register_eval(eval_selector, || {
                    Box::new(<$evaluation_type>::new())
                });
            }
        }
    };
}
