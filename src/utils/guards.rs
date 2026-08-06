//! Win32 handles that are released on drop, so the calls that own them can return
//! early without leaking.

use windows::Win32::{
    Foundation::{CloseHandle, FreeLibrary, HANDLE, HMODULE},
    Graphics::Gdi::{DeleteObject, HBITMAP, HDC, HGDIOBJ, ReleaseDC},
    UI::WindowsAndMessaging::{DestroyIcon, HICON},
};

pub struct AutoDc(pub HDC);

impl Drop for AutoDc {
    fn drop(&mut self) {
        if !self.0.0.is_null() {
            let _ = unsafe { ReleaseDC(None, self.0) };
        }
    }
}

pub struct AutoBitmap(pub HBITMAP);

impl Drop for AutoBitmap {
    fn drop(&mut self) {
        if !self.0.0.is_null() {
            let _ = unsafe { DeleteObject(HGDIOBJ::from(self.0)) };
        }
    }
}

pub struct AutoIcon(pub HICON);

impl Drop for AutoIcon {
    fn drop(&mut self) {
        if !self.0.0.is_null() {
            let _ = unsafe { DestroyIcon(self.0) };
        }
    }
}

pub struct AutoModule(pub HMODULE);

impl Drop for AutoModule {
    fn drop(&mut self) {
        if !self.0.0.is_null() {
            let _ = unsafe { FreeLibrary(self.0) };
        }
    }
}

pub struct AutoHandle(pub HANDLE);

impl Drop for AutoHandle {
    fn drop(&mut self) {
        if !self.0.0.is_null() {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}
