use windows_icons::{get_icon_base64_by_path, get_icon_by_path, get_icon_by_path_with_size, IconSize};

fn main() {
    let _ = std::fs::create_dir("output");

    let icon = get_icon_by_path("C:\\Windows\\System32\\notepad.exe").unwrap();
    icon.save("output/notepad.png").unwrap();

    let icon = get_icon_by_path("C:\\Windows\\System32\\calc.exe").unwrap();
    icon.save("output/calc.png").unwrap();

    let icon = get_icon_by_path("C:\\Windows\\System32\\cmd.exe").unwrap();
    icon.save("output/cmd.png").unwrap();

    for (size, name) in [
        (IconSize::Small, "small"),
        (IconSize::Medium, "medium"),
        (IconSize::Large, "large"),
        (IconSize::ExtraLarge, "extralarge"),
    ] {
        let icon = get_icon_by_path_with_size("C:\\Windows\\System32\\notepad.exe", size).unwrap();
        icon.save(format!("output/notepad_{name}.png")).unwrap();
        println!("notepad {name}: {}x{}", icon.width(), icon.height());
    }

    let base64 = get_icon_base64_by_path("C:\\Windows\\System32\\notepad.exe").unwrap();
    println!("Notepad: {}", base64);

    let base64 = get_icon_base64_by_path("C:\\Windows\\System32\\calc.exe").unwrap();
    println!("Calc: {}", base64);

    let base64 = get_icon_base64_by_path("C:\\Windows\\System32\\cmd.exe").unwrap();
    println!("Cmd: {}", base64);
}
