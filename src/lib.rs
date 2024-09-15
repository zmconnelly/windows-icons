use base64::engine::general_purpose;
use base64::Engine as _;
use image::RgbaImage;
use utils::image_utils::{get_hicon, icon_to_image};
use utils::process_utils::get_process_path;
use uwp_apps::{get_uwp_icon, get_uwp_icon_base64};

mod utils {
    pub mod image_utils;
    pub mod process_utils;
}
mod uwp_apps;

pub fn get_icon_by_process_id(process_id: u32) -> RgbaImage {
    let path = get_process_path(process_id).expect("Failed to get process path");
    println!("Path: {}", path);
    if path.contains("WindowsApps") {
        return get_uwp_icon(&path).expect("Failed to get UWP icon");
    } else {
        return get_icon_by_path(&path);
    }
}

pub fn get_icon_by_path(path: &str) -> RgbaImage {
    unsafe {
        let icon = get_hicon(path);
        icon_to_image(icon)
    }
}

pub fn get_icon_base64_by_process_id(process_id: u32) -> String {
    let path = get_process_path(process_id).expect("Failed to get process path");
    get_icon_base64_by_path(&path)
}

pub fn get_icon_base64_by_path(path: &str) -> String {
    if path.contains("WindowsApps") {
        return get_uwp_icon_base64(path).expect("Failed to get UWP icon base64");
    }
    let icon_image = get_icon_by_path(path);
    let mut buffer = Vec::new();
    icon_image
        .write_to(
            &mut std::io::Cursor::new(&mut buffer),
            image::ImageFormat::Png,
        )
        .unwrap();
    general_purpose::STANDARD.encode(buffer)
}
