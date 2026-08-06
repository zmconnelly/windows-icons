use windows_icons::{get_icon_base64_by_process_id, get_icon_by_process_id};

fn main() {
    let _ = std::fs::create_dir("output");

    let process_id: u32 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| {
            eprintln!("usage: cargo run --example process -- <process id>");
            std::process::exit(1);
        })
        .parse()
        .expect("process id must be a number");

    let icon = get_icon_by_process_id(process_id).unwrap();
    icon.save("output/process.png").unwrap();

    let base64 = get_icon_base64_by_process_id(process_id).unwrap();
    println!("Process {}: {}", process_id, base64);
}
