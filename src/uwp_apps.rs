use std::{
    error::Error,
    ffi::OsStr,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use glob::glob;
use image::RgbaImage;

use crate::icon_size::IconSize;
use crate::utils::image_utils::{icon_file_to_base64, icon_file_to_image, resize_rgba_image};

/// UWP apps are installed to `<Program Files>\WindowsApps\<package folder>\...`, and the
/// `Program Files` segment is localized (`Programme` on German Windows, and so on), so
/// `WindowsApps` is the only segment that can be matched reliably. Requiring a package
/// folder below it excludes the app execution aliases in
/// `%LOCALAPPDATA%\Microsoft\WindowsApps`, which sit directly in that folder.
pub fn is_uwp_app(path: &Path) -> bool {
    let path = path.to_string_lossy();

    if path.to_lowercase().contains("windowssubsystemforandroid") {
        return false;
    }

    let mut segments = path.split(['/', '\\']).filter(|s| !s.is_empty());
    if !segments.any(|s| s.eq_ignore_ascii_case("WindowsApps")) {
        return false;
    }

    segments.nth(1).is_some()
}

pub fn get_uwp_icon(file_path: &Path) -> Result<RgbaImage, Box<dyn Error>> {
    let icon_path = get_icon_file_path(file_path)?;
    let rgba_image = icon_file_to_image(&icon_path).map_err(|e| {
        io::Error::other(format!("failed to get icon image for {file_path:?}: {e}"))
    })?;

    Ok(rgba_image)
}

/// Decode the UWP logo and fit it into a `size.pixels()` square, preserving
/// aspect ratio (letterboxed on a transparent canvas when not square).
pub fn get_uwp_icon_with_size(
    file_path: &Path,
    size: IconSize,
) -> Result<RgbaImage, Box<dyn Error>> {
    let rgba_image = get_uwp_icon(file_path)?;
    Ok(resize_rgba_image(rgba_image, size.pixels()))
}

pub fn get_uwp_icon_base64(file_path: &Path) -> Result<String, Box<dyn Error>> {
    let icon_path = get_icon_file_path(file_path)?;
    let base64 = icon_file_to_base64(&icon_path).map_err(|e| {
        io::Error::other(format!("failed to get icon base64 for {file_path:?}: {e}"))
    })?;

    Ok(base64)
}

fn get_icon_file_path(app_path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if !app_path.exists() {
        return Err(Box::new(io::Error::new(
            ErrorKind::NotFound,
            format!("app path does not exist: {app_path:?}"),
        )));
    }

    let package_folder = package_folder_from_path(app_path).ok_or_else(|| {
        io::Error::new(
            ErrorKind::NotFound,
            format!("failed to get UWP package directory: {app_path:?}"),
        )
    })?;

    let manifest_path = package_folder.join("AppxManifest.xml");
    if !manifest_path.exists() {
        return fuzzy_get_icon_file_path(&package_folder).map_err(|e| {
            Box::new(io::Error::other(format!(
                "AppxManifest.xml does not exist and {e}"
            ))) as Box<dyn Error>
        });
    }

    let manifest_content = std::fs::read_to_string(&manifest_path)
        .map_err(|_| io::Error::other("could not read AppxManifest.xml"))?;

    let icon_full_path = package_folder.join(extract_icon_path(&manifest_content)?);
    if icon_full_path.exists() {
        Ok(icon_full_path)
    } else {
        find_matching_logo_file(&icon_full_path, &package_folder)
    }
}

/// Finds the package root for nested UWP executables.
fn package_folder_from_path(app_path: &Path) -> Option<PathBuf> {
    let mut package_folder = PathBuf::new();
    let mut windows_apps_found = false;

    for component in app_path.components() {
        package_folder.push(component.as_os_str());

        if windows_apps_found {
            return Some(package_folder);
        }

        windows_apps_found = component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("WindowsApps");
    }

    None
}

fn extract_icon_path(manifest_content: &str) -> Result<String, Box<dyn Error>> {
    manifest_content
        .split_once("<Logo>")
        .and_then(|(_, rest)| rest.split_once("</Logo>"))
        .map(|(icon_path, _)| icon_path.trim().to_string())
        .ok_or_else(|| {
            io::Error::new(ErrorKind::NotFound, "icon path not found in manifest").into()
        })
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

    // The manifest names one logo, but the package ships scaled variants of it such as
    // `Logo.scale-200.png`, alongside high contrast versions that would look wrong here.
    let pattern = format!("{parent_path}/{filter_name}*.{extension}");
    let excluded_themes = ["contrast-white", "contrast-black"];
    let logo_file = largest_matching_file(std::slice::from_ref(&pattern), |name| {
        !excluded_themes.iter().any(|theme| name.contains(theme))
    })?;

    match logo_file {
        Some(path) => Ok(path),
        None => fuzzy_get_icon_file_path(package_folder),
    }
}

fn fuzzy_get_icon_file_path(package_folder: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if !package_folder.exists() {
        return Err(Box::new(io::Error::new(
            ErrorKind::NotFound,
            format!("package folder not found: {package_folder:?}"),
        )));
    }

    let folder = package_folder.to_string_lossy();
    let mut patterns = Vec::new();
    for name in ["logo", "icon", "DesktopShortcut"] {
        for extension in ["png", "ico"] {
            patterns.push(format!("{folder}/**/{name}.{extension}"));
        }
    }

    largest_matching_file(&patterns, |_| true)?.ok_or_else(|| {
        io::Error::new(ErrorKind::NotFound, "no icon found in package folder").into()
    })
}

/// The largest file matching any of `patterns`, which is the highest resolution logo
/// when a package ships the same image at several scales.
fn largest_matching_file(
    patterns: &[String],
    accept: impl Fn(&str) -> bool,
) -> Result<Option<PathBuf>, Box<dyn Error>> {
    let mut largest: Option<(PathBuf, u64)> = None;

    for pattern in patterns {
        for path in glob(pattern)?.filter_map(Result::ok) {
            if !path.is_file() {
                continue;
            }

            let name = path
                .file_stem()
                .and_then(OsStr::to_str)
                .unwrap_or_default()
                .to_lowercase();

            if !accept(&name) {
                continue;
            }

            let size = path.metadata()?.len();
            if largest.as_ref().is_none_or(|(_, largest)| size > *largest) {
                largest = Some((path, size));
            }
        }
    }

    Ok(largest.map(|(path, _)| path))
}

#[cfg(test)]
mod tests {
    use super::{extract_icon_path, is_uwp_app, package_folder_from_path};
    use std::path::{Path, PathBuf};

    fn is_uwp(path: &str) -> bool {
        is_uwp_app(Path::new(path))
    }

    #[test]
    fn detects_uwp_apps_regardless_of_program_files_localization() {
        assert!(is_uwp(
            r"C:\Program Files\WindowsApps\Microsoft.WindowsCalculator_11.2210.0.0_x64__8wekyb3d8bbwe\Calculator.exe"
        ));
        assert!(is_uwp(
            r"C:\Programme\WindowsApps\Microsoft.WindowsCalculator_11.2210.0.0_x64__8wekyb3d8bbwe\Calculator.exe"
        ));
        assert!(is_uwp(
            r"D:\Archivos de programa\WindowsApps\Some.Package_1.0.0.0_x64__abcdefg\App.exe"
        ));
    }

    #[test]
    fn matches_windowsapps_case_insensitively_and_with_either_separator() {
        assert!(is_uwp(
            r"C:\Program Files\windowsapps\Some.Package_1.0.0.0_x64__abcdefg\App.exe"
        ));
        assert!(is_uwp(
            "C:/Program Files/WindowsApps/Some.Package_1.0.0.0_x64__abcdefg/App.exe"
        ));
    }

    #[test]
    fn ignores_windows_subsystem_for_android() {
        assert!(!is_uwp(
            r"C:\Program Files\WindowsApps\MicrosoftCorporationII.WindowsSubsystemForAndroid_2211.40000.11.0_x64__8wekyb3d8bbwe\WsaClient.exe"
        ));
    }

    #[test]
    fn ignores_app_execution_aliases() {
        assert!(!is_uwp(
            r"C:\Users\test\AppData\Local\Microsoft\WindowsApps\python.exe"
        ));
    }

    #[test]
    fn ignores_regular_executables() {
        assert!(!is_uwp(r"C:\Windows\System32\notepad.exe"));
        assert!(!is_uwp(r"C:\Program Files\Git\bin\git.exe"));
    }

    #[test]
    fn finds_package_folder_for_nested_executables() {
        let app_path = Path::new(
            r"C:\Program Files\WindowsApps\Microsoft.WindowsNotepad_1.0.0.0_x64__8wekyb3d8bbwe\Notepad\Notepad.exe",
        );

        assert_eq!(
            package_folder_from_path(app_path),
            Some(PathBuf::from(
                r"C:\Program Files\WindowsApps\Microsoft.WindowsNotepad_1.0.0.0_x64__8wekyb3d8bbwe"
            ))
        );
    }

    #[test]
    fn reads_the_logo_from_a_manifest() {
        let manifest = r#"
            <Package>
              <Properties>
                <DisplayName>Calculator</DisplayName>
                <Logo>Assets\StoreLogo.png</Logo>
              </Properties>
            </Package>
        "#;

        assert_eq!(
            extract_icon_path(manifest).unwrap(),
            r"Assets\StoreLogo.png"
        );
    }

    #[test]
    fn fails_on_a_manifest_without_a_logo() {
        assert!(extract_icon_path("<Package></Package>").is_err());
        assert!(extract_icon_path("</Logo> out of order <Logo>").is_err());
    }
}
