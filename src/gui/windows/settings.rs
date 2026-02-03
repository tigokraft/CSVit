use eframe::egui;
use crate::backend::settings::{Settings, Theme, KeybindingMode, KeyCombo};


pub struct SettingsWindow {
    selected_tab: SettingsTab,
    key_capture: Option<&'static str>, 
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum SettingsTab {
    General,
    Keybindings,
    Theme,
}

impl SettingsWindow {
    pub fn new() -> Self {
        Self {
            selected_tab: SettingsTab::General,
            key_capture: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool, settings: &mut Settings) {
        egui::Window::new("Settings")
            .open(open)
            .min_width(400.0)
            .min_height(300.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.selected_tab, SettingsTab::General, "General");
                    ui.selectable_value(&mut self.selected_tab, SettingsTab::Keybindings, "Keybindings");
                    ui.selectable_value(&mut self.selected_tab, SettingsTab::Theme, "Theme");
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.selected_tab {
                        SettingsTab::General => self.show_general(ui, settings),
                        SettingsTab::Keybindings => self.show_keybindings(ui, ctx, settings),
                        SettingsTab::Theme => self.show_theme(ui, settings),
                    }
                });
                
                ui.separator();
                ui.horizontal(|ui| {
                     if ui.button("Save Settings").clicked() {
                         settings.save();
                     }
                     if ui.button("Reset to Defaults").clicked() {
                         *settings = Settings::default();
                     }
                });
            });
    }

    fn show_general(&mut self, ui: &mut egui::Ui, settings: &mut Settings) {
        ui.heading("Font");
        egui::ComboBox::from_id_salt("font_selector")
            .selected_text(&settings.font_family)
            .show_ui(ui, |ui| {
                for font in Settings::available_fonts() {
                    let selected = settings.font_family == font;
                    if ui.selectable_label(selected, font).clicked() {
                        settings.font_family = font.to_string();
                    }
                }
            });
        
        ui.separator();
        ui.heading("Appearance");
        ui.add(egui::Slider::new(&mut settings.font_size, 10.0..=24.0).text("Font Size"));
        ui.add(egui::Slider::new(&mut settings.row_height, 20.0..=60.0).text("Row Height"));
        
        ui.separator();
        ui.heading("Behavior");
        ui.checkbox(&mut settings.use_edit_modal, "Use Popup for Editing");
        ui.checkbox(&mut settings.auto_beautify_json, "Auto-beautify JSON in Popup");
        ui.checkbox(&mut settings.show_profile_hud, "Show Column Profile HUD (Ctrl+B)");

        ui.separator();
        ui.heading("Recent Files");
        ui.add(egui::Slider::new(&mut settings.max_recent_files, 1..=20).text("Max Recent Files"));
        if ui.button("Clear Recent Files").clicked() {
            settings.recent_files.clear();
        }
    }

    fn show_keybindings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, settings: &mut Settings) {
        let key_capture = &mut self.key_capture;
        let keymap = &mut settings.keymap;

        ui.heading("Keybinding Mode");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut settings.keybinding_mode, KeybindingMode::Standard, "Standard (GUI)");
            ui.selectable_value(&mut settings.keybinding_mode, KeybindingMode::Vim, "Vim (Modal)");
        });
        ui.label(egui::RichText::new("Note: Keybindings apply to Standard mode and global shortcuts.").weak().small());
        
        ui.separator();
        ui.heading("Shortcuts");
        
        egui::Grid::new("keybinds_grid").striped(true).show(ui, |ui| {
            Self::key_binder(ui, ctx, key_capture, "Move Up", "move_up", &mut keymap.move_up);
            Self::key_binder(ui, ctx, key_capture, "Move Down", "move_down", &mut keymap.move_down);
            Self::key_binder(ui, ctx, key_capture, "Move Left", "move_left", &mut keymap.move_left);
            Self::key_binder(ui, ctx, key_capture, "Move Right", "move_right", &mut keymap.move_right);
            ui.end_row();

            Self::key_binder(ui, ctx, key_capture, "Undo", "undo", &mut keymap.undo);
            Self::key_binder(ui, ctx, key_capture, "Redo", "redo", &mut keymap.redo);
            Self::key_binder(ui, ctx, key_capture, "Save", "save", &mut keymap.save);
            Self::key_binder(ui, ctx, key_capture, "Toggle HUD", "toggle_hud", &mut keymap.toggle_hud);
            ui.end_row();
        });
    }
    
    fn key_binder(
        ui: &mut egui::Ui, 
        ctx: &egui::Context, 
        key_capture: &mut Option<&'static str>, 
        label: &str, 
        id: &'static str, 
        combo: &mut KeyCombo
    ) {
        ui.label(label);
        
        let is_capturing = *key_capture == Some(id);
        let btn_text = if is_capturing {
            "Please press a key pattern...".to_string()
        } else {
            let mut s = String::new();
            if combo.modifiers.ctrl { s.push_str("Ctrl+"); }
            if combo.modifiers.alt { s.push_str("Alt+"); }
            if combo.modifiers.shift { s.push_str("Shift+"); }
            if combo.modifiers.command { s.push_str("Cmd+"); }
            s.push_str(&format!("{:?}", combo.key));
            s
        };
        
        if ui.button(btn_text).clicked() {
            *key_capture = Some(id);
        }
        
        if is_capturing {
            // Check for key events directly to catch the press
            let mut captured = false;
            let mut captured_combo = None;

            ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Key { key, pressed: true, modifiers, .. } = event {
                        // Ignore pure modifier presses (like just pressing Ctrl)
                        // egui doesn't have a key variant for "Modifiers", but modifiers are tracked separately
                        // We check if the key itself is a modifier key? 
                        // Actually, just check if it's a "real" key.
                        // But egui::Key includes all keys.
                        
                        // We want to capture e.g. 'S' with modifiers.
                        // If key is e.g. Key::Escape and no modifiers, we cancel?
                        
                        if *key == egui::Key::Escape && modifiers.is_none() {
                            captured = true; // Cancel
                            break;
                        }
                        
                        // Accept the key press
                        captured_combo = Some(KeyCombo { 
                            key: *key, 
                            modifiers: *modifiers 
                        });
                        captured = true;
                        break; 
                    }
                }
            });

            if captured {
                if let Some(new_combo) = captured_combo {
                    *combo = new_combo;
                }
                *key_capture = None;
            }
        }
        
        ui.end_row();
    }

    fn show_theme(&mut self, ui: &mut egui::Ui, settings: &mut Settings) {
         ui.heading("Built-in Themes");
         egui::ComboBox::from_id_salt("theme_selector")
            .selected_text(settings.theme.name())
            .show_ui(ui, |ui| {
                for theme in Theme::builtin_all() {
                    let selected = settings.theme == *theme;
                    if ui.selectable_label(selected, theme.name()).clicked() {
                        settings.theme = *theme;
                    }
                }
                
                if !settings.custom_themes.is_empty() {
                    ui.separator();
                    for (i, custom) in settings.custom_themes.iter().enumerate() {
                        let theme = Theme::Custom(i);
                        let selected = settings.theme == theme;
                        if ui.selectable_label(selected, &custom.name).clicked() {
                            settings.theme = theme;
                        }
                    }
                }
            });
            
        ui.separator();
        ui.heading("Workspace Colors");
        let mut stripe_enabled = settings.stripe_color.is_some();
        if ui.checkbox(&mut stripe_enabled, "Enable Striped Rows").changed() {
            if stripe_enabled {
                settings.stripe_color = Some([40, 40, 50]); // Default dark stripe
            } else {
                settings.stripe_color = None;
            }
        }
        
        if let Some(ref mut color) = settings.stripe_color {
             ui.horizontal(|ui| {
                 ui.label("Stripe Color:");
                 let mut rgb = *color;
                 if ui.color_edit_button_srgb(&mut rgb).changed() {
                     *color = rgb;
                 }
             });
        }
    }
}
