mod core;
mod evaluation;
mod factory;

pub use evaluation::*;
pub use factory::{register_evaluation, EvaluationSuiteFactory};
