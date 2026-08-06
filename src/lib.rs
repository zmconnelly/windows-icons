mod app_exec_link;
mod dll_icons;
mod icon_size;
mod utils;
mod uwp_apps;

use std::{error::Error, path::Path};

use image::RgbaImage;

pub use dll_icons::DllIcon;
pub use icon_size::IconSize;

use app_exec_link::resolve_app_exec_link;
use dll_icons::get_dll_hicon_to_image;
use utils::image_utils::{get_hicon_to_image, get_hicon_to_image_with_size, image_to_base64};
use utils::process_utils::get_process_path;
use uwp_apps::{get_uwp_icon, get_uwp_icon_base64, get_uwp_icon_with_size, is_uwp_app};

pub fn get_icon_by_path<P: AsRef<Path>>(path: P) -> Result<RgbaImage, Box<dyn Error>> {
    let resolved = resolve_app_exec_link(path.as_ref());
    let path = &*resolved;

    if is_uwp_app(path) {
        get_uwp_icon(path)
    } else {
        get_hicon_to_image(path)
    }
}

/// Extract a file icon at an Explorer-aligned [`IconSize`].
///
/// See [`IconSize`] for DPI and jumbo-padding notes. For UWP packages this
/// decodes the logo and fits it into a square of `size.pixels()`, preserving
/// aspect ratio on a transparent canvas.
pub fn get_icon_by_path_with_size<P: AsRef<Path>>(
    path: P,
    size: IconSize,
) -> Result<RgbaImage, Box<dyn Error>> {
    let resolved = resolve_app_exec_link(path.as_ref());
    let path = &*resolved;

    if is_uwp_app(path) {
        get_uwp_icon_with_size(path, size)
    } else {
        get_hicon_to_image_with_size(path, size)
    }
}

pub fn get_icon_base64_by_path<P: AsRef<Path>>(path: P) -> Result<String, Box<dyn Error>> {
    let resolved = resolve_app_exec_link(path.as_ref());
    let path = &*resolved;

    if is_uwp_app(path) {
        get_uwp_icon_base64(path)
    } else {
        let icon_image = get_hicon_to_image(path)?;
        image_to_base64(icon_image)
    }
}

/// Like [`get_icon_by_path_with_size`], then PNG-encodes the result as base64.
///
/// Unlike [`get_icon_base64_by_path`] for UWP (which returns the raw logo file
/// bytes), this always encodes the resized raster as PNG.
pub fn get_icon_base64_by_path_with_size<P: AsRef<Path>>(
    path: P,
    size: IconSize,
) -> Result<String, Box<dyn Error>> {
    let icon_image = get_icon_by_path_with_size(path, size)?;
    image_to_base64(icon_image)
}

pub fn get_icon_by_process_id(process_id: u32) -> Result<RgbaImage, Box<dyn Error>> {
    let process_path = get_process_path(process_id)?;
    get_icon_by_path(&process_path)
}

pub fn get_icon_by_process_id_with_size(
    process_id: u32,
    size: IconSize,
) -> Result<RgbaImage, Box<dyn Error>> {
    let process_path = get_process_path(process_id)?;
    get_icon_by_path_with_size(&process_path, size)
}

pub fn get_icon_base64_by_process_id(process_id: u32) -> Result<String, Box<dyn Error>> {
    let process_path = get_process_path(process_id)?;
    get_icon_base64_by_path(&process_path)
}

pub fn get_icon_base64_by_process_id_with_size(
    process_id: u32,
    size: IconSize,
) -> Result<String, Box<dyn Error>> {
    let process_path = get_process_path(process_id)?;
    get_icon_base64_by_path_with_size(&process_path, size)
}

pub fn get_icon_by_dll(dll_icon: DllIcon) -> Result<RgbaImage, Box<dyn Error>> {
    get_dll_hicon_to_image(dll_icon)
}

pub fn get_icon_base64_by_dll(dll_icon: DllIcon) -> Result<String, Box<dyn Error>> {
    let dll_image = get_icon_by_dll(dll_icon)?;
    image_to_base64(dll_image)
}
