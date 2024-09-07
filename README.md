# Windows Icons

A simple Rust library to extract icons from files and processes on Windows.

## Features

- Extract icons from files by path
- Extract icons from running processes by process ID
- Convert icons to PNG images
- Convert icons to base64-encoded strings

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
windows-icons = "0.1.0"
```

## Usage

Here are some examples of how to use the Windows Icons library:

```rust
// Get icon as an image from a file path
let icon = get_icon_image_by_path("C:\\Windows\\System32\\notepad.exe");
icon.save("notepad.png").unwrap();

// Get icon as a base64 string from a file path
let base64 = get_icon_base64_by_path("C:\\Windows\\System32\\calc.exe");
println!("Calculator icon: {}", base64);

// Get icon as an image from a process ID
let process_id = 1234;

let icon = get_icon_image_by_process_id(process_id);
icon.save("process.png").unwrap();

// Get icon as a base64 encoded string from a process ID
let base64 = get_icon_base64_by_process_id(process_id);
println!("Process {} icon: {}", process_id, base64);
```

For more examples, check the `examples/main.rs` file in the repository.

## API

The library provides the following functionality:

- `get_icon_by_path(path: &str) -> RgbaImage`
- `get_icon_base64_by_path(path: &str) -> String`
- `get_icon_by_process_id(process_id: u32) -> RgbaImage`
- `get_icon_base64_by_process_id(process_id: u32) -> String`

## Requirements

This library is designed to work on Windows systems only.

## License

This project is licensed under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgements

This library uses the following crates:

- `image` for image processing
- `base64` for base64 encoding
- `winapi` and `windows` for Windows API interactions

## Disclaimer

This library uses unsafe Rust code to interact with the Windows API. While efforts have been made to ensure safety, use it at your own risk.
