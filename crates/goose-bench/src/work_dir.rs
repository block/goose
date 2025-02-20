use std::fs;
use std::path::PathBuf;

pub struct WorkDir {
    pub path: PathBuf,
}

impl Default for WorkDir {
    fn default() -> Self { WorkDir { path: PathBuf::from(".") } }
}
impl WorkDir {
    pub fn new(path: &str) -> Self { WorkDir { path: PathBuf::from(path) } }
    pub fn work_from(path: String) -> anyhow::Result<WorkDir> {
        match fs::create_dir_all(&path) {
            Ok(_) => Ok(WorkDir::new(path.as_str())),
            _ => Err(anyhow::Error::msg("work dir creation failed")),
        }
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        if let Some(parent_path) = self.path.parent() {
            std::env::set_current_dir(parent_path).unwrap();
        };
    }
}