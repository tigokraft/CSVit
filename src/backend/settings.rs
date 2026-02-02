use serde::{Deserialize, Serialize};
use std::fs;
use directories::ProjectDirs;

#[derive(PartialEq, Clone, Copy, Serialize, Deserialize, Debug)]
pub enum Theme {
    System,
    Dark,
    Light,
    Monokai,
    Solarized,
    Nord,
    Dracula,
    Catppuccin,
}

impl Theme {
    pub fn all() -> &'static [Theme] {
        &[
            Theme::System,
            Theme::Dark,
            Theme::Light,
            Theme::Monokai,
            Theme::Solarized,
            Theme::Nord,
            Theme::Dracula,
            Theme::Catppuccin,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Theme::System => "System",
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Monokai => "Monokai",
            Theme::Solarized => "Solarized",
            Theme::Nord => "Nord",
            Theme::Dracula => "Dracula",
            Theme::Catppuccin => "Catppuccin",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: Theme,
    pub font_size: f32,
    pub row_height: f32,
    pub use_edit_modal: bool,
    #[serde(default)]
    pub auto_beautify_json: bool,
    #[serde(default)]
    pub recent_files: Vec<String>,
    #[serde(default = "default_max_recent")]
    pub max_recent_files: usize,
    #[serde(default)]
    pub stripe_color: Option<[u8; 3]>,
}

fn default_max_recent() -> usize {
    10
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            font_size: 14.0,
            row_height: 24.0,
            use_edit_modal: false,
            auto_beautify_json: false,
            recent_files: Vec::new(),
            max_recent_files: 10,
            stripe_color: None,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "csvit") {
            let config_dir = proj_dirs.config_dir();
            let config_path = config_dir.join("config.json");
            
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(settings) = serde_json::from_str(&content) {
                        return settings;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn add_recent_file(&mut self, path: &str) {
        // Remove if already exists
        self.recent_files.retain(|p| p != path);
        // Add to front
        self.recent_files.insert(0, path.to_string());
        // Trim to max
        self.recent_files.truncate(self.max_recent_files);
        self.save();
    }

    pub fn save(&self) {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "csvit") {
            let config_dir = proj_dirs.config_dir();
            if !config_dir.exists() {
                let _ = fs::create_dir_all(config_dir);
            }
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
