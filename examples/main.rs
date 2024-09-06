use windows_icons::{get_icon_base64_by_path, get_icon_image_by_path};

fn main() {
    let _ = std::fs::create_dir("output");

    let icon = get_icon_image_by_path("C:\\Windows\\System32\\notepad.exe");
    icon.save("output/notepad.png").unwrap();

    let icon = get_icon_image_by_path("C:\\Windows\\System32\\calc.exe");
    icon.save("output/calc.png").unwrap();

    let icon = get_icon_image_by_path("C:\\Windows\\System32\\cmd.exe");
    icon.save("output/cmd.png").unwrap();

    let base64 = get_icon_base64_by_path("C:\\Windows\\System32\\notepad.exe");
    println!("Notepad: {}", base64);

    let base64 = get_icon_base64_by_path("C:\\Windows\\System32\\calc.exe");
    println!("Calc: {}", base64);

    let base64 = get_icon_base64_by_path("C:\\Windows\\System32\\cmd.exe");
    println!("Cmd: {}", base64);
}
