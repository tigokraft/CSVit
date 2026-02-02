use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;

#[derive(PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Theme {
    System,
    Dark,
    Light,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: Theme,
    pub font_size: f32,
    pub row_height: f32,
    pub use_edit_modal: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            font_size: 14.0,
            row_height: 24.0,
            use_edit_modal: false,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "csvit") {
            let config_dir = proj_dirs.config_dir();
            let config_path = config_dir.join("config.toml");
            
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    // Try parsing as TOML (need to add toml dependency ideally, or just use JSON/MsgPack. 
                    // Actually, let's use serde_json for simplicity unless user asked for specific format.
                    // Wait, config usually TOML. But JSON is built-in to typical deps here or easy to add. 
                    // Let's check dependencies. We have serde_json. 
                    // Let's stick to JSON for now to avoid adding another crate if not needed, 
                    // but standard is usually TOML/YAML on Linux/Mac. 
                    // Wait, `toml` crate is not in Cargo.toml. 
                    // I will use JSON for now.
                    if let Ok(settings) = serde_json::from_str(&content) {
                        return settings;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "csvit") {
            let config_dir = proj_dirs.config_dir();
            if !config_dir.exists() {
                let _ = fs::create_dir_all(config_dir);
            }
            let config_path = config_dir.join("config.toml"); // Naming it .toml but content is JSON? Bad practice. 
            // Let's name it config.json
            let config_path = config_dir.join("config.json");
            
            if let Ok(content) = serde_json::to_string_pretty(self) {
                 let _ = fs::write(config_path, content);
            }
        }
    }

    pub fn reset() {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "csvit") {
             let config_dir = proj_dirs.config_dir();
             let config_path = config_dir.join("config.json");
             if config_path.exists() {
                 let _ = fs::remove_file(config_path);
             }
        }
    }
}
