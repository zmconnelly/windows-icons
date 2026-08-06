use crate::utils::image_utils::hicon_to_image;

use std::{
    error::Error,
    ffi::OsStr,
    io::{self, ErrorKind},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use image::RgbaImage;
use windows::{
    Win32::{
        Foundation::{FreeLibrary, HANDLE, HMODULE},
        System::LibraryLoader::{GetModuleHandleW, LoadLibraryW},
        UI::{
            Shell::ExtractIconW,
            WindowsAndMessaging::{HICON, IMAGE_ICON, LR_CREATEDIBSECTION, LoadImageW},
        },
    },
    core::{HSTRING, PCWSTR},
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum DllResource {
    System(String, u32),
    Other(PathBuf, String, u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DllIcon(Option<DllResource>);

impl Default for DllIcon {
    fn default() -> Self {
        Self::new()
    }
}

impl DllIcon {
    pub fn new() -> Self {
        DllIcon(None)
    }

    pub fn with_resource<P: AsRef<Path>>(self, path: P, name: &str, size: u32) -> Self {
        let path = path.as_ref().to_path_buf();
        DllIcon(Some(DllResource::Other(path, name.to_owned(), size)))
    }

    pub fn with_shell32(self, index: u32) -> Self {
        DllIcon(Some(DllResource::System("shell32.dll".to_owned(), index)))
    }

    pub fn with_imageres(self, index: u32) -> Self {
        DllIcon(Some(DllResource::System("imageres.dll".to_owned(), index)))
    }

    pub fn with_ddores(self, index: u32) -> Self {
        DllIcon(Some(DllResource::System("ddores.dll".to_owned(), index)))
    }

    pub fn with_mmres(self, index: u32) -> Self {
        DllIcon(Some(DllResource::System("mmres.dll".to_owned(), index)))
    }

    pub fn with_wmploc(self, index: u32) -> Self {
        DllIcon(Some(DllResource::System("wmploc.dll".to_owned(), index)))
    }

    pub fn with_dmdskres(self, index: u32) -> Self {
        DllIcon(Some(DllResource::System("dmdskres.dll".to_owned(), index)))
    }

    pub fn with_setupapi(self, index: u32) -> Self {
        DllIcon(Some(DllResource::System("setupapi.dll".to_owned(), index)))
    }

    pub fn with_explorer(self, index: u32) -> Self {
        DllIcon(Some(DllResource::System("explorer.exe".to_owned(), index)))
    }

    pub fn with_imagesp1(self, index: u32) -> Self {
        DllIcon(Some(DllResource::System("imagesp1.dll".to_owned(), index)))
    }

    pub fn with_pifmgr(self, index: u32) -> Self {
        DllIcon(Some(DllResource::System("pifmgr.dll".to_owned(), index)))
    }

    pub fn with_networkexplorer(self, index: u32) -> Self {
        DllIcon(Some(DllResource::System(
            "networkexplorer.dll".to_owned(),
            index,
        )))
    }
}

struct AutoModule(HMODULE);

impl Drop for AutoModule {
    fn drop(&mut self) {
        if !self.0.0.is_null() {
            let _ = unsafe { FreeLibrary(self.0) };
        }
    }
}

#[allow(non_snake_case)]
fn MAKEINTRESOURCEW(id: i32) -> PCWSTR {
    unsafe { std::mem::transmute::<_, PCWSTR>(id as usize) }
}

pub fn get_dll_hicon_to_image(dll_icon: DllIcon) -> Result<RgbaImage, Box<dyn Error>> {
    let hicon = unsafe { get_dll_hicon(dll_icon) }?;
    unsafe { hicon_to_image(hicon) }
}

unsafe fn get_hicon_handle(
    dll_name: &HSTRING,
    name: PCWSTR,
    width: u32,
    height: u32,
) -> windows::core::Result<HANDLE> {
    let w = i32::try_from(width)?;
    let h = i32::try_from(height)?;

    let mut module_handle = unsafe { GetModuleHandleW(dll_name) }?;
    let mut _module_guard = None;

    if module_handle.is_invalid() {
        module_handle = unsafe { LoadLibraryW(dll_name) }?;
        _module_guard = Some(AutoModule(module_handle));
    }

    unsafe {
        LoadImageW(
            Some(module_handle.into()),
            name,
            IMAGE_ICON,
            w,
            h,
            LR_CREATEDIBSECTION,
        )
    }
}

unsafe fn get_dll_hicon(dll_icon: DllIcon) -> Result<HICON, Box<dyn Error>> {
    let resource = dll_icon
        .0
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "no dll resources added"))?;

    match resource {
        DllResource::System(s, i) => {
            let index = i.checked_sub(1).ok_or("index underflow")?;
            let dll_name = HSTRING::from(s);
            let hicon = unsafe { ExtractIconW(None, &dll_name, index) };
            if hicon.0.is_null() {
                let last_error = windows::core::Error::from_win32();
                Err(Box::new(io::Error::other(format!(
                    "failed to extract icon from index - {last_error}"
                ))))
            } else {
                Ok(hicon)
            }
        }
        DllResource::Other(path, name, size) => {
            let wide_path: Vec<u16> = OsStr::new(&path).encode_wide().chain(Some(0)).collect();
            let dll_handle = HSTRING::from_wide(&wide_path);
            let (w, h) = (size, size);

            let hicon_handle = if let Ok(id) = name.trim().parse::<i32>() {
                let i = MAKEINTRESOURCEW(id.to_owned());
                unsafe { get_hicon_handle(&dll_handle, i, w, h) }?
            } else {
                let name = PCWSTR::from_raw(HSTRING::from(&name).as_ptr());
                unsafe { get_hicon_handle(&dll_handle, name, w, h) }?
            };

            if hicon_handle.0.is_null() {
                let last_error = windows::core::Error::from_win32();
                Err(Box::new(io::Error::other(format!(
                    "failed to get hIcon from resource: {name} - {last_error}."
                ))))
            } else {
                Ok(HICON(hicon_handle.0))
            }
        }
    }
}
