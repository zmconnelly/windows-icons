use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};

use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::{
        ProcessStatus::K32GetModuleFileNameExW,
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
};

pub fn get_process_path(process_id: u32) -> Result<PathBuf, windows::core::Error> {
    unsafe {
        let process_handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            process_id,
        )?;
        let mut buffer = vec![0u16; 1024];
        let size = K32GetModuleFileNameExW(Some(HANDLE(process_handle.0)), None, &mut buffer);
        CloseHandle(process_handle).map_err(|_| {
            windows::core::Error::new(
                windows::core::HRESULT(-1),
                "failed to close process handle.",
            )
        })?;

        if size == 0 {
            return Err(windows::core::Error::from_thread());
        }

        buffer.truncate(size as usize);
        let path = PathBuf::from(OsString::from_wide(&buffer));

        Ok(path)
    }
}
