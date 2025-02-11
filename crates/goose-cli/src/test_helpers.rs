/// Helper function to set up a temporary home directory for testing, returns path of that temp dir.
/// Also creates a default profiles.json to avoid obscure test failures when there are no profiles.
#[cfg(test)]
pub fn run_with_tmp_dir<F: FnOnce() -> T, T>(func: F) -> T {
    use std::ffi::OsStr;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_path_buf();
    setup_profile(&temp_dir_path, None);

    temp_env::with_vars(
        [
            ("HOME", Some(temp_dir_path.as_os_str())),
            ("DATABRICKS_HOST", Some(OsStr::new("tmp_host_url"))),
        ],
        func,
    )
}

#[cfg(test)]
#[allow(dead_code)]
pub async fn run_with_tmp_dir_async<F, Fut, T>(func: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    use std::ffi::OsStr;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_path_buf();
    setup_profile(&temp_dir_path, None);

    temp_env::async_with_vars(
        [
            ("HOME", Some(temp_dir_path.as_os_str())),
            ("DATABRICKS_HOST", Some(OsStr::new("tmp_host_url"))),
        ],
        func(),
    )
    .await
}

#[cfg(test)]
use std::path::Path;

#[cfg(test)]
/// Setup a goose profile for testing, and an optional profile string
fn setup_profile(temp_dir_path: &Path, profile_string: Option<&str>) {
    use std::fs;

    let profile_path = temp_dir_path
        .join(".config")
        .join("goose")
        .join("profiles.json");
    fs::create_dir_all(profile_path.parent().unwrap()).unwrap();
    let default_profile = r#"
{
    "profile_items": {
        "default": {
            "provider": "databricks",
            "model": "goose",
            "additional_extensions": []
        }
    }
}"#;

    fs::write(&profile_path, profile_string.unwrap_or(default_profile)).unwrap();
}
