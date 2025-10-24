use std::{env, ffi::OsString, path::PathBuf};

use crate::config::{Config, ConfigError};

pub fn search_path_var() -> Result<OsString, ConfigError> {
    let mut paths: Vec<_> = env::var_os("PATH")
        .map(|p| env::split_paths(&p).collect())
        .unwrap_or_default();

    let to_add = Config::global()
        .get_goose_search_paths()
        .or_else(|err| match err {
            ConfigError::NotFound(_) => Ok(vec![]),
            err => Err(err),
        })?;

    paths.extend(to_add.into_iter().map(PathBuf::from));

    env::join_paths(paths).map_err(|e| ConfigError::DeserializeError(format!("{}", e)))
}
