use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

pub struct WorkDir {
    pub path: PathBuf,
    traversal: Vec<PathBuf>,
}

impl Default for WorkDir {
    fn default() -> Self {
        let path = PathBuf::from(".").canonicalize().unwrap();
        WorkDir {
            path: path.clone(),
            traversal: vec![path.clone()],
        }
    }
}
impl WorkDir {
    pub fn new(path: &str) -> Self {
        let path = PathBuf::from(path);
        WorkDir {
            path: path.clone(),
            traversal: vec![path.clone()],
        }
    }

    pub fn at(path: String, include_dirs: Vec<PathBuf>) -> anyhow::Result<WorkDir> {
        fs::create_dir_all(&path)?;

        let dirs = include_dirs
            .iter()
            .map(|d| d.canonicalize().unwrap())
            .collect::<Vec<_>>();

        let p = PathBuf::from(&path).canonicalize()?;
        let _: Vec<_> = dirs
            .iter()
            .map(|d| WorkDir::deep_copy(d.as_path(), p.as_path()))
            .collect();

        std::env::set_current_dir(&path)?;

        Ok(WorkDir::new(p.to_string_lossy().to_string().as_str()))
    }
    pub fn move_to(&mut self, path: String) -> anyhow::Result<&mut Self> {
        fs::create_dir_all(&path)?;
        self.traversal.push(PathBuf::from(&path));
        std::env::set_current_dir(&path)?;
        Ok(self)
    }

    pub fn fs_get(&mut self, path: String) -> anyhow::Result<PathBuf> {
        let p = Path::new(&path);
        if !p.exists() {
            let artifact_at_root = if p.is_dir() {
                self.traversal[0].clone().join(&path).canonicalize()?
            } else {
                self.traversal[0]
                    .clone()
                    .join(p.parent().unwrap_or(Path::new("")))
                    .canonicalize()?
            };

            let here = PathBuf::from(".").canonicalize()?;

            WorkDir::deep_copy(artifact_at_root.as_path(), here.as_path())?;
        }

        Ok(PathBuf::from(path))
    }

    fn deep_copy(src: &Path, dst: &Path) -> io::Result<()> {
        // Create the destination directory with the source's name
        let dst_dir = if let Some(src_name) = src.file_name() {
            dst.join(src_name)
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Source path must have a file name",
            ));
        };

        // Create the destination directory if it doesn't exist
        if !dst_dir.exists() {
            fs::create_dir_all(&dst_dir)?;
        }

        // Copy each entry in the source directory
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst_dir.join(entry.file_name());

            if ty.is_dir() {
                WorkDir::deep_copy(&src_path, dst_path.parent().unwrap())?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        self.traversal.pop();
        std::env::set_current_dir("..").unwrap()
    }
}
