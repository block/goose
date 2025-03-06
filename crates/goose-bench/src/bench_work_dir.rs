use chrono::Local;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

pub struct BenchmarkWorkDir {
    pub base_path: PathBuf,
    run_name: String,
    suite: Option<String>,
    eval: Option<String>,
}

impl Default for BenchmarkWorkDir {
    fn default() -> Self {
        BenchmarkWorkDir::new("work_dir".to_string(), Vec::new())
    }
}
impl BenchmarkWorkDir {
    pub fn new(work_dir_name: String, include_dirs: Vec<PathBuf>) -> Self {
        let base_path = PathBuf::from(format!("./benchmark-{}", work_dir_name));
        fs::create_dir_all(&base_path).unwrap();

        let current_time = Local::now().format("T%H_%M_%S").to_string();
        let current_date = Local::now().format("%Y-%m-%d").to_string();
        let run_name = format!("{}-{}", &current_date, current_time);

        let mut base_path = PathBuf::from(&base_path).canonicalize().unwrap();
        base_path.push(run_name.clone());
        fs::create_dir_all(&base_path).unwrap();
        base_path.pop();

        // abs paths from dir-strings
        let dirs = include_dirs
            .iter()
            .map(|d| d.canonicalize().unwrap())
            .collect::<Vec<_>>();

        // deep copy each dir
        let _: Vec<_> = dirs
            .iter()
            .map(|d| BenchmarkWorkDir::deep_copy(d.as_path(), base_path.as_path()))
            .collect();

        std::env::set_current_dir(&base_path).unwrap();

        BenchmarkWorkDir {
            base_path,
            run_name,
            suite: None,
            eval: None,
        }
    }
    pub fn cd(&mut self, path: PathBuf) -> anyhow::Result<&mut Self> {
        fs::create_dir_all(&path)?;
        std::env::set_current_dir(&path)?;
        Ok(self)
    }
    pub fn set_suite(&mut self, suite: &str) {
        self.eval = None;
        self.suite = Some(suite.to_string());

        let mut suite_dir = self.base_path.clone();
        suite_dir.push(self.run_name.clone());
        suite_dir.push(suite);

        self.cd(suite_dir.clone()).unwrap_or_else(|_| {
            panic!("Failed to execute cd into {}", suite_dir.clone().display())
        });
    }
    pub fn set_eval(&mut self, eval: &str) {
        self.eval = Some(eval.to_string());

        let mut eval_dir = self.base_path.clone();
        eval_dir.push(self.run_name.clone());
        eval_dir.push(self.suite.clone().unwrap());
        eval_dir.push(eval);

        self.cd(eval_dir.clone())
            .unwrap_or_else(|_| panic!("Failed to execute cd into {}", eval_dir.clone().display()));
    }

    pub fn fs_get(&mut self, path: String) -> anyhow::Result<PathBuf> {
        let p = Path::new(&path);
        if !p.exists() {
            let artifact_at_root = if p.is_dir() {
                self.base_path.clone().join(&path).canonicalize()?
            } else {
                self.base_path
                    .clone()
                    .join(p.parent().unwrap_or(Path::new("")))
                    .canonicalize()?
            };

            let here = PathBuf::from(".").canonicalize()?;

            BenchmarkWorkDir::deep_copy(artifact_at_root.as_path(), here.as_path())?;
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
                BenchmarkWorkDir::deep_copy(&src_path, dst_path.parent().unwrap())?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }
}
