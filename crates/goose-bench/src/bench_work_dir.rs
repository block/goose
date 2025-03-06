use chrono::Local;
use std::fs;
use std::io;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

pub struct BenchmarkWorkDir {
    pub base_path: PathBuf,
    cwd: PathBuf,
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

        let current_time = Local::now().format("%H:%M:%S").to_string();
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
            .map(|d| BenchmarkWorkDir::deep_copy(d.as_path(), base_path.as_path(), true))
            .collect();

        std::env::set_current_dir(&base_path).unwrap();

        BenchmarkWorkDir {
            base_path: base_path.clone(),
            cwd: base_path.clone(),
            run_name,
            suite: None,
            eval: None,
        }
    }
    pub fn cd(&mut self, path: PathBuf) -> anyhow::Result<&mut Self> {
        fs::create_dir_all(&path)?;
        std::env::set_current_dir(&path)?;
        self.cwd = path;
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

    fn chop_relative_base<P: AsRef<Path>>(path: P) -> anyhow::Result<PathBuf> {
        let path = path.as_ref();

        // Get the path components as an iterator
        let mut components = path.components();

        // Check the first component
        if let Some(first) = components.next() {
            use std::path::Component;

            match first {
                Component::ParentDir => Err(anyhow::anyhow!("RelativePathBaseError: Only paths relative to the current working directory are supported.")),
                // If first component is "."
                Component::CurDir => Ok(components.collect()),
                // Otherwise, keep the full path
                _ => {
                    // Create a new PathBuf
                    let mut result = PathBuf::new();
                    // Add back the first component
                    result.push(first);
                    // Add all remaining components
                    result.extend(components);
                    Ok(result)
                }
            }
        } else {
            // Empty path
            Ok(PathBuf::new())
        }
    }

    pub fn fs_get(&mut self, path: String) -> anyhow::Result<PathBuf> {
        let p = PathBuf::from(&path);
        if p.exists() {
            return Ok(PathBuf::from(path));
        }

        if p.is_absolute() {
            return Err(anyhow::anyhow!("AbsolutePathError: Only paths relative to the current working directory are supported."));
        }

        let asset_rel_path = Self::chop_relative_base(p.clone())
            .unwrap_or_else(|_| panic!("AbsolutePathError: Only paths relative to the current working directory are supported."));

        let here = PathBuf::from(".").canonicalize()?;
        let artifact_at_root = self.base_path.clone().join(asset_rel_path);

        BenchmarkWorkDir::deep_copy(artifact_at_root.as_path(), here.as_path(), true)?;
        Ok(PathBuf::from(path))
    }

    fn deep_copy<P, Q>(src: P, dst: Q, recursive: bool) -> io::Result<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let src = src.as_ref();
        let dst = dst.as_ref();

        let mut cmd = Command::new("cp");

        // Add -r flag if recursive is true
        if recursive {
            cmd.arg("-r");
        }

        // Add source and destination paths
        cmd.arg(src).arg(dst);

        // Execute the command
        let output = cmd.output()?;

        if output.status.success() {
            Ok(())
        } else {
            let error_message = String::from_utf8_lossy(&output.stderr).to_string();
            Err(io::Error::new(ErrorKind::Other, error_message))
        }
    }
}
