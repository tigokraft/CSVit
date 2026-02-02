#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod backend;
pub mod gui;

use clap::Parser;
use std::path::PathBuf;
use anyhow::Result;
use crate::backend::loader::CsvLoader;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the CSV file to open
    #[arg(short, long)]
    file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    let (loader, filename) = if let Some(path) = args.file {
         let path_str = path.to_string_lossy().to_string();
         println!("Loading file: {:?}", path);
         let loader = CsvLoader::new(&path)?;
         println!("File loaded. {} records found.", loader.total_records());
         (Some(std::sync::Arc::new(loader)), Some(path_str))
    } else {
        (None, None)
    };
    
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 900.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "CSVit",
        native_options,
        Box::new(move |cc| Ok(Box::new(crate::gui::app::GuiApp::new(cc, loader.clone(), filename.clone())))),
    ).map_err(|e| anyhow::anyhow!("Eframe error: {}", e))?;

    Ok(())
}

