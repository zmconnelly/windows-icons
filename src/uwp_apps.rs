use std::{
    error::Error,
    ffi::OsStr,
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use glob::glob;
use image::RgbaImage;

use crate::utils::image_utils::{icon_file_to_base64, icon_file_to_image};

pub fn get_uwp_icon(file_path: &Path) -> Result<RgbaImage, Box<dyn Error>> {
    let icon_path = get_icon_file_path(file_path)?;
    let rgba_image = icon_file_to_image(&icon_path).map_err(|e| {
        io::Error::other(format!(
            "Failed to get icon image for path: '{file_path:?}'\n{e}"
        ))
    })?;
    Ok(rgba_image)
}

pub fn get_uwp_icon_base64(file_path: &Path) -> Result<String, Box<dyn Error>> {
    let icon_path = get_icon_file_path(file_path)?;
    let base64 = icon_file_to_base64(&icon_path).map_err(|e| {
        io::Error::other(format!(
            "Failed to get icon base64 for path: '{file_path:?}'\n{e}"
        ))
    })?;
    Ok(base64)
}

fn get_icon_file_path(app_path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if !app_path.exists() {
        return Err(Box::new(io::Error::new(
            ErrorKind::NotFound,
            format!("app path does not exist: '{app_path:?}'"),
        )));
    }

    let package_folder = app_path.parent().ok_or_else(|| {
        io::Error::new(
            ErrorKind::NotFound,
            format!("failed to get parent directory: '{app_path:?}'"),
        )
    })?;
    let manifest_path = package_folder.join("AppxManifest.xml");
    if manifest_path.exists() {
        let manifest_content = fs::read_to_string(&manifest_path)
            .map_err(|_| io::Error::other("could not to read the AppxManifest.xml."))?;

        let icon_path = extract_icon_path(&manifest_content)?;
        let icon_full_path = package_folder.join(icon_path);
        if icon_full_path.exists() {
            Ok(icon_full_path)
        } else {
            find_matching_logo_file(&icon_full_path, package_folder)
        }
    } else {
        fuzzy_get_icon_file_path(package_folder).map_err(|e| {
            Box::new(io::Error::other(format!(
                "AppxManifest.xml does not exist and {e}"
            ))) as Box<dyn Error>
        })
    }
}

fn extract_icon_path(manifest_content: &str) -> Result<String, Box<dyn Error>> {
    // Look for the <Logo>...</Logo> tag in the manifest
    let start_tag = "<Logo>";
    let end_tag = "</Logo>";

    if let Some(start) = manifest_content.find(start_tag)
        && let Some(end) = manifest_content.find(end_tag)
    {
        let start_pos = start + start_tag.len();
        let icon_path = &manifest_content[start_pos..end];
        return Ok(icon_path.trim().to_string());
    }

    Err(Box::new(io::Error::new(
        ErrorKind::NotFound,
        "icon path not found in manifest.",
    )))
}

fn find_matching_logo_file(
    icon_full_path: &Path,
    package_folder: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    let parent_path = icon_full_path
        .parent()
        .and_then(Path::to_str)
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "no directory found"))?;

    let filter_name = icon_full_path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "no file name found"))?;

    let extension = icon_full_path
        .extension()
        .and_then(OsStr::to_str)
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "no extension found"))?;

    // '*' might be ".scale-size", and may include theme characters "contrast-white" and "contrast-black"
    let pattern = format!("{parent_path}/{filter_name}*.{extension}");
    let exclude_theme = ["contrast-white", "contrast-black"];
    let mut matching_logo_files = Vec::new();

    for logo_path in glob(&pattern)?.filter_map(Result::ok) {
        if logo_path.is_file() {
            let name = logo_path
                .file_stem()
                .and_then(OsStr::to_str)
                .unwrap_or_default()
                .to_lowercase();

            if !exclude_theme.iter().any(|t| name.contains(t)) {
                let size = logo_path.metadata()?.len();
                matching_logo_files.push((logo_path, size));
            }
        }
    }

    let max_size_logo_file_path = matching_logo_files
        .iter()
        .max_by_key(|(_, size)| size)
        .map(|(path, _)| path.to_owned());

    if let Some(path) = max_size_logo_file_path {
        Ok(path)
    } else {
        fuzzy_get_icon_file_path(package_folder)
    }
}

fn fuzzy_get_icon_file_path(package_folder: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if !package_folder.exists() {
        return Err(Box::new(io::Error::new(
            ErrorKind::NotFound,
            format!("Package folder not found: {package_folder:?}"),
        )));
    }

    let matching_names = ["logo", "icon", "DesktopShortcut"];
    let matching_extension = ["png", "ico"];
    let mut matching_logo_files = Vec::new();

    for name in matching_names {
        for ext in matching_extension {
            let pattern = format!("{}/**/{}.{}", package_folder.to_string_lossy(), name, ext);
            for logo_path in glob(&pattern)?.filter_map(Result::ok) {
                if logo_path.is_file() {
                    let metadata = fs::metadata(&logo_path).map_err(|_| {
                        io::Error::new(ErrorKind::NotFound, "failed to get logo information")
                    })?;
                    let logo_size = metadata.len();
                    matching_logo_files.push((logo_path, logo_size));
                }
            }
        }
    }

    let max_size_logo_file_path = matching_logo_files
        .iter()
        .max_by_key(|(_, size)| size)
        .map(|(path, _)| path.to_owned())
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "No icon found in package folder"))?;

    Ok(max_size_logo_file_path)
}
