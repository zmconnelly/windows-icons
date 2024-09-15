use windows_icons::{
    get_icon_base64_by_path, get_icon_base64_by_process_id, get_icon_by_path,
    get_icon_by_process_id,
};

fn main() {
    let _ = std::fs::create_dir("output");

    let icon = get_icon_by_path("C:\\Windows\\System32\\notepad.exe");
    icon.save("output/notepad.png").unwrap();

    let icon = get_icon_by_path("C:\\Windows\\System32\\calc.exe");
    icon.save("output/calc.png").unwrap();

    let icon = get_icon_by_path("C:\\Windows\\System32\\cmd.exe");
    icon.save("output/cmd.png").unwrap();

    let base64 = get_icon_base64_by_path("C:\\Windows\\System32\\notepad.exe");
    println!("Notepad: {}", base64);

    let base64 = get_icon_base64_by_path("C:\\Windows\\System32\\calc.exe");
    println!("Calc: {}", base64);

    let base64 = get_icon_base64_by_path("C:\\Windows\\System32\\cmd.exe");
    println!("Cmd: {}", base64);

    // Substitute the process id to test
    let process_id = 0000;

    let base64 = get_icon_base64_by_process_id(process_id);
    println!("Process {}: {}", process_id, base64);

    let icon = get_icon_by_process_id(process_id);
    icon.save("output/process.png").unwrap();
}
