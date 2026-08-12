use anyhow::Result;
use std::sync::OnceLock;

pub type StringParamResolver = fn(&'static str) -> Result<Option<String>>;
pub type BoolParamResolver = fn(&'static str) -> Result<Option<bool>>;

static STRING_PARAM_RESOLVER: OnceLock<StringParamResolver> = OnceLock::new();
static BOOL_PARAM_RESOLVER: OnceLock<BoolParamResolver> = OnceLock::new();

pub fn set_string_param_resolver(resolve_param: StringParamResolver) {
    let _ = STRING_PARAM_RESOLVER.set(resolve_param);
}

pub fn set_bool_param_resolver(resolve_param: BoolParamResolver) {
    let _ = BOOL_PARAM_RESOLVER.set(resolve_param);
}

pub fn string_param(key: &'static str) -> Result<Option<String>> {
    match STRING_PARAM_RESOLVER.get() {
        Some(resolve_param) => resolve_param(key),
        None => Ok(None),
    }
}

pub fn bool_param(key: &'static str) -> Result<Option<bool>> {
    match BOOL_PARAM_RESOLVER.get() {
        Some(resolve_param) => resolve_param(key),
        None => Ok(None),
    }
}
