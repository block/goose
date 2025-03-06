mod core;
mod evaluation;
mod factory;
mod small_models;

pub use evaluation::*;
pub use factory::{register_evaluation, EvaluationSuiteFactory};
