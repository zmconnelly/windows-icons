use std::{
    error::Error,
    ffi::OsStr,
    fs::File,
    io::{self, Read},
    mem::MaybeUninit,
    os::windows::ffi::OsStrExt,
    path::Path,
    sync::Mutex,
};

use base64::{Engine, engine::general_purpose};
use image::RgbaImage;
use windows::{
    Win32::{
        Foundation::SIZE,
        Graphics::Gdi::{
            BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, GetDC, GetDIBits,
            GetObjectW, HBITMAP, HGDIOBJ,
        },
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        System::Com::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize},
        UI::{
            Controls::{ILD_TRANSPARENT, IImageList},
            Shell::{
                IShellItem, IShellItemImageFactory, SHCreateItemFromParsingName, SHFILEINFOW,
                SHGFI_ICON, SHGFI_SYSICONINDEX, SHGetFileInfoW, SHGetImageList, SHIL_EXTRALARGE,
                SHIL_JUMBO, SHIL_SMALL, SIIGBF_ICONONLY, SIIGBF_RESIZETOFIT,
            },
            WindowsAndMessaging::{GetIconInfo, HICON},
        },
    },
    core::{Interface, PCWSTR},
};

use crate::icon_size::IconSize;
use crate::utils::guards::{AutoBitmap, AutoDc, AutoIcon};

/// Shell icon APIs are not reliably thread-safe; serialize sized extraction.
static SHELL_LOCK: Mutex<()> = Mutex::new(());

/// Initializes COM for the current thread and uninitializes on drop when this
/// call was responsible for the successful `CoInitializeEx`.
struct ComApartment(bool);

impl ComApartment {
    fn enter() -> Self {
        let hr =
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        // S_OK / S_FALSE both succeed and must be balanced with CoUninitialize.
        Self(hr.is_ok())
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

pub fn get_hicon_to_image(file_path: &Path) -> Result<RgbaImage, Box<dyn Error>> {
    let hicon = unsafe { get_hicon(file_path) }?;
    unsafe { hicon_to_image(hicon) }
}

pub fn get_hicon_to_image_with_size(
    file_path: &Path,
    size: IconSize,
) -> Result<RgbaImage, Box<dyn Error>> {
    let _shell = SHELL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = ComApartment::enter();

    match size {
        IconSize::Large => unsafe { shell_item_icon_to_image(file_path, size.pixels()) },
        IconSize::Small => {
            let hicon = unsafe { get_hicon_from_image_list(file_path, SHIL_SMALL) }?;
            unsafe { hicon_to_image(hicon) }
        }
        IconSize::Medium => {
            let hicon = unsafe { get_hicon_from_image_list(file_path, SHIL_EXTRALARGE) }?;
            unsafe { hicon_to_image(hicon) }
        }
        IconSize::ExtraLarge => {
            let hicon = unsafe { get_hicon_from_image_list(file_path, SHIL_JUMBO) }?;
            unsafe { hicon_to_image(hicon) }
        }
    }
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

unsafe fn get_hicon_from_image_list(
    file_path: &Path,
    shil: u32,
) -> Result<HICON, Box<dyn Error>> {
    let wide_path: Vec<u16> = OsStr::new(file_path).encode_wide().chain(Some(0)).collect();
    let mut shfileinfo = MaybeUninit::<SHFILEINFOW>::uninit();

    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR::from_raw(wide_path.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(shfileinfo.as_mut_ptr()),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX,
        )
    };

    if result == 0 {
        let last_error = windows::core::Error::from_thread();
        return Err(Box::new(io::Error::other(format!(
            "failed to get system icon index for {file_path:?}: {last_error}"
        ))));
    }

    let shfileinfo = unsafe { shfileinfo.assume_init() };

    let image_list: IImageList = unsafe { SHGetImageList(shil as i32) }.map_err(|e| {
        io::Error::other(format!("SHGetImageList failed for {file_path:?}: {e}"))
    })?;

    unsafe { image_list.GetIcon(shfileinfo.iIcon, ILD_TRANSPARENT.0) }.map_err(|e| {
        io::Error::other(format!("IImageList::GetIcon failed for {file_path:?}: {e}")).into()
    })
}

unsafe fn shell_item_icon_to_image(
    file_path: &Path,
    pixels: u32,
) -> Result<RgbaImage, Box<dyn Error>> {
    let wide_path: Vec<u16> = OsStr::new(file_path).encode_wide().chain(Some(0)).collect();
    let item: IShellItem =
        unsafe { SHCreateItemFromParsingName(PCWSTR::from_raw(wide_path.as_ptr()), None) }
            .map_err(|e| {
                io::Error::other(format!(
                    "SHCreateItemFromParsingName failed for {file_path:?}: {e}"
                ))
            })?;

    let factory: IShellItemImageFactory = item.cast().map_err(|e| {
        io::Error::other(format!(
            "IShellItemImageFactory cast failed for {file_path:?}: {e}"
        ))
    })?;

    let side = i32::try_from(pixels)?;
    let bitmap = unsafe {
        factory.GetImage(
            SIZE { cx: side, cy: side },
            SIIGBF_ICONONLY | SIIGBF_RESIZETOFIT,
        )
    }
    .map_err(|e| {
        io::Error::other(format!(
            "IShellItemImageFactory::GetImage failed for {file_path:?}: {e}"
        ))
    })?;

    unsafe { hbitmap_to_image(bitmap) }
}

unsafe fn hbitmap_to_image(bitmap: HBITMAP) -> Result<RgbaImage, Box<dyn Error>> {
    let _bitmap_guard = AutoBitmap(bitmap);
    unsafe { gdi_bitmap_to_image(bitmap) }
}

/// Reads pixels from an `HBITMAP` without taking ownership of the handle.
unsafe fn gdi_bitmap_to_image(bitmap: HBITMAP) -> Result<RgbaImage, Box<dyn Error>> {
    let bitmap_size_i32 = i32::try_from(size_of::<BITMAP>())?;
    let biheader_size_u32 = u32::try_from(size_of::<BITMAPINFOHEADER>())?;

    let mut gdi_bitmap: MaybeUninit<BITMAP> = MaybeUninit::uninit();
    let result = unsafe {
        GetObjectW(
            HGDIOBJ::from(bitmap),
            bitmap_size_i32,
            Some(gdi_bitmap.as_mut_ptr().cast()),
        )
    };
    if result != bitmap_size_i32 {
        return Err(Box::new(io::Error::other(format!(
            "GetObjectW failed, expected {bitmap_size_i32}, got {result}"
        ))));
    }
    let gdi_bitmap = unsafe { gdi_bitmap.assume_init() };

    let width_u32 = gdi_bitmap.bmWidth.unsigned_abs();
    let height_u32 = gdi_bitmap.bmHeight.unsigned_abs();
    let width_i32 = i32::try_from(width_u32)?;
    let height_i32 = i32::try_from(height_u32)?;
    let width_usize = usize::try_from(width_u32)?;
    let height_usize = usize::try_from(height_u32)?;

    let pixel_count = width_usize
        .checked_mul(height_usize)
        .ok_or_else(|| io::Error::other("buffer size overflow"))?;

    let mut buf = vec![0u32; pixel_count];

    let dc = unsafe { GetDC(None) };
    if dc.0.is_null() {
        return Err(Box::new(io::Error::other("GetDC returned null")));
    }
    let _dc_guard = AutoDc(dc);

    // Absolute width/height with a negative biHeight request a top-down buffer,
    // even when GetObjectW reports a top-down DIB (bmHeight < 0).
    let mut bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: biheader_size_u32,
            biWidth: width_i32,
            biHeight: -height_i32,
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
            bitmap,
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
    } else if result != height_i32 {
        return Err(Box::new(io::Error::other(format!(
            "GetDIBits failed, expected lines: `{height_i32}`, got: `{result}`"
        ))));
    }

    bgra_buf_to_rgba_image(width_u32, height_u32, &buf)
}

fn bgra_buf_to_rgba_image(
    width: u32,
    height: u32,
    buf: &[u32],
) -> Result<RgbaImage, Box<dyn Error>> {
    let pixel_data = unsafe {
        std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), std::mem::size_of_val(buf))
    };

    let rgba_data = pixel_data
        .chunks_exact(4)
        .flat_map(|px| [px[2], px[1], px[0], px[3]])
        .collect::<Vec<_>>();

    RgbaImage::from_raw(width, height, rgba_data)
        .ok_or_else(|| "pixel data does not match the bitmap dimensions".into())
}

pub unsafe fn hicon_to_image(icon: HICON) -> Result<RgbaImage, Box<dyn Error>> {
    let mut info = MaybeUninit::uninit();
    unsafe {
        GetIconInfo(icon, info.as_mut_ptr())
            .map_err(|e| io::Error::other(format!("GetIconInfo failed: {e}")))
    }?;
    let info = unsafe { info.assume_init() };

    let _mask_guard = AutoBitmap(info.hbmMask);
    let _color_guard = AutoBitmap(info.hbmColor);
    let _icon_guard = AutoIcon(icon);

    unsafe { gdi_bitmap_to_image(info.hbmColor) }
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

/// Fits `image` into a `pixels`×`pixels` canvas, preserving aspect ratio and
/// centering on a transparent background when the source is not square.
pub fn resize_rgba_image(image: RgbaImage, pixels: u32) -> RgbaImage {
    if image.width() == pixels && image.height() == pixels {
        return image;
    }

    let (width, height) = (image.width(), image.height());
    if width == 0 || height == 0 || pixels == 0 {
        return RgbaImage::new(pixels, pixels);
    }

    let scale = (pixels as f32 / width as f32).min(pixels as f32 / height as f32);
    let new_w = ((width as f32 * scale).round() as u32).max(1).min(pixels);
    let new_h = ((height as f32 * scale).round() as u32).max(1).min(pixels);

    let resized = image::imageops::resize(
        &image,
        new_w,
        new_h,
        image::imageops::FilterType::Triangle,
    );

    if new_w == pixels && new_h == pixels {
        return resized;
    }

    let mut canvas = RgbaImage::new(pixels, pixels);
    let x = i64::from((pixels - new_w) / 2);
    let y = i64::from((pixels - new_h) / 2);
    image::imageops::overlay(&mut canvas, &resized, x, y);
    canvas
}

#[cfg(test)]
mod tests {
    use super::resize_rgba_image;
    use image::{Rgba, RgbaImage};

    #[test]
    fn resize_preserves_aspect_with_letterboxing() {
        let mut src = RgbaImage::new(100, 50);
        for pixel in src.pixels_mut() {
            *pixel = Rgba([255, 0, 0, 255]);
        }

        let out = resize_rgba_image(src, 40);
        assert_eq!((out.width(), out.height()), (40, 40));
        // Fitted size is 40×20, centered vertically → transparent rows at top/bottom.
        assert_eq!(out.get_pixel(20, 0).0[3], 0);
        assert_eq!(out.get_pixel(20, 20).0, [255, 0, 0, 255]);
    }
}
