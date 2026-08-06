use std::{
    error::Error,
    ffi::OsStr,
    fs::File,
    io::{self, Read},
    mem::MaybeUninit,
    os::windows::ffi::OsStrExt,
    path::Path,
};

use base64::{Engine, engine::general_purpose};
use image::RgbaImage;
use windows::{
    Win32::{
        Graphics::Gdi::{
            BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, GetDC, GetDIBits,
            GetObjectW, HGDIOBJ,
        },
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        UI::{
            Shell::{SHFILEINFOW, SHGFI_ICON, SHGetFileInfoW},
            WindowsAndMessaging::{GetIconInfo, HICON},
        },
    },
    core::PCWSTR,
};

use crate::utils::guards::{AutoBitmap, AutoDc, AutoIcon};

pub fn get_hicon_to_image(file_path: &Path) -> Result<RgbaImage, Box<dyn Error>> {
    let hicon = unsafe { get_hicon(file_path) }?;
    unsafe { hicon_to_image(hicon) }
}

unsafe fn get_hicon(file_path: &Path) -> Result<HICON, Box<dyn Error>> {
    let wide_path: Vec<u16> = OsStr::new(file_path).encode_wide().chain(Some(0)).collect();
    let mut shfileinfo = MaybeUninit::<SHFILEINFOW>::uninit();

    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR::from_raw(wide_path.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(shfileinfo.as_mut_ptr()),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON,
        )
    };

    if result == 0 {
        let last_error = windows::core::Error::from_thread();
        return Err(Box::new(io::Error::other(format!(
            "failed to get hIcon for {file_path:?}: {last_error}"
        ))));
    }

    let shfileinfo = unsafe { shfileinfo.assume_init() };

    Ok(shfileinfo.hIcon)
}

pub unsafe fn hicon_to_image(icon: HICON) -> Result<RgbaImage, Box<dyn Error>> {
    let bitmap_size_i32 = i32::try_from(size_of::<BITMAP>())?;
    let biheader_size_u32 = u32::try_from(size_of::<BITMAPINFOHEADER>())?;

    let mut info = MaybeUninit::uninit();
    unsafe {
        GetIconInfo(icon, info.as_mut_ptr())
            .map_err(|e| io::Error::other(format!("GetIconInfo failed: {e}")))
    }?;
    let info = unsafe { info.assume_init() };

    let _mask_guard = AutoBitmap(info.hbmMask);
    let _color_guard = AutoBitmap(info.hbmColor);
    let _icon_guard = AutoIcon(icon);

    let mut bitmap: MaybeUninit<BITMAP> = MaybeUninit::uninit();
    let result = unsafe {
        GetObjectW(
            HGDIOBJ::from(info.hbmColor),
            bitmap_size_i32,
            Some(bitmap.as_mut_ptr().cast()),
        )
    };
    if result != bitmap_size_i32 {
        return Err(Box::new(io::Error::other(format!(
            "GetObjectW failed, expected {bitmap_size_i32}, got {result}"
        ))));
    }
    let bitmap = unsafe { bitmap.assume_init() };

    let width_u32 = bitmap.bmWidth.unsigned_abs();
    let height_u32 = bitmap.bmHeight.unsigned_abs();
    let width_usize = usize::try_from(width_u32)?;
    let height_usize = usize::try_from(height_u32)?;
    let expected_lines = i32::try_from(height_u32)?;

    let pixel_count = width_usize
        .checked_mul(height_usize)
        .ok_or_else(|| io::Error::other("buffer size overflow"))?;

    let mut buf = vec![0u32; pixel_count];

    let dc = unsafe { GetDC(None) };
    if dc.0.is_null() {
        return Err(Box::new(io::Error::other("GetDC returned null")));
    }
    let _dc_guard = AutoDc(dc);

    let mut bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: biheader_size_u32,
            biWidth: bitmap.bmWidth,
            // Negative height asks for a top-down image, matching RgbaImage's row order.
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
    let result = unsafe {
        GetDIBits(
            dc,
            info.hbmColor,
            0,
            height_u32,
            Some(buf.as_mut_ptr().cast()),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        )
    };
    if result == 0 {
        let last_error = windows::core::Error::from_thread();
        return Err(Box::new(io::Error::other(format!(
            "GetDIBits failed: {last_error}"
        ))));
    } else if result != expected_lines {
        return Err(Box::new(io::Error::other(format!(
            "GetDIBits failed, expected lines: `{expected_lines}`, got: `{result}`"
        ))));
    }

    let pixel_data = unsafe {
        std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), buf.len() * size_of::<u32>())
    };

    // BGRA -> RGBA
    let rgba_data = pixel_data
        .chunks_exact(4)
        .flat_map(|px| [px[2], px[1], px[0], px[3]])
        .collect::<Vec<_>>();

    RgbaImage::from_raw(width_u32, height_u32, rgba_data)
        .ok_or_else(|| "pixel data does not match the icon dimensions".into())
}

fn read_icon_file(icon_path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut file = File::open(icon_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub fn icon_file_to_image(icon_path: &Path) -> Result<RgbaImage, Box<dyn Error>> {
    let buffer = read_icon_file(icon_path)?;
    let image = image::load_from_memory(&buffer)
        .map_err(|e| io::Error::other(format!("image decode failed: {e}")))?;
    Ok(image.to_rgba8())
}

pub fn icon_file_to_base64(icon_path: &Path) -> Result<String, Box<dyn Error>> {
    let buffer = read_icon_file(icon_path)?;
    Ok(general_purpose::STANDARD.encode(&buffer))
}

pub fn image_to_base64(img: RgbaImage) -> Result<String, Box<dyn Error>> {
    let mut buffer = Vec::with_capacity(1024 * 50);
    img.write_to(
        &mut std::io::Cursor::new(&mut buffer),
        image::ImageFormat::Png,
    )?;
    Ok(general_purpose::STANDARD.encode(buffer))
}
