use std::io::{self, Write};
use std::path::Path;

#[cfg(windows)]
fn create_owner_only_file(path: &Path) -> io::Result<std::fs::File> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{FromRawHandle, RawHandle};
    use std::ptr;
    use winapi::shared::minwindef::HLOCAL;
    use winapi::shared::sddl::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use winapi::um::fileapi::{CREATE_NEW, CreateFileW};
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::minwinbase::SECURITY_ATTRIBUTES;
    use winapi::um::winbase::LocalFree;
    use winapi::um::winnt::{
        FILE_ATTRIBUTE_TEMPORARY, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        GENERIC_READ, GENERIC_WRITE, PSECURITY_DESCRIPTOR,
    };

    let sddl: Vec<u16> = "D:P(A;;FA;;;OW)\0".encode_utf16().collect();
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();

    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1 as u32,
            &mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }

    let mut security_attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            &mut security_attributes,
            CREATE_NEW,
            FILE_ATTRIBUTE_TEMPORARY,
            ptr::null_mut(),
        )
    };
    let error = (handle == INVALID_HANDLE_VALUE).then(io::Error::last_os_error);

    unsafe {
        LocalFree(descriptor as HLOCAL);
    }
    if let Some(error) = error {
        Err(error)
    } else {
        Ok(unsafe { std::fs::File::from_raw_handle(handle as RawHandle) })
    }
}

#[cfg(windows)]
fn create_private_temporary_file(parent: &Path) -> io::Result<tempfile::NamedTempFile> {
    tempfile::Builder::new().make_in(parent, create_owner_only_file)
}

#[cfg(not(windows))]
fn create_private_temporary_file(parent: &Path) -> io::Result<tempfile::NamedTempFile> {
    tempfile::NamedTempFile::new_in(parent)
}

pub(crate) fn write_private_file(path: &Path, contents: &str) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "private file path must have a parent directory",
        )
    })?;
    std::fs::create_dir_all(parent)?;

    let mut temporary = create_private_temporary_file(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    temporary.write_all(contents.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs::File;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    #[test]
    fn replaces_loose_existing_file_with_private_inode() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("token.json");
        std::fs::write(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let old_file = File::open(&path).unwrap();
        let old_inode = old_file.metadata().unwrap().ino();

        write_private_file(&path, "new-secret").unwrap();

        let metadata = std::fs::metadata(&path).unwrap();
        assert_ne!(metadata.ino(), old_inode);
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "new-secret");
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use std::ffi::c_void;
    use std::fs::File;
    use std::os::windows::io::AsRawHandle;
    use std::ptr;
    use winapi::shared::minwindef::HLOCAL;
    use winapi::shared::winerror::ERROR_SUCCESS;
    use winapi::um::accctrl::SE_FILE_OBJECT;
    use winapi::um::aclapi::GetSecurityInfo;
    use winapi::um::securitybaseapi::{
        CreateWellKnownSid, EqualSid, GetAce, GetSecurityDescriptorControl,
    };
    use winapi::um::winbase::LocalFree;
    use winapi::um::winnt::{
        ACCESS_ALLOWED_ACE, ACCESS_ALLOWED_ACE_TYPE, DACL_SECURITY_INFORMATION, FILE_ALL_ACCESS,
        OWNER_SECURITY_INFORMATION, PACL, PSECURITY_DESCRIPTOR, PSID, SE_DACL_PROTECTED,
        SECURITY_MAX_SID_SIZE, WinCreatorOwnerRightsSid,
    };

    fn assert_owner_only_protected_dacl(file: &File) {
        let mut owner: PSID = ptr::null_mut();
        let mut dacl: PACL = ptr::null_mut();
        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
        let status = unsafe {
            GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                ptr::null_mut(),
                &mut dacl,
                ptr::null_mut(),
                &mut descriptor,
            )
        };
        assert_eq!(status, ERROR_SUCCESS);

        let mut control = 0;
        let mut revision = 0;
        assert_ne!(
            unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) },
            0
        );
        assert_ne!(control & SE_DACL_PROTECTED, 0);
        assert!(!dacl.is_null());
        assert_eq!(unsafe { (*dacl).AceCount }, 1);

        let mut ace: *mut c_void = ptr::null_mut();
        assert_ne!(unsafe { GetAce(dacl, 0, &mut ace) }, 0);
        let allowed = ace.cast::<ACCESS_ALLOWED_ACE>();
        assert_eq!(
            unsafe { (*allowed).Header.AceType },
            ACCESS_ALLOWED_ACE_TYPE
        );
        assert_eq!(
            unsafe { (*allowed).Mask } & FILE_ALL_ACCESS,
            FILE_ALL_ACCESS
        );

        let mut expected_sid = [0u8; SECURITY_MAX_SID_SIZE];
        let mut expected_sid_size = expected_sid.len() as u32;
        assert_ne!(
            unsafe {
                CreateWellKnownSid(
                    WinCreatorOwnerRightsSid,
                    ptr::null_mut(),
                    expected_sid.as_mut_ptr().cast(),
                    &mut expected_sid_size,
                )
            },
            0
        );
        let actual_sid = unsafe { &mut (*allowed).SidStart as *mut u32 as PSID };
        assert_ne!(
            unsafe { EqualSid(actual_sid, expected_sid.as_mut_ptr().cast()) },
            0
        );
        unsafe {
            LocalFree(descriptor as HLOCAL);
        }
    }

    #[test]
    fn creates_temporary_file_with_owner_only_protected_dacl() {
        let directory = tempfile::tempdir().unwrap();
        let temporary = create_private_temporary_file(directory.path()).unwrap();

        assert_owner_only_protected_dacl(temporary.as_file());
    }

    #[test]
    fn writes_owner_only_protected_dacl() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("token.json");

        write_private_file(&path, "new-secret").unwrap();

        let file = File::open(&path).unwrap();
        assert_owner_only_protected_dacl(&file);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "new-secret");
    }
}
