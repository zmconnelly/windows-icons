use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};

use windows::Win32::System::{
    ProcessStatus::K32GetModuleFileNameExW,
    Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
};

use crate::utils::guards::AutoHandle;

pub fn get_process_path(process_id: u32) -> Result<PathBuf, windows::core::Error> {
    let process_handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            process_id,
        )
    }?;
    let process_handle = AutoHandle(process_handle);

    let mut buffer = vec![0u16; 1024];
    let size = unsafe { K32GetModuleFileNameExW(Some(process_handle.0), None, &mut buffer) };
    if size == 0 {
        return Err(windows::core::Error::from_thread());
    }

    buffer.truncate(size as usize);

    Ok(PathBuf::from(OsString::from_wide(&buffer)))
}
