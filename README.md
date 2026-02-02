# CSV Editor

A high-performance, modern CSV editor built with Rust and egui. Designed to handle large files efficiently using memory mapping.

## Features

- **High Performance**: Uses memory-mapped files (`memmap2`) to open and navigate large CSV datasets instantly.
- **Modern UI**: Clean, dark-mode interface with a "shadcn-like" aesthetic.
- **Dual View Modes**: Switch between a structured **Table View** and raw **Text View**.
- **Editing**: Double-click any cell to edit its content.
- **JSON Support**: 
    - Export the entire file to JSON.
    - Right-click any row to view it as a JSON object.
- **Word Wrap**: Toggle word wrapping for long cell content.

## Prerequisites

To build and run this application, you need the Rust toolchain installed on your system.

- [Install Rust](https://www.rust-lang.org/tools/install)

## Building from Source

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd csv_editor
   ```

2. Build and run in release mode (recommended for best performance):
   ```bash
   cargo run --release
   ```

   Or just build the executable:
   ```bash
   cargo build --release
   ```
   The binary will be located at `target/release/csv_editor.exe`.

## Usage

### Opening Files
You can open a file in two ways:
1. **GUI**: file Launch the app and click the **"Open File"** button to select a CSV via the system file dialog.
2. **Command Line**: Pass the file path as an argument:
   ```bash
   cargo run --release -- --file "path/to/your/data.csv"
   ```

### Controls
- **Edit Cell**: Double-click on any cell in the table to start editing. Press `Enter` to confirm or `Escape` to cancel.
- **Context Menu**: Right-click on a row to see options like "View Row as JSON".
- **View Modes**: Use the toggle in the top-right corner to switch between Table and Text views.
- **Export**: Click the "Export JSON" button to save the current CSV data as a JSON file.

## Tech Stack
- **Language**: Rust
- **GUI Framework**: [egui](https://github.com/emilk/egui) / [eframe](https://github.com/emilk/egui/tree/master/crates/eframe)
- **CSV Parsing**: [csv](https://github.com/BurntSushi/rust-csv)
- **File Dialogs**: [rfd](https://github.com/PolyMeilex/rfd)
