use etcetera::AppStrategyArgs;
use once_cell::sync::Lazy;

pub static APP_STRATEGY: Lazy<AppStrategyArgs> = Lazy::new(|| AppStrategyArgs {
    top_level_domain: "Block".to_string(),
    author: "Block".to_string(),
    app_name: "goose".to_string(),
});

pub mod computercontroller;
mod developer;
mod google_drive;
mod jetbrains;
mod memory;
mod tutorial;

pub use computercontroller::ComputerControllerRouter;
pub use developer::DeveloperRouter;
pub use google_drive::GoogleDriveRouter;
pub use jetbrains::JetBrainsRouter;
pub use memory::MemoryRouter;
pub use tutorial::TutorialRouter;
