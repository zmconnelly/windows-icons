use std::{error::Error, fs, path::Path};

use image::RgbaImage;

use crate::utils::image_utils::{get_icon_from_base64, read_image_to_base64};

pub fn get_uwp_icon(process_path: &str) -> Result<RgbaImage, Box<dyn Error>> {
    let icon_path = get_icon_file_path(process_path)?;
    println!("Icon path: {}", icon_path);
    let base64 = read_image_to_base64(&icon_path)?;
    let icon = get_icon_from_base64(&base64)?;
    Ok(icon)
}

pub fn get_uwp_icon_base64(process_path: &str) -> Result<String, Box<dyn Error>> {
    let icon_path = get_icon_file_path(process_path)?;
    let base64 = read_image_to_base64(&icon_path)?;
    Ok(base64)
}

fn get_icon_file_path(app_path: &str) -> Result<String, Box<dyn Error>> {
    let package_folder = Path::new(app_path).parent().unwrap();

    let desktop_icon_path = package_folder.join("assets").join("DesktopShortcut.ico");

    if desktop_icon_path.exists() {
        return Ok(desktop_icon_path.to_str().unwrap().to_string());
    } else {
        let manifest_path = package_folder.join("AppxManifest.xml");
        let manifest_content = fs::read_to_string(&manifest_path)?;

        let icon_path = extract_icon_path(&manifest_content)?;
        let icon_full_path = package_folder.join(icon_path);

        return Ok(icon_full_path.to_str().unwrap().to_string());
    }
}

fn extract_icon_path(manifest_content: &str) -> Result<String, Box<dyn Error>> {
    // Look for the <Logo>...</Logo> tag in the manifest
    let start_tag = "<Logo>";
    let end_tag = "</Logo>";

    if let Some(start) = manifest_content.find(start_tag) {
        if let Some(end) = manifest_content.find(end_tag) {
            let start_pos = start + start_tag.len();
            let icon_path = &manifest_content[start_pos..end];
            return Ok(icon_path.trim().to_string());
        }
    }

    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Icon path not found in manifest.",
    )))
}
