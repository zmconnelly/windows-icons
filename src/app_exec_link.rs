use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    fs,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::MetadataExt,
    },
    path::{Path, PathBuf},
};

use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
            OPEN_EXISTING,
        },
        System::{IO::DeviceIoControl, Ioctl::FSCTL_GET_REPARSE_POINT},
    },
    core::PCWSTR,
};

/// Reparse tag of the app execution aliases in `%LOCALAPPDATA%\Microsoft\WindowsApps`.
const IO_REPARSE_TAG_APPEXECLINK: u32 = 0x8000_001B;

/// `MAXIMUM_REPARSE_DATA_BUFFER_SIZE`, the largest payload the file system will return.
const MAX_REPARSE_DATA_SIZE: usize = 16 * 1024;

/// `REPARSE_DATA_BUFFER` header: tag `u32`, data length `u16`, reserved `u16`.
const REPARSE_HEADER_SIZE: usize = 8;

struct AutoHandle(HANDLE);

impl Drop for AutoHandle {
    fn drop(&mut self) {
        if !self.0.0.is_null() {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

/// App execution aliases are zero-byte reparse points standing in for an installed UWP
/// app, so the icon has to come from the package they point at. Any other path is
/// returned unchanged, as is an alias whose target cannot be read.
pub fn resolve_app_exec_link(path: &Path) -> Cow<'_, Path> {
    match read_link_target(path) {
        Some(target) => Cow::Owned(target),
        None => Cow::Borrowed(path),
    }
}

fn read_link_target(path: &Path) -> Option<PathBuf> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 == 0 {
        return None;
    }

    let buffer = unsafe { read_reparse_data(path) }.ok()?;
    let (tag, data) = split_reparse_data(&buffer)?;
    if tag != IO_REPARSE_TAG_APPEXECLINK {
        return None;
    }

    let target = parse_app_exec_link_target(data)?;
    target.exists().then_some(target)
}

unsafe fn read_reparse_data(path: &Path) -> windows::core::Result<Vec<u8>> {
    let wide_path: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();

    // Opening with no access rights is enough to query the reparse data, and
    // FILE_FLAG_OPEN_REPARSE_POINT keeps the file system from following the link.
    let handle = unsafe {
        CreateFileW(
            PCWSTR::from_raw(wide_path.as_ptr()),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }?;
    let handle = AutoHandle(handle);

    // Held as u32s so the buffer is aligned for the header fields.
    let mut buffer = vec![0u32; MAX_REPARSE_DATA_SIZE / size_of::<u32>()];
    let mut bytes_returned = 0u32;
    unsafe {
        DeviceIoControl(
            handle.0,
            FSCTL_GET_REPARSE_POINT,
            None,
            0,
            Some(buffer.as_mut_ptr().cast()),
            MAX_REPARSE_DATA_SIZE as u32,
            Some(&mut bytes_returned),
            None,
        )
    }?;

    let len = (bytes_returned as usize).min(MAX_REPARSE_DATA_SIZE);
    let bytes = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), len) };

    Ok(bytes.to_vec())
}

fn split_reparse_data(buffer: &[u8]) -> Option<(u32, &[u8])> {
    let tag = u32::from_le_bytes(buffer.get(0..4)?.try_into().ok()?);
    let data_length = u16::from_le_bytes(buffer.get(4..6)?.try_into().ok()?) as usize;
    let data = buffer.get(REPARSE_HEADER_SIZE..REPARSE_HEADER_SIZE.checked_add(data_length)?)?;

    Some((tag, data))
}

/// The `AppExecLink` payload is a version `u32` followed by NUL-separated UTF-16
/// strings: package family name, application user model id, target executable and
/// application type. Only the executable is of interest here.
fn parse_app_exec_link_target(data: &[u8]) -> Option<PathBuf> {
    const TARGET_STRING_INDEX: usize = 2;

    let strings = data.get(size_of::<u32>()..)?;
    let utf16: Vec<u16> = strings
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();

    let target = utf16.split(|unit| *unit == 0).nth(TARGET_STRING_INDEX)?;
    if target.is_empty() {
        return None;
    }

    Some(PathBuf::from(OsString::from_wide(target)))
}

#[cfg(test)]
mod tests {
    use super::{
        IO_REPARSE_TAG_APPEXECLINK, REPARSE_HEADER_SIZE, parse_app_exec_link_target,
        split_reparse_data,
    };
    use std::path::PathBuf;

    fn app_exec_link_payload(strings: &[&str]) -> Vec<u8> {
        let mut payload = 3u32.to_le_bytes().to_vec();
        for string in strings {
            for unit in string.encode_utf16().chain(Some(0)) {
                payload.extend_from_slice(&unit.to_le_bytes());
            }
        }
        payload
    }

    fn reparse_buffer(tag: u32, data: &[u8]) -> Vec<u8> {
        let mut buffer = tag.to_le_bytes().to_vec();
        buffer.extend_from_slice(&(data.len() as u16).to_le_bytes());
        buffer.extend_from_slice(&0u16.to_le_bytes());
        buffer.extend_from_slice(data);
        buffer
    }

    #[test]
    fn reads_the_target_executable_from_an_app_exec_link() {
        let target = r"C:\Program Files\WindowsApps\Microsoft.DesktopAppInstaller_1.0.0.0_x64__8wekyb3d8bbwe\AppInstallerPythonRedirector.exe";
        let payload = app_exec_link_payload(&[
            "Microsoft.DesktopAppInstaller_8wekyb3d8bbwe",
            "Microsoft.DesktopAppInstaller_8wekyb3d8bbwe!PythonRedirector",
            target,
            "0",
        ]);
        let buffer = reparse_buffer(IO_REPARSE_TAG_APPEXECLINK, &payload);

        let (tag, data) = split_reparse_data(&buffer).unwrap();

        assert_eq!(tag, IO_REPARSE_TAG_APPEXECLINK);
        assert_eq!(
            parse_app_exec_link_target(data),
            Some(PathBuf::from(target))
        );
    }

    #[test]
    fn reports_the_tag_of_other_reparse_points() {
        const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;

        let buffer = reparse_buffer(IO_REPARSE_TAG_SYMLINK, &[0u8; 16]);
        let (tag, _) = split_reparse_data(&buffer).unwrap();

        assert_ne!(tag, IO_REPARSE_TAG_APPEXECLINK);
    }

    #[test]
    fn rejects_truncated_payloads() {
        assert!(split_reparse_data(&[0u8; 4]).is_none());

        let claims_more_data_than_it_has = {
            let mut buffer = IO_REPARSE_TAG_APPEXECLINK.to_le_bytes().to_vec();
            buffer.extend_from_slice(&64u16.to_le_bytes());
            buffer.extend_from_slice(&0u16.to_le_bytes());
            buffer.extend_from_slice(&[0u8; 8]);
            buffer
        };
        assert!(split_reparse_data(&claims_more_data_than_it_has).is_none());

        assert!(parse_app_exec_link_target(&[]).is_none());
        assert_eq!(REPARSE_HEADER_SIZE, 8);
    }

    #[test]
    fn rejects_payloads_without_a_target() {
        let payload = app_exec_link_payload(&["Package_8wekyb3d8bbwe", "Package!App"]);
        assert!(parse_app_exec_link_target(&payload).is_none());
    }
}
