use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

pub use super::Evaluation;

type EvaluationConstructor = Box<dyn Fn() -> Box<dyn Evaluation> + Send + Sync>;

// Use std::sync::RwLock for interior mutability
static EVALUATION_REGISTRY: OnceLock<RwLock<HashMap<&'static str, EvaluationConstructor>>> = OnceLock::new();

/// Initialize the registry if it hasn't been initialized
fn registry() -> &'static RwLock<HashMap<&'static str, EvaluationConstructor>> {
    EVALUATION_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register a new evaluation version
pub fn register_evaluation(
    version: &'static str,
    constructor: impl Fn() -> Box<dyn Evaluation> + Send + Sync + 'static,
) {
    let registry = registry();
    if let Ok(mut map) = registry.write() {
        map.insert(version, Box::new(constructor));
    }
}

pub struct EvaluationFactory;

impl EvaluationFactory {
    pub fn create(version: &str) -> Option<Box<dyn Evaluation>> {
        let registry = registry();
        let map = registry
            .read()
            .expect("Failed to read the benchmark evaluation registry.");
        let constructor = map.get(version)?;
        Some(constructor())
    }

    pub fn available_evaluations() -> Vec<&'static str> {
        registry()
            .read()
            .map(|map| map.keys().copied().collect())
            .unwrap_or_default()
    }
}

#[macro_export]
macro_rules! register_evaluation {
    ($version:expr, $evaluation_type:ty) => {
        paste::paste! {
            #[ctor::ctor]
            #[allow(non_snake_case)]
            fn [<__register_evaluation_ $version>]() {
                $crate::eval_suites::factory::register_evaluation($version, || {
                    Box::new(<$evaluation_type>::new())
                });
            }
        }
    };
}
