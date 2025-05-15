use anyhow::Context;
use chrono::Local;
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

pub static BUILTIN_EVAL_ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets");

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BenchmarkWorkDir {
    pub base_path: PathBuf,
    pub run_dir: PathBuf,
    pub cwd: PathBuf,
    pub run_id: Option<String>,
}

impl Default for BenchmarkWorkDir {
    fn default() -> Self {
        Self::new("work_dir".to_string(), Vec::new())
    }
}

impl BenchmarkWorkDir {
    pub fn new(work_dir_name: String, include_dirs: Vec<PathBuf>) -> Self {
        let run_dir = std::env::current_dir().unwrap().canonicalize().unwrap();
        let base_path = PathBuf::from(format!("./{}", work_dir_name));
        fs::create_dir_all(&base_path).unwrap();

        let base_path = PathBuf::from(&base_path).canonicalize().unwrap();

        // abs paths from dir-strings
        let dirs = Self::canonical_dirs(include_dirs);

        // deep copy each dir
        let _: Vec<_> = dirs
            .iter()
            .map(|d| Self::deep_copy(d.as_path(), base_path.as_path(), true))
            .collect();

        Self::copy_auto_included_dirs(&base_path);

        std::env::set_current_dir(&base_path).unwrap();

        BenchmarkWorkDir {
            base_path: base_path.clone(),
            run_dir,
            cwd: base_path.clone(),
            run_id: None,
        }
    }

    pub fn init_experiment(output_dir: PathBuf) -> anyhow::Result<()> {
        if !output_dir.is_absolute() {
            anyhow::bail!(
                "Internal Error: init_experiment received a non-absolute path: {}",
                output_dir.display()
            );
        }

        // create experiment folder
        let current_time = Local::now().format("%H:%M:%S").to_string();
        let current_date = Local::now().format("%Y-%m-%d").to_string();
        let exp_folder_name = format!("benchmark-{}-{}", &current_date, &current_time);
        let base_path = output_dir.join(exp_folder_name);

        fs::create_dir_all(&base_path).with_context(|| {
            format!(
                "Failed to create benchmark directory: {}",
                base_path.display()
            )
        })?;
        std::env::set_current_dir(&base_path).with_context(|| {
            format!(
                "Failed to change working directory to: {}",
                base_path.display()
            )
        })?;
        Ok(())
    }

    pub fn canonical_dirs(include_dirs: Vec<PathBuf>) -> Vec<PathBuf> {
        include_dirs
            .iter()
            .map(|d| {
                let canon = d.canonicalize();
                if canon.is_err() {
                    eprintln!("{:?} can't be canonicalized", d);
                    panic!();
                }
                canon.unwrap()
            })
            .collect::<Vec<_>>()
    }
    fn copy_auto_included_dirs(dest: &Path) {
        let mut assets_dest = dest.to_path_buf();
        assets_dest.push("assets");
        if !assets_dest.exists() {
            fs::create_dir_all(&assets_dest).unwrap();
        }
        BUILTIN_EVAL_ASSETS.extract(assets_dest).unwrap();
    }
    pub fn cd(&mut self, path: PathBuf) -> anyhow::Result<&mut Self> {
        fs::create_dir_all(&path)?;
        std::env::set_current_dir(&path)?;
        self.cwd = path;
        Ok(self)
    }
    pub(crate) fn _run_dir(&mut self) -> Option<PathBuf> {
        if let Some(run_id) = &self.run_id {
            let mut eval_dir = self.base_path.clone();
            eval_dir.push(run_id);
            return Some(eval_dir);
        }
        None
    }

    pub fn set_eval(&mut self, eval: &str, run_id: String) {
        self.run_id = Some(run_id.clone());

        let eval = eval.replace(":", std::path::MAIN_SEPARATOR_STR);
        let mut eval_dir = self.base_path.clone();
        eval_dir.push(run_id);
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

        Self::deep_copy(artifact_at_root.as_path(), here.as_path(), true)?;
        Ok(PathBuf::from(path))
    }

    pub(crate) fn deep_copy<P, Q>(src: P, dst: Q, recursive: bool) -> io::Result<()>
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
            Err(io::Error::other(error_message))
        }
    }

    pub fn save(&self) {
        let work_dir = serde_json::to_string_pretty(&self).unwrap();
        fs::write("work_dir.json", work_dir).expect("Unable to write work-dir as file");
    }
}

impl Drop for BenchmarkWorkDir {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.run_dir).unwrap();
    }
}
