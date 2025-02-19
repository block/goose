pub use super::Evaluation;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

type EvaluationConstructor = fn() -> Box<dyn Evaluation>;

// Use std::sync::RwLock for interior mutability
static EVALUATION_REGISTRY: OnceLock<RwLock<HashMap<&'static str, Vec<EvaluationConstructor>>>> =
    OnceLock::new();

/// Initialize the registry if it hasn't been initialized
fn registry() -> &'static RwLock<HashMap<&'static str, Vec<EvaluationConstructor>>> {
    EVALUATION_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register a new evaluation version
pub fn register_evaluation(suite_name: &'static str, constructor: fn() -> Box<dyn Evaluation>) {
    let registry = registry();
    if let Ok(mut map) = registry.write() {
        map.entry(suite_name)
            .or_insert_with(Vec::new)
            .push(constructor);
    }
}

pub struct EvaluationSuiteFactory;

impl EvaluationSuiteFactory {
    pub fn create(suite_name: &str) -> Option<Vec<Box<dyn Evaluation>>> {
        let registry = registry();
        let map = registry
            .read()
            .expect("Failed to read the benchmark evaluation registry.");

        let constructors = map.get(suite_name)?;
        let instances = constructors
            .iter()
            .map(|&constructor| constructor())
            .collect::<Vec<_>>();

        Some(instances)
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
    ($suite_name:expr, $evaluation_type:ty) => {
        paste::paste! {
            #[ctor::ctor]
            #[allow(non_snake_case)]
            fn [<__register_evaluation_ $suite_name>]() {
                $crate::eval_suites::factory::register_evaluation($suite_name, || {
                    Box::new(<$evaluation_type>::new())
                });
            }
        }
    };
}
