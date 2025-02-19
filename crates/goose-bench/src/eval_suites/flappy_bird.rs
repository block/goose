use crate::eval_suites::{Evaluation, Extension, Model};
use crate::eval_suites::evaluation::EvaluationReport;
use crate::register_evaluation;

pub struct FlappyBird {}

impl FlappyBird {
    fn new() -> FlappyBird {
        FlappyBird {}
    }
}

impl Evaluation for FlappyBird {
    fn run(&self) -> anyhow::Result<EvaluationReport> {
        let mut metrics = Vec::new();

        Ok(EvaluationReport::new(metrics))
    }

    fn models(&self) -> Vec<Model> {
        todo!()
    }

    fn extensions(&self) -> Vec<Extension> {
        todo!()
    }
}

register_evaluation!("flappy_bird", FlappyBird);
