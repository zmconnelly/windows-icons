mod utils {
    pub mod image_utils;
    pub mod process_utils;
}
mod dll_icons;
mod uwp_apps;

pub use dll_icons::DllIcon;
use dll_icons::get_dll_hicon_to_image;
use utils::image_utils::{get_hicon_to_image, image_to_base64};
use utils::process_utils::get_process_path;
use uwp_apps::{get_uwp_icon, get_uwp_icon_base64};

use std::{error::Error, path::Path};

use image::RgbaImage;

/// UWP apps are installed to `<Program Files>\WindowsApps\<package folder>\...`. The
/// `Program Files` segment is localized (`Programme` on German Windows, and so on), so
/// `WindowsApps` is the only segment that can be matched reliably.
///
/// Requiring a package folder below `WindowsApps` keeps the app execution aliases in
/// `%LOCALAPPDATA%\Microsoft\WindowsApps` out, since those sit directly in that folder
/// and carry no package manifest.
fn is_uwp_app(path: &Path) -> bool {
    let path = path.to_string_lossy();

    let is_wsa = path.to_lowercase().contains("windowssubsystemforandroid");
    if is_wsa {
        return false;
    }

    let mut segments = path.split(['/', '\\']).filter(|s| !s.is_empty());
    if !segments.any(|s| s.eq_ignore_ascii_case("WindowsApps")) {
        return false;
    }

    // A package folder and a file name must follow for this to be an installed app.
    segments.nth(1).is_some()
}

pub fn get_icon_by_path<P: AsRef<Path>>(path: P) -> Result<RgbaImage, Box<dyn Error>> {
    let path = path.as_ref();
    if is_uwp_app(path) {
        get_uwp_icon(path)
    } else {
        get_hicon_to_image(path)
    }
}

pub fn get_icon_base64_by_path<P: AsRef<Path>>(path: P) -> Result<String, Box<dyn Error>> {
    let path = path.as_ref();
    if is_uwp_app(path) {
        get_uwp_icon_base64(path)
    } else {
        let icon_image = get_icon_by_path(path)?;
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

#[cfg(test)]
mod tests {
    use super::is_uwp_app;
    use std::path::Path;

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
}
