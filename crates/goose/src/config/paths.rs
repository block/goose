use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
#[cfg(all(feature = "system-keyring", unix, not(target_os = "redox")))]
use std::ffi::CStr;
use std::ffi::OsString;
#[cfg(all(feature = "system-keyring", unix, not(target_os = "redox")))]
use std::mem::MaybeUninit;
#[cfg(all(feature = "system-keyring", unix, not(target_os = "redox")))]
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
#[cfg(all(feature = "system-keyring", unix, not(target_os = "redox")))]
use std::ptr;

#[cfg(all(feature = "system-keyring", unix, not(target_os = "redox")))]
fn os_user_home_dir() -> Option<PathBuf> {
    // SAFETY: sysconf reads a process-global constant and does not dereference pointers.
    let buffer_size = unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) };
    let buffer_size = if buffer_size < 0 {
        512
    } else {
        buffer_size as usize
    };
    let mut buffer = vec![0_u8; buffer_size];
    let mut passwd = MaybeUninit::<libc::passwd>::uninit();
    let mut result = ptr::null_mut();

    // SAFETY: passwd and buffer are valid writable allocations for the duration of the call.
    let status = unsafe {
        libc::getpwuid_r(
            libc::getuid(),
            passwd.as_mut_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            &mut result,
        )
    };
    if status != 0 || result.is_null() {
        return None;
    }

    // SAFETY: a successful getpwuid_r initialized passwd.
    let passwd = unsafe { passwd.assume_init() };
    if passwd.pw_dir.is_null() {
        return None;
    }
    // SAFETY: pw_dir is a non-null, null-terminated string owned by passwd's buffer.
    let bytes = unsafe { CStr::from_ptr(passwd.pw_dir) }.to_bytes();
    (!bytes.is_empty()).then(|| PathBuf::from(OsString::from_vec(bytes.to_vec())))
}

#[cfg(all(feature = "system-keyring", any(not(unix), target_os = "redox")))]
fn os_user_home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

pub struct Paths;

impl Paths {
    fn app_strategy() -> impl AppStrategy {
        // NOTE: "Block" is kept here for backwards compatibility with existing
        // user config/data directories (e.g. ~/Library/Application Support/Block/goose/).
        // Changing this would orphan existing installations.
        choose_app_strategy(AppStrategyArgs {
            top_level_domain: "Block".to_string(),
            author: "Block".to_string(),
            app_name: "goose".to_string(),
        })
        .expect("goose requires a home dir")
    }

    fn get_dir(dir_type: DirType) -> PathBuf {
        if let Some(base) = Self::path_root() {
            match dir_type {
                DirType::Config => base.join("config"),
                DirType::Data => base.join("data"),
                DirType::State => base.join("state"),
                DirType::Plugins => base.join(".agents").join("plugins"),
                DirType::Agents => base.join(".agents").join("agents"),
                DirType::AgentsHome => base.join(".agents"),
            }
        } else {
            let strategy = Self::app_strategy();

            match dir_type {
                DirType::Config => strategy.config_dir(),
                DirType::Data => strategy.data_dir(),
                DirType::State => strategy.state_dir().unwrap_or(strategy.data_dir()),
                DirType::Plugins => strategy.home_dir().join(".agents").join("plugins"),
                DirType::Agents => strategy.home_dir().join(".agents").join("agents"),
                DirType::AgentsHome => strategy.home_dir().join(".agents"),
            }
        }
    }

    pub(crate) fn path_root() -> Option<PathBuf> {
        Self::validated_path_root(std::env::var_os("GOOSE_PATH_ROOT"))
    }

    fn validated_path_root(value: Option<OsString>) -> Option<PathBuf> {
        value.map(PathBuf::from).filter(|path| path.is_absolute())
    }

    pub fn config_dir() -> PathBuf {
        Self::get_dir(DirType::Config)
    }

    #[cfg(feature = "system-keyring")]
    pub(crate) fn os_user_home_dir() -> PathBuf {
        os_user_home_dir().expect("goose requires an OS user home dir")
    }

    pub fn data_dir() -> PathBuf {
        Self::get_dir(DirType::Data)
    }

    pub fn state_dir() -> PathBuf {
        Self::get_dir(DirType::State)
    }

    pub fn plugins_dir() -> PathBuf {
        Self::get_dir(DirType::Plugins)
    }

    pub fn agents_dir() -> PathBuf {
        Self::get_dir(DirType::Agents)
    }

    pub fn agents_home_dir() -> PathBuf {
        Self::get_dir(DirType::AgentsHome)
    }

    pub fn in_agents_home_dir(subpath: &str) -> PathBuf {
        Self::agents_home_dir().join(subpath)
    }

    pub fn in_state_dir(subpath: &str) -> PathBuf {
        Self::state_dir().join(subpath)
    }

    pub fn in_config_dir(subpath: &str) -> PathBuf {
        Self::config_dir().join(subpath)
    }

    pub fn in_data_dir(subpath: &str) -> PathBuf {
        Self::data_dir().join(subpath)
    }
}

enum DirType {
    Config,
    Data,
    State,
    Plugins,
    Agents,
    AgentsHome,
}

#[cfg(test)]
mod tests {
    use super::Paths;
    use std::ffi::OsString;

    #[test]
    fn path_root_requires_an_absolute_path() {
        assert_eq!(Paths::validated_path_root(None), None);
        assert_eq!(Paths::validated_path_root(Some(OsString::new())), None);
        assert_eq!(
            Paths::validated_path_root(Some(OsString::from("relative/root"))),
            None
        );

        let absolute = std::env::current_dir()
            .unwrap()
            .join("nonexistent-goose-root");
        assert_eq!(
            Paths::validated_path_root(Some(absolute.clone().into_os_string())),
            Some(absolute)
        );
    }
}
