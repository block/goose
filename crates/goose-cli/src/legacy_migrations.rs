#[cfg(unix)]
use goose::config::paths::Paths;

pub(crate) fn repair_legacy_project_tracker_permissions() {
    #[cfg(unix)]
    if let Err(error) =
        unix::restrict_legacy_project_tracker_file(&Paths::in_data_dir("projects.json"))
    {
        tracing::warn!(
            error = %error,
            "Could not restrict permissions on the legacy project tracker"
        );
    }
}

#[cfg(unix)]
mod unix {
    use anyhow::{bail, Context, Result};
    #[cfg(any(test, not(target_os = "macos")))]
    use std::fs::Permissions;
    use std::fs::{self, File, OpenOptions};
    #[cfg(target_os = "macos")]
    use std::os::fd::AsRawFd;
    #[cfg(any(test, not(target_os = "macos")))]
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::path::Path;

    #[derive(Debug, Eq, PartialEq)]
    enum RestrictionOutcome {
        Absent,
        AlreadyPrivate,
        Restricted,
        Skipped,
    }

    pub(super) fn restrict_legacy_project_tracker_file(path: &Path) -> Result<()> {
        restrict_legacy_project_tracker_file_with_outcome(path).map(|_| ())
    }

    fn restrict_legacy_project_tracker_file_with_outcome(
        path: &Path,
    ) -> Result<RestrictionOutcome> {
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(RestrictionOutcome::Absent);
            }
            Err(error) if error.raw_os_error() == Some(libc::ELOOP) => {
                return Ok(RestrictionOutcome::Skipped);
            }
            Err(error) => {
                return Err(error).with_context(|| format!("failed to open {}", path.display()));
            }
        };

        let before = file.metadata()?;
        if !safe_to_restrict(&before) {
            return Ok(RestrictionOutcome::Skipped);
        }

        let already_private = before.mode() & 0o7777 == 0o600;
        #[cfg(target_os = "macos")]
        restrict_legacy_file_descriptor(file.as_raw_fd(), 0o600)?;
        #[cfg(not(target_os = "macos"))]
        if !already_private {
            file.set_permissions(Permissions::from_mode(0o600))?;
        }

        verify_restricted_file(path, &file, &before)?;

        Ok(if already_private {
            RestrictionOutcome::AlreadyPrivate
        } else {
            RestrictionOutcome::Restricted
        })
    }

    fn safe_to_restrict(metadata: &fs::Metadata) -> bool {
        safe_file_metadata(
            metadata.is_file(),
            metadata.uid(),
            metadata.nlink(),
            unsafe { libc::geteuid() },
        )
    }

    fn safe_file_metadata(is_file: bool, uid: u32, link_count: u64, effective_uid: u32) -> bool {
        is_file && uid == effective_uid && link_count == 1
    }

    #[cfg(target_os = "macos")]
    fn restrict_legacy_file_descriptor(fd: std::os::fd::RawFd, mode: libc::mode_t) -> Result<()> {
        use std::io::{Error, ErrorKind};

        mod apple {
            use libc::{c_int, c_void, mode_t};

            pub(super) type FileSec = *mut c_void;
            pub(super) type Acl = *mut c_void;
            pub(super) type AclEntry = *mut c_void;

            pub(super) const FILESEC_MODE: c_int = 4;
            pub(super) const FILESEC_ACL: c_int = 5;
            pub(super) const ACL_TYPE_EXTENDED: c_int = 0x100;
            pub(super) const ACL_FIRST_ENTRY: c_int = 0;

            unsafe extern "C" {
                pub(super) fn filesec_init() -> FileSec;
                pub(super) fn filesec_free(filesec: FileSec);
                pub(super) fn filesec_set_property(
                    filesec: FileSec,
                    property: c_int,
                    value: *const c_void,
                ) -> c_int;
                pub(super) fn fchmodx_np(fd: c_int, filesec: FileSec) -> c_int;
                pub(super) fn acl_get_fd_np(fd: c_int, acl_type: c_int) -> Acl;
                pub(super) fn acl_get_entry(
                    acl: Acl,
                    entry_id: c_int,
                    entry: *mut AclEntry,
                ) -> c_int;
                pub(super) fn acl_free(object: *mut c_void) -> c_int;
            }

            pub(super) struct OwnedFileSec(pub(super) FileSec);

            impl Drop for OwnedFileSec {
                fn drop(&mut self) {
                    unsafe { filesec_free(self.0) };
                }
            }

            pub(super) struct OwnedAcl(pub(super) Acl);

            impl Drop for OwnedAcl {
                fn drop(&mut self) {
                    unsafe { acl_free(self.0) };
                }
            }

            pub(super) fn mode_pointer(mode: &mode_t) -> *const c_void {
                std::ptr::from_ref(mode).cast()
            }

            pub(super) fn remove_acl_pointer() -> *const c_void {
                std::ptr::dangling()
            }
        }

        let filesec = unsafe { apple::filesec_init() };
        if filesec.is_null() {
            return Err(Error::last_os_error().into());
        }
        let filesec = apple::OwnedFileSec(filesec);
        if unsafe {
            apple::filesec_set_property(filesec.0, apple::FILESEC_MODE, apple::mode_pointer(&mode))
        } < 0
        {
            return Err(Error::last_os_error().into());
        }
        if unsafe {
            apple::filesec_set_property(filesec.0, apple::FILESEC_ACL, apple::remove_acl_pointer())
        } < 0
        {
            return Err(Error::last_os_error().into());
        }

        if unsafe { apple::fchmodx_np(fd, filesec.0) } < 0 {
            let error = Error::last_os_error();
            if error.raw_os_error() != Some(libc::EOPNOTSUPP) {
                return Err(error.into());
            }
            if unsafe { libc::fchmod(fd, mode) } < 0 {
                return Err(Error::last_os_error().into());
            }
        }

        let mut metadata: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstat(fd, &mut metadata) } < 0 {
            return Err(Error::last_os_error().into());
        }
        if metadata.st_mode & 0o7777 != mode {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "legacy project tracker permissions were not restricted",
            )
            .into());
        }

        let acl = unsafe { apple::acl_get_fd_np(fd, apple::ACL_TYPE_EXTENDED) };
        if acl.is_null() {
            let error = Error::last_os_error();
            if matches!(
                error.raw_os_error(),
                Some(libc::EOPNOTSUPP) | Some(libc::ENOENT)
            ) {
                return Ok(());
            }
            return Err(error.into());
        }
        let acl = apple::OwnedAcl(acl);
        let mut entry = std::ptr::null_mut();
        if unsafe { apple::acl_get_entry(acl.0, apple::ACL_FIRST_ENTRY, &mut entry) } == 0 {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                "legacy project tracker extended ACL was not removed",
            )
            .into());
        }
        let error = Error::last_os_error();
        if !matches!(
            error.raw_os_error(),
            Some(libc::EINVAL) | Some(libc::ENOENT)
        ) {
            return Err(error.into());
        }

        Ok(())
    }

    fn verify_restricted_file(path: &Path, file: &File, before: &fs::Metadata) -> Result<()> {
        let opened = file.metadata()?;
        let current = fs::symlink_metadata(path)?;

        if opened.dev() != before.dev()
            || opened.ino() != before.ino()
            || current.dev() != opened.dev()
            || current.ino() != opened.ino()
            || !safe_to_restrict(&opened)
            || !safe_to_restrict(&current)
            || opened.mode() & 0o7777 != 0o600
            || current.mode() & 0o7777 != 0o600
        {
            bail!("legacy project tracker changed while restricting permissions");
        }

        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::io::Write;
        use tempfile::tempdir;

        #[cfg(target_os = "macos")]
        fn add_everyone_read_acl(path: &Path) {
            let status = std::process::Command::new("/bin/chmod")
                .arg("+a")
                .arg("everyone allow read")
                .arg(path)
                .status()
                .unwrap();
            assert!(status.success());
        }

        #[cfg(target_os = "macos")]
        fn descriptor_has_extended_acl(file: &File) -> bool {
            use libc::{c_int, c_void};

            unsafe extern "C" {
                fn acl_get_fd_np(fd: c_int, acl_type: c_int) -> *mut c_void;
                fn acl_get_entry(
                    acl: *mut c_void,
                    entry_id: c_int,
                    entry: *mut *mut c_void,
                ) -> c_int;
                fn acl_free(object: *mut c_void) -> c_int;
            }

            let acl = unsafe { acl_get_fd_np(file.as_raw_fd(), 0x100) };
            if acl.is_null() {
                let error = std::io::Error::last_os_error();
                assert!(matches!(
                    error.raw_os_error(),
                    Some(libc::ENOENT) | Some(libc::EOPNOTSUPP)
                ));
                return false;
            }
            let mut entry = std::ptr::null_mut();
            let has_entry = unsafe { acl_get_entry(acl, 0, &mut entry) } == 0;
            unsafe { acl_free(acl) };
            has_entry
        }

        fn create_file(path: &Path, mode: u32, contents: &[u8]) {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(mode)
                .open(path)
                .unwrap();
            file.write_all(contents).unwrap();
            file.set_permissions(Permissions::from_mode(mode)).unwrap();
        }

        #[test]
        fn restricts_legacy_file_without_changing_contents() {
            let directory = tempdir().unwrap();
            let path = directory.path().join("projects.json");
            let contents = br#"{"last_instruction":"secret"}"#;
            create_file(&path, 0o644, contents);
            #[cfg(target_os = "macos")]
            {
                add_everyone_read_acl(&path);
                assert!(descriptor_has_extended_acl(
                    &OpenOptions::new().read(true).open(&path).unwrap()
                ));
            }

            assert_eq!(
                restrict_legacy_project_tracker_file_with_outcome(&path).unwrap(),
                RestrictionOutcome::Restricted
            );
            assert_eq!(fs::read(&path).unwrap(), contents);
            assert_eq!(fs::metadata(&path).unwrap().mode() & 0o7777, 0o600);
            #[cfg(target_os = "macos")]
            assert!(!descriptor_has_extended_acl(
                &OpenOptions::new().read(true).open(&path).unwrap()
            ));
        }

        #[test]
        fn leaves_private_legacy_file_private() {
            let directory = tempdir().unwrap();
            let path = directory.path().join("projects.json");
            create_file(&path, 0o600, b"private");
            #[cfg(target_os = "macos")]
            {
                add_everyone_read_acl(&path);
                assert!(descriptor_has_extended_acl(
                    &OpenOptions::new().read(true).open(&path).unwrap()
                ));
            }

            assert_eq!(
                restrict_legacy_project_tracker_file_with_outcome(&path).unwrap(),
                RestrictionOutcome::AlreadyPrivate
            );
            assert_eq!(fs::read(&path).unwrap(), b"private");
            assert_eq!(fs::metadata(&path).unwrap().mode() & 0o7777, 0o600);
            #[cfg(target_os = "macos")]
            assert!(!descriptor_has_extended_acl(
                &OpenOptions::new().read(true).open(&path).unwrap()
            ));
        }

        #[test]
        fn missing_legacy_file_is_a_no_op() {
            let directory = tempdir().unwrap();
            let path = directory.path().join("projects.json");

            assert_eq!(
                restrict_legacy_project_tracker_file_with_outcome(&path).unwrap(),
                RestrictionOutcome::Absent
            );
        }

        #[test]
        fn skips_symlinks_without_changing_the_target() {
            use std::os::unix::fs::symlink;

            let directory = tempdir().unwrap();
            let target = directory.path().join("target.json");
            let path = directory.path().join("projects.json");
            create_file(&target, 0o644, b"outside");
            symlink(&target, &path).unwrap();

            assert_eq!(
                restrict_legacy_project_tracker_file_with_outcome(&path).unwrap(),
                RestrictionOutcome::Skipped
            );
            assert_eq!(fs::read(&target).unwrap(), b"outside");
            assert_eq!(fs::metadata(&target).unwrap().mode() & 0o7777, 0o644);
        }

        #[test]
        fn skips_hard_links_without_changing_the_inode() {
            let directory = tempdir().unwrap();
            let target = directory.path().join("target.json");
            let path = directory.path().join("projects.json");
            create_file(&target, 0o644, b"shared");
            fs::hard_link(&target, &path).unwrap();

            assert_eq!(
                restrict_legacy_project_tracker_file_with_outcome(&path).unwrap(),
                RestrictionOutcome::Skipped
            );
            assert_eq!(fs::read(&target).unwrap(), b"shared");
            assert_eq!(fs::metadata(&target).unwrap().mode() & 0o7777, 0o644);
            assert_eq!(fs::metadata(&path).unwrap().mode() & 0o7777, 0o644);
        }

        #[test]
        fn only_current_user_owned_regular_single_link_files_are_safe_to_restrict() {
            assert!(safe_file_metadata(true, 501, 1, 501));
            assert!(!safe_file_metadata(false, 501, 1, 501));
            assert!(!safe_file_metadata(true, 502, 1, 501));
            assert!(!safe_file_metadata(true, 501, 2, 501));
        }
    }
}
