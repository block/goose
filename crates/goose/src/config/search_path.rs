use std::{env, ffi::OsString, path::PathBuf};

use crate::config::{Config, ConfigError};

pub fn search_path_var() -> Result<OsString, ConfigError> {
    let paths = Config::global()
        .get_goose_search_paths()
        .or_else(|err| match err {
            ConfigError::NotFound(_) => Ok(vec![]),
            err => Err(err),
        })?
        .into_iter()
        .map(|s| PathBuf::from(shellexpand::tilde(&s).as_ref()));

    env::join_paths(
        paths.chain(
            env::var_os("PATH")
                .as_ref()
                .map(env::split_paths)
                .into_iter()
                .flatten(),
        ),
    )
    .map_err(|e| ConfigError::DeserializeError(format!("{}", e)))
}
