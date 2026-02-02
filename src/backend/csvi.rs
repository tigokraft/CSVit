use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::formatting::FormatMap;

/// Metadata stored in the .csvi archive
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CsviMetadata {
    pub version: u32,
    pub formatting: FormatMap,
    pub column_names: Vec<String>,
    pub column_widths: Vec<f32>,
    #[serde(default)]
    pub view_settings: ViewSettings,
}

/// View settings to restore editor state
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ViewSettings {
    pub scroll_position: f32,
    pub selected_cell: Option<(usize, usize)>,
    pub zoom_level: f32,
}

impl CsviMetadata {
    pub fn new() -> Self {
        Self {
            version: 1,
            formatting: FormatMap::new(),
            column_names: Vec::new(),
            column_widths: Vec::new(),
            view_settings: ViewSettings::default(),
        }
    }
}

/// Save data and metadata as a .csvi archive
pub fn save_csvi(path: &Path, csv_data: &str, metadata: &CsviMetadata) -> Result<()> {
    let file = File::create(path).context("Failed to create .csvi file")?;
    let mut zip = ZipWriter::new(file);
    
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    // Write CSV data
    zip.start_file("data.csv", options)
        .context("Failed to add data.csv to archive")?;
    zip.write_all(csv_data.as_bytes())
        .context("Failed to write CSV data")?;

    // Write metadata
    let metadata_json = serde_json::to_string_pretty(metadata)
        .context("Failed to serialize metadata")?;
    zip.start_file("metadata.json", options)
        .context("Failed to add metadata.json to archive")?;
    zip.write_all(metadata_json.as_bytes())
        .context("Failed to write metadata")?;

    zip.finish().context("Failed to finalize archive")?;
    Ok(())
}

/// Load a .csvi archive
pub fn load_csvi(path: &Path) -> Result<(String, CsviMetadata)> {
    let file = File::open(path).context("Failed to open .csvi file")?;
    let mut archive = ZipArchive::new(file).context("Failed to read .csvi archive")?;

    // Read CSV data
    let mut csv_data = String::new();
    {
        let mut csv_file = archive
            .by_name("data.csv")
            .context("data.csv not found in archive")?;
        csv_file
            .read_to_string(&mut csv_data)
            .context("Failed to read CSV data")?;
    }

    // Read metadata
    let metadata = {
        let mut meta_file = archive
            .by_name("metadata.json")
            .context("metadata.json not found in archive")?;
        let mut meta_str = String::new();
        meta_file
            .read_to_string(&mut meta_str)
            .context("Failed to read metadata")?;
        serde_json::from_str(&meta_str).context("Failed to parse metadata")?
    };

    Ok((csv_data, metadata))
}

/// Export only the CSV data (no formatting)
pub fn export_csv(path: &Path, csv_data: &str) -> Result<()> {
    std::fs::write(path, csv_data).context("Failed to write CSV file")?;
    Ok(())
}

/// Check if a file is a .csvi archive
pub fn is_csvi_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext.eq_ignore_ascii_case("csvi"))
        .unwrap_or(false)
}
