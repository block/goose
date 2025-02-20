use std::fs;
use std::path::PathBuf;

pub struct WorkDir {
    pub path: PathBuf,
}

impl Default for WorkDir {
    fn default() -> Self {
        WorkDir {
            path: PathBuf::from("."),
        }
    }
}
impl WorkDir {
    pub fn new(path: &str) -> Self {
        WorkDir {
            path: PathBuf::from(path),
        }
    }

    pub fn work_from(path: String) -> anyhow::Result<WorkDir> {
        let _ = fs::create_dir_all(&path)?;
        let _ = std::env::set_current_dir(&path)?;
        Ok(WorkDir::new(path.as_str()))
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        std::env::set_current_dir("..").unwrap()
    }
}
