use std::{
    error::Error,
    ffi::OsStr,
    fs::File,
    io::Read,
    mem::{self, MaybeUninit},
    os::windows::ffi::OsStrExt,
    ptr,
};

use base64::{engine::general_purpose, Engine};
use image::RgbaImage;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::HWND,
        Graphics::Gdi::{
            DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
            BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, HGDIOBJ,
        },
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        UI::{
            Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON},
            WindowsAndMessaging::{GetIconInfo, HICON},
        },
    },
};

pub unsafe fn icon_to_image(icon: HICON) -> RgbaImage {
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

pub unsafe fn get_hicon(file_path: &str) -> HICON {
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

pub fn read_image_to_base64(file_path: &str) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(general_purpose::STANDARD.encode(&buffer))
}

pub fn get_icon_from_base64(base64: &str) -> Result<RgbaImage, Box<dyn Error>> {
    let buffer = general_purpose::STANDARD.decode(base64)?;
    let image = image::load_from_memory(&buffer)?;
    Ok(image.to_rgba8())
}
