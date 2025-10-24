use std::{env, ffi::OsString, path::PathBuf};

use crate::config::{Config, ConfigError};

pub fn search_path_var() -> Result<OsString, ConfigError> {
    let mut paths: Vec<_> = Config::global()
        .get_goose_search_paths()
        .or_else(|err| match err {
            ConfigError::NotFound(_) => Ok(vec![]),
            err => Err(err),
        })?
        .into_iter()
        .map(PathBuf::from)
        .collect();

    if let Some(existing) = env::var_os("PATH") {
        paths.extend(env::split_paths(&existing));
    }

    env::join_paths(paths).map_err(|e| ConfigError::DeserializeError(format!("{}", e)))
}
