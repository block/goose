use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

/// Values gosling itself exported, so real environment variables (which take
/// precedence over config) can be told apart from our own earlier exports.
static EXPORTED_BY_GOSLING: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// Export AWS_* keys from gosling config/secrets into the process environment so
/// `aws_config`'s default chain can resolve them.
///
/// `std::env::set_var` is a data race against any concurrent `getenv` on the
/// multi-threaded runtime (libc setenv/getenv are unsynchronized; this can
/// segfault), and providers are constructed per session and per subagent. To
/// keep config changes working without a restart while minimizing that
/// exposure, exports are serialized, never overwrite variables gosling did not
/// set itself, and only touch the environment when a value actually changed —
/// steady-state provider construction performs no environment mutation.
pub(crate) fn export_aws_env<I>(sources: I)
where
    I: IntoIterator<Item = HashMap<String, Value>>,
{
    let mut exported = EXPORTED_BY_GOSLING
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let exported = exported.get_or_insert_with(HashMap::new);

    for map in sources {
        for (key, value) in map {
            if !key.starts_with("AWS_") {
                continue;
            }
            let Some(value) = value.as_str() else {
                continue;
            };
            match exported.get(&key) {
                None => {
                    if std::env::var_os(&key).is_none() {
                        std::env::set_var(&key, value);
                        exported.insert(key, value.to_string());
                    }
                }
                Some(previous) if previous != value => {
                    std::env::set_var(&key, value);
                    exported.insert(key, value.to_string());
                }
                _ => {}
            }
        }
    }
}
