use base64::engine::general_purpose;
use base64::Engine as _;
use image::RgbaImage;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::mem::{self, MaybeUninit};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;
use std::ptr;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Graphics::Gdi::GetObjectW;
use windows::Win32::System::ProcessStatus::K32GetModuleFileNameExW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON};
use windows::Win32::{
    Foundation::HWND,
    Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        DIB_RGB_COLORS, HDC, HGDIOBJ,
    },
    Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
    UI::WindowsAndMessaging::{GetIconInfo, HICON},
};

unsafe fn get_hicon(file_path: &str) -> HICON {
    let wide_path: Vec<u16> = OsStr::new(file_path).encode_wide().chain(Some(0)).collect();
    let mut shfileinfo: SHFILEINFOW = std::mem::zeroed();

    let result = SHGetFileInfoW(
        PCWSTR::from_raw(wide_path.as_ptr()),
        FILE_FLAGS_AND_ATTRIBUTES(0),
        Some(&mut shfileinfo as *mut SHFILEINFOW),
        std::mem::size_of::<SHFILEINFOW>() as u32,
        SHGFI_ICON,
    );

    if result == 0 {
        panic!("Failed to get icon for file: {}", file_path);
    }

    shfileinfo.hIcon
}

unsafe fn icon_to_image(icon: HICON) -> RgbaImage {
    let bitmap_size_i32 = i32::try_from(mem::size_of::<BITMAP>()).unwrap();
    let biheader_size_u32 = u32::try_from(mem::size_of::<BITMAPINFOHEADER>()).unwrap();

    let mut info = MaybeUninit::uninit();
    GetIconInfo(icon, info.as_mut_ptr()).unwrap();
    let info = info.assume_init();
    DeleteObject(info.hbmMask).unwrap();

    let mut bitmap: MaybeUninit<BITMAP> = MaybeUninit::uninit();
    let result = GetObjectW(
        HGDIOBJ(info.hbmColor.0),
        bitmap_size_i32,
        Some(bitmap.as_mut_ptr().cast()),
    );
    assert!(result == bitmap_size_i32);
    let bitmap = bitmap.assume_init();

    let width_u32 = u32::try_from(bitmap.bmWidth).unwrap();
    let height_u32 = u32::try_from(bitmap.bmHeight).unwrap();
    let width_usize = usize::try_from(bitmap.bmWidth).unwrap();
    let height_usize = usize::try_from(bitmap.bmHeight).unwrap();
    let buf_size = width_usize.checked_mul(height_usize).unwrap();
    let mut buf: Vec<u32> = Vec::with_capacity(buf_size);

    let dc = GetDC(HWND(ptr::null_mut()));
    assert!(dc != HDC(ptr::null_mut()));

    let mut bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: biheader_size_u32,
            biWidth: bitmap.bmWidth,
            biHeight: -bitmap.bmHeight,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [Default::default()],
    };
    let result = GetDIBits(
        dc,
        info.hbmColor,
        0,
        height_u32,
        Some(buf.as_mut_ptr().cast()),
        &mut bitmap_info,
        DIB_RGB_COLORS,
    );
    assert!(result == bitmap.bmHeight);
    buf.set_len(buf.capacity());

    let result = ReleaseDC(HWND(ptr::null_mut()), dc);
    assert!(result == 1);
    DeleteObject(info.hbmColor).unwrap();

    RgbaImage::from_fn(width_u32, height_u32, |x, y| {
        let x_usize = usize::try_from(x).unwrap();
        let y_usize = usize::try_from(y).unwrap();
        let idx = y_usize * width_usize + x_usize;
        let [b, g, r, a] = buf[idx].to_le_bytes();
        [r, g, b, a].into()
    })
}

pub fn get_process_path(process_id: u32) -> Result<String, windows::core::Error> {
    unsafe {
        let process_handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            process_id,
        )?;
        let mut buffer = vec![0u16; 1024];
        let size = K32GetModuleFileNameExW(HANDLE(process_handle.0), None, &mut buffer);
        CloseHandle(process_handle).unwrap();

        if size == 0 {
            return Err(windows::core::Error::from_win32());
        }

        buffer.truncate(size as usize);
        let path = OsString::from_wide(&buffer).into_string().map_err(|_| {
            windows::core::Error::new(
                windows::core::HRESULT(-1),
                "Invalid Unicode in path".to_string(),
            )
        })?;

        Ok(path)
    }
}

pub fn get_icon_image_by_process_id(process_id: u32) -> RgbaImage {
    let path = get_process_path(process_id).unwrap();
    get_icon_image_by_path(&path)
}

pub fn get_icon_image_by_path(path: &str) -> RgbaImage {
    unsafe {
        let icon = get_hicon(path);
        icon_to_image(icon)
    }
}

pub fn get_icon_base64_by_process_id(process_id: u32) -> String {
    let path = get_process_path(process_id).unwrap();
    get_icon_base64_by_path(&path)
}

pub fn get_icon_base64_by_path(path: &str) -> String {
    let icon_image = get_icon_image_by_path(path);
    let mut buffer = Vec::new();
    icon_image
        .write_to(
            &mut std::io::Cursor::new(&mut buffer),
            image::ImageFormat::Png,
        )
        .unwrap();
    general_purpose::STANDARD.encode(buffer)
}
