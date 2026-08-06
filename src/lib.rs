mod app_exec_link;
mod dll_icons;
mod utils;
mod uwp_apps;

use std::{error::Error, path::Path};

use image::RgbaImage;

pub use dll_icons::DllIcon;

use app_exec_link::resolve_app_exec_link;
use dll_icons::get_dll_hicon_to_image;
use utils::image_utils::{get_hicon_to_image, image_to_base64};
use utils::process_utils::get_process_path;
use uwp_apps::{get_uwp_icon, get_uwp_icon_base64, is_uwp_app};

pub fn get_icon_by_path<P: AsRef<Path>>(path: P) -> Result<RgbaImage, Box<dyn Error>> {
    let resolved = resolve_app_exec_link(path.as_ref());
    let path = &*resolved;

    if is_uwp_app(path) {
        get_uwp_icon(path)
    } else {
        get_hicon_to_image(path)
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

pub fn get_icon_by_process_id(process_id: u32) -> Result<RgbaImage, Box<dyn Error>> {
    let process_path = get_process_path(process_id)?;
    get_icon_by_path(&process_path)
}

pub fn get_icon_base64_by_process_id(process_id: u32) -> Result<String, Box<dyn Error>> {
    let process_path = get_process_path(process_id)?;
    get_icon_base64_by_path(&process_path)
}

pub fn get_icon_by_dll(dll_icon: DllIcon) -> Result<RgbaImage, Box<dyn Error>> {
    get_dll_hicon_to_image(dll_icon)
}

pub fn get_icon_base64_by_dll(dll_icon: DllIcon) -> Result<String, Box<dyn Error>> {
    let dll_image = get_icon_by_dll(dll_icon)?;
    image_to_base64(dll_image)
}
