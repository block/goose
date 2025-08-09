use etcetera::AppStrategyArgs;
use once_cell::sync::Lazy;

pub static APP_STRATEGY: Lazy<AppStrategyArgs> = Lazy::new(|| AppStrategyArgs {
    top_level_domain: "Block".to_string(),
    author: "Block".to_string(),
    app_name: "goose".to_string(),
});

mod build;
pub mod computercontroller;
mod developer;
pub mod google_drive;
mod memory;
mod tutorial;

pub use build::BuildRouter;
pub use computercontroller::ComputerControllerRouter;
pub use developer::DeveloperRouter;
pub use google_drive::GoogleDriveRouter;
pub use memory::MemoryRouter;
pub use tutorial::TutorialRouter;
