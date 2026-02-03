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
    Custom(usize), // Index into custom_themes
}

impl Theme {
    pub fn builtin_all() -> &'static [Theme] {
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
            Theme::Custom(_) => "Custom",
        }
    }
}

/// Custom theme definition - can be loaded from JSON files
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomTheme {
    pub name: String,
    pub bg_primary: [u8; 3],
    pub bg_secondary: [u8; 3],
    pub text_primary: [u8; 3],
    pub text_secondary: [u8; 3],
    pub accent: [u8; 3],
    pub selection: [u8; 3],
    pub border: [u8; 3],
    #[serde(default)]
    pub stripe: Option<[u8; 3]>,
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self {
            name: "Custom".to_string(),
            bg_primary: [30, 30, 46],
            bg_secondary: [49, 50, 68],
            text_primary: [205, 214, 244],
            text_secondary: [166, 173, 200],
            accent: [203, 166, 247],
            selection: [69, 71, 90],
            border: [88, 91, 112],
            stripe: None,
        }
    }
}

/// Keybinding mode - Standard (GUI) or Vim (modal)
#[derive(PartialEq, Clone, Copy, Serialize, Deserialize, Debug, Default)]
pub enum KeybindingMode {
    #[default]
    Standard,  // Traditional GUI with Ctrl+S, Ctrl+Z, mouse-first
    Vim,       // Modal editing with hjkl, :commands, keyboard-first
}

impl KeybindingMode {
    pub fn name(&self) -> &'static str {
        match self {
            KeybindingMode::Standard => "Standard",
            KeybindingMode::Vim => "Vim",
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
    #[serde(default)]
    pub custom_themes: Vec<CustomTheme>,
    #[serde(default = "default_font")]
    pub font_family: String,
    #[serde(default)]
    pub keybinding_mode: KeybindingMode,
    #[serde(default)]
    pub show_profile_hud: bool,
}

fn default_max_recent() -> usize {
    10
}

fn default_font() -> String {
    "JetBrains Mono".to_string()
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
            custom_themes: Vec::new(),
            font_family: default_font(),
            keybinding_mode: KeybindingMode::Standard,
            show_profile_hud: false,
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

    /// Load custom themes from the themes directory
    pub fn load_custom_themes(&mut self) {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "csvit") {
            let themes_dir = proj_dirs.config_dir().join("themes");
            if themes_dir.exists() {
                if let Ok(entries) = fs::read_dir(&themes_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map(|e| e == "json").unwrap_or(false) {
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(theme) = serde_json::from_str::<CustomTheme>(&content) {
                                    // Only add if not already present
                                    if !self.custom_themes.iter().any(|t| t.name == theme.name) {
                                        self.custom_themes.push(theme);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get custom theme by index
    pub fn get_custom_theme(&self, idx: usize) -> Option<&CustomTheme> {
        self.custom_themes.get(idx)
    }

    /// Available fonts
    pub fn available_fonts() -> Vec<&'static str> {
        vec![
            "JetBrains Mono",
            "Fira Code", 
            "SF Mono",
            "Menlo",
            "Monaco",
            "Consolas",
            "Source Code Pro",
        ]
    }
}
