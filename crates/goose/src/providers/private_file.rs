use std::io::{self, Write};
use std::path::Path;

#[cfg(windows)]
fn restrict_to_owner(file: &std::fs::File) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use std::ptr;
    use winapi::shared::minwindef::HLOCAL;
    use winapi::shared::sddl::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use winapi::shared::winerror::ERROR_SUCCESS;
    use winapi::um::accctrl::SE_FILE_OBJECT;
    use winapi::um::aclapi::SetSecurityInfo;
    use winapi::um::securitybaseapi::GetSecurityDescriptorDacl;
    use winapi::um::winbase::LocalFree;
    use winapi::um::winnt::{
        DACL_SECURITY_INFORMATION, PACL, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    };

    let sddl: Vec<u16> = "D:P(A;;FA;;;OW)\0".encode_utf16().collect();
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();

    // The descriptor grants full access only to the file owner and protects the
    // DACL from inheriting broader permissions from a shared parent directory.
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

    let result = (|| -> io::Result<()> {
        let mut dacl_present = 0;
        let mut dacl: PACL = ptr::null_mut();
        let mut dacl_defaulted = 0;
        if unsafe {
            GetSecurityDescriptorDacl(
                descriptor,
                &mut dacl_present,
                &mut dacl,
                &mut dacl_defaulted,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if dacl_present == 0 || dacl.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "owner-only security descriptor has no DACL",
            ));
        }

        let status = unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                dacl,
                ptr::null_mut(),
            )
        };
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        Ok(())
    })();

    unsafe {
        LocalFree(descriptor as HLOCAL);
    }
    result
}

pub(crate) fn write_private_file(path: &Path, contents: &str) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "private file path must have a parent directory",
        )
    })?;
    std::fs::create_dir_all(parent)?;

    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(windows)]
    restrict_to_owner(temporary.as_file())?;
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
    use winapi::um::securitybaseapi::{GetAce, GetSecurityDescriptorControl};
    use winapi::um::winbase::LocalFree;
    use winapi::um::winnt::{
        ACCESS_ALLOWED_ACE, ACCESS_ALLOWED_ACE_TYPE, DACL_SECURITY_INFORMATION, FILE_ALL_ACCESS,
        OWNER_SECURITY_INFORMATION, PACL, PSECURITY_DESCRIPTOR, PSID, SE_DACL_PROTECTED,
    };

    #[test]
    fn writes_owner_only_protected_dacl() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("token.json");

        write_private_file(&path, "new-secret").unwrap();

        let file = File::open(&path).unwrap();
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
        assert_eq!(std::fs::read_to_string(path).unwrap(), "new-secret");

        unsafe {
            LocalFree(descriptor as HLOCAL);
        }
    }
}
