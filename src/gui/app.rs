use std::sync::Arc;
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use crate::backend::loader::CsvLoader;
use crate::backend::paged_reader::PagedReader;
use crate::backend::editor::EditBuffer;
use crate::backend::parser::CsvParser;



use crate::backend::settings::{Settings, Theme};

#[derive(PartialEq)]
pub enum ViewMode {
    Table,
    Text,
    Graph,
}

pub struct EditorState {
    loader: Arc<CsvLoader>,
    reader: PagedReader,
    editor: EditBuffer,
    view_mode: ViewMode,
    input_buffer: String,
    editing_cell: Option<(usize, usize)>,
    filename: String,
    word_wrap: bool,
    json_modal: Option<(usize, String)>,
    num_columns: usize,
    column_widths: Vec<f32>,
    selected_cell: Option<(usize, usize)>,
    edit_modal: Option<(usize, usize, String)>,
    // Graph state
    graph_x_col: usize,
    graph_y_col: usize,
    graph_data: Vec<[f64; 2]>,
    // In-memory grid for new/edited files
    grid: Option<crate::backend::grid::EditableGrid>,
}

pub enum AppState {
    Welcome,
    Editor(EditorState),
    Loading(String), // Show loading spinner
    Error(String),
}

pub struct GuiApp {
    state: AppState,
    settings: Settings,
    show_settings: bool,
    show_new_csv_dialog: bool,
    new_csv_columns: usize,
    new_csv_rows: usize,
}

impl GuiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, loader: Option<Arc<CsvLoader>>, filename: Option<String>) -> Self {
        let mut settings = Settings::load();
        
        let state = if let (Some(loader), Some(name)) = (loader, filename.clone()) {
            if let Some(ref path) = filename {
                settings.add_recent_file(path);
            }
             AppState::Editor(EditorState {
                loader: loader.clone(),
                reader: PagedReader::new(loader.clone()),
                editor: EditBuffer::new(),
                view_mode: ViewMode::Table,
                input_buffer: String::new(),
                editing_cell: None,
                filename: name,
                word_wrap: false,
                json_modal: None,
                num_columns: loader.num_columns(),
                column_widths: loader.estimate_column_widths(),
                selected_cell: None,
                edit_modal: None,
                graph_x_col: 0,
                graph_y_col: 1,
                graph_data: Vec::new(),
                grid: None,
            })
        } else {
            AppState::Welcome
        };
        
        Self { 
            state,
            settings,
            show_settings: false,
            show_new_csv_dialog: false,
            new_csv_columns: 3,
            new_csv_rows: 10,
        }
    }

    fn load_file(&mut self, path: &str) {
        self.state = AppState::Loading(path.to_string());
        match CsvLoader::new(std::path::Path::new(path)) {
            Ok(loader) => {
                let arc_loader = Arc::new(loader);
                self.settings.add_recent_file(path);
                self.state = AppState::Editor(EditorState {
                    loader: arc_loader.clone(),
                    reader: PagedReader::new(arc_loader.clone()),
                    editor: EditBuffer::new(),
                    view_mode: ViewMode::Table,
                    input_buffer: String::new(),
                    editing_cell: None,
                    filename: path.to_string(),
                    word_wrap: false,
                    json_modal: None,
                    num_columns: arc_loader.num_columns(),
                    column_widths: arc_loader.estimate_column_widths(),
                    selected_cell: None,
                    edit_modal: None,
                    graph_x_col: 0,
                    graph_y_col: 1,
                    graph_data: Vec::new(),
                    grid: None,
                });
            }
            Err(e) => {
                self.state = AppState::Error(format!("Failed to load file: {}", e));
            }
        }
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).pick_file() {
            let path_str = path.to_string_lossy().to_string();
            self.load_file(&path_str);
        }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_style(ctx, &self.settings); 

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
             ui.horizontal(|ui| {
                 ui.menu_button("File", |ui| {
                     if ui.button("📄 New CSV").clicked() {
                         self.show_new_csv_dialog = true;
                         ui.close();
                     }
                     if ui.button("📂 Open").clicked() {
                         self.open_file_dialog();
                         ui.close();
                     }
                     ui.separator();
                     ui.menu_button("Recent Files", |ui| {
                         if self.settings.recent_files.is_empty() {
                             ui.label("No recent files");
                         } else {
                             for path in self.settings.recent_files.clone() {
                                 let display_name = std::path::Path::new(&path)
                                     .file_name()
                                     .map(|n| n.to_string_lossy().to_string())
                                     .unwrap_or_else(|| path.clone());
                                 if ui.button(&display_name).on_hover_text(&path).clicked() {
                                     self.load_file(&path);
                                     ui.close();
                                 }
                             }
                         }
                     });
                 });
                 if ui.button("⚙ Settings").clicked() {
                     self.show_settings = true;
                 }
             });
        });

        // Settings Window
        if self.show_settings {
             let mut open = true;
             egui::Window::new("Settings")
                .open(&mut open)
                .resizable(true)
                .min_width(350.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("Theme");
                        egui::ComboBox::from_id_salt("theme_selector")
                            .selected_text(match &self.settings.theme {
                                Theme::Custom(idx) => {
                                    self.settings.custom_themes.get(*idx)
                                        .map(|t| t.name.as_str())
                                        .unwrap_or("Custom")
                                }
                                t => t.name(),
                            })
                            .show_ui(ui, |ui| {
                                for theme in Theme::builtin_all() {
                                    ui.selectable_value(&mut self.settings.theme, *theme, theme.name());
                                }
                                ui.separator();
                                for (i, custom) in self.settings.custom_themes.iter().enumerate() {
                                    ui.selectable_value(&mut self.settings.theme, Theme::Custom(i), &custom.name);
                                }
                            });
                        
                        ui.horizontal(|ui| {
                            if ui.button("📂 Import Theme").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("JSON Theme", &["json"])
                                    .pick_file()
                                {
                                    if let Ok(content) = std::fs::read_to_string(&path) {
                                        if let Ok(theme) = serde_json::from_str::<crate::backend::settings::CustomTheme>(&content) {
                                            self.settings.custom_themes.push(theme);
                                            self.settings.save();
                                        }
                                    }
                                }
                            }
                            if ui.button("🔄 Reload Themes").clicked() {
                                self.settings.load_custom_themes();
                            }
                        });
                        
                        ui.separator();
                        ui.heading("Font");
                        egui::ComboBox::from_id_salt("font_selector")
                            .selected_text(&self.settings.font_family)
                            .show_ui(ui, |ui| {
                                for font in Settings::available_fonts() {
                                    let selected = self.settings.font_family == font;
                                    if ui.selectable_label(selected, font).clicked() {
                                        self.settings.font_family = font.to_string();
                                    }
                                }
                            });
                        
                        ui.separator();
                        ui.heading("Appearance");
                        ui.add(egui::Slider::new(&mut self.settings.font_size, 10.0..=24.0).text("Font Size"));
                        ui.add(egui::Slider::new(&mut self.settings.row_height, 20.0..=60.0).text("Row Height"));
                        
                        ui.separator();
                        ui.heading("Behavior");
                        ui.checkbox(&mut self.settings.use_edit_modal, "Use Popup for Editing");
                        ui.checkbox(&mut self.settings.auto_beautify_json, "Auto-beautify JSON in Popup");
                        
                        ui.separator();
                        ui.heading("Recent Files");
                        ui.add(egui::Slider::new(&mut self.settings.max_recent_files, 1..=20).text("Max Recent Files"));
                        if ui.button("Clear Recent Files").clicked() {
                            self.settings.recent_files.clear();
                        }
                        
                        ui.separator();
                        ui.horizontal(|ui| {
                             if ui.button("Save Settings").clicked() {
                                 self.settings.save();
                             }
                             if ui.button("Reset to Defaults").clicked() {
                                 Settings::reset();
                                 self.settings = Settings::load();
                             }
                        });
                    });
                });
             if !open {
                 self.show_settings = false;
                 self.settings.save();
             }
        }
        // New CSV Dialog
        if self.show_new_csv_dialog {
            let mut open = true;
            egui::Window::new("Create New CSV")
                .open(&mut open)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Columns:");
                        ui.add(egui::DragValue::new(&mut self.new_csv_columns).range(1..=100));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Rows:");
                        ui.add(egui::DragValue::new(&mut self.new_csv_rows).range(1..=1000));
                    });
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            // Create an in-memory CSV structure
                            let cols = self.new_csv_columns;
                            let rows = self.new_csv_rows;
                            let default_widths: Vec<f32> = (0..cols).map(|_| 100.0).collect();
                            let grid = crate::backend::grid::EditableGrid::new(cols, rows);
                            self.state = AppState::Editor(EditorState {
                                loader: Arc::new(CsvLoader::empty(cols, rows)),
                                reader: PagedReader::empty(),
                                editor: EditBuffer::new(),
                                view_mode: ViewMode::Table,
                                input_buffer: String::new(),
                                editing_cell: None,
                                filename: "Untitled.csv".to_string(),
                                word_wrap: false,
                                json_modal: None,
                                num_columns: cols,
                                column_widths: default_widths,
                                selected_cell: None,
                                edit_modal: None,
                                graph_x_col: 0,
                                graph_y_col: 1.min(cols.saturating_sub(1)),
                                graph_data: Vec::new(),
                                grid: Some(grid),
                            });
                            self.show_new_csv_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_new_csv_dialog = false;
                        }
                    });
                });
            if !open {
                self.show_new_csv_dialog = false;
            }
        }

        // Handle Drag & Drop
        if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
            let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
            if let Some(file) = dropped_files.first() {
                if let Some(path) = &file.path {
                    let path_str = path.to_string_lossy().to_string();
                    self.load_file(&path_str);
                }
            }
        }

        let mut next_state = None;

        match &mut self.state {
            AppState::Welcome => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        ui.heading(egui::RichText::new("CSVit").size(48.0).strong());
                        ui.label(egui::RichText::new("High performance editor for large CSV files").size(16.0).color(egui::Color32::from_gray(150)));
                        ui.add_space(30.0);
                        
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 220.0);
                            if ui.add(egui::Button::new(egui::RichText::new("📄 New CSV").size(16.0))
                                .min_size(egui::vec2(140.0, 45.0))
                                .corner_radius(6.0)
                            ).clicked() {
                                self.show_new_csv_dialog = true;
                            }
                            ui.add_space(20.0);
                            if ui.add(egui::Button::new(egui::RichText::new("📂 Open File").size(16.0))
                                .min_size(egui::vec2(140.0, 45.0))
                                .corner_radius(6.0)
                            ).clicked() {
                                self.open_file_dialog();
                            }
                        });
                        
                        // Recent Files Section
                        if !self.settings.recent_files.is_empty() {
                            ui.add_space(40.0);
                            ui.heading(egui::RichText::new("Recent Files").size(18.0));
                            ui.add_space(10.0);
                            
                            egui::Frame::default()
                                .inner_margin(12.0)
                                .corner_radius(8.0)
                                .fill(ui.visuals().extreme_bg_color)
                                .show(ui, |ui| {
                                    for path in self.settings.recent_files.clone().iter().take(5) {
                                        let display_name = std::path::Path::new(path)
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| path.clone());
                                        if ui.add(egui::Button::new(&display_name)
                                            .min_size(egui::vec2(300.0, 30.0))
                                        ).on_hover_text(path).clicked() {
                                            self.load_file(path);
                                        }
                                    }
                                });
                        }
                    });
                });
            }
            AppState::Error(msg) => {
                let mut back_clicked = false;
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Error");
                        ui.label(msg.as_str());
                        if ui.button("Back").clicked() {
                            back_clicked = true;
                        }
                    });
                });
                if back_clicked {
                    next_state = Some(AppState::Welcome);
                }
            }
            AppState::Loading(name) => {
                 egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(format!("Loading {}...", name));
                        ui.spinner();
                    });
                });
            }
            AppState::Editor(state) => {
                render_editor(state, ctx, &self.settings);
            }
        }

        if let Some(s) = next_state {
            self.state = s;
        }
    }
}

fn render_editor(state: &mut EditorState, ctx: &egui::Context, settings: &Settings) {
    // Override font size
    let mut style = (*ctx.style()).clone();
    style.text_styles.iter_mut().for_each(|(_, font_id)| {
        font_id.size = settings.font_size;
    });
    // This is a bit heavy to do every frame, but fine for now. 
    // Ideally we'd set this once or in apply_style if it wasn't varying per-frame potentially.
    // Actually apply_style is better, but here we can scope it to the editor panel if we wanted.
    // Let's execute it on the ui scope.

    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.style_mut().text_styles = style.text_styles.clone(); // Apply font
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("CSVit").strong());
            ui.label(egui::RichText::new(&state.filename).color(egui::Color32::from_gray(150)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                 ui.selectable_value(&mut state.view_mode, ViewMode::Table, "Table");
                 ui.selectable_value(&mut state.view_mode, ViewMode::Text, "Text");
                 ui.selectable_value(&mut state.view_mode, ViewMode::Graph, "Graph");
                 ui.separator();
                 ui.checkbox(&mut state.word_wrap, "Word Wrap");
                 ui.separator();
                 if ui.button("Export JSON").clicked() {
                     if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).save_file() {
                         let input = state.filename.clone();
                         let output = path.to_string_lossy().to_string();
                         std::thread::spawn(move || {
                             let _ = crate::backend::export::export_to_json(&input, &output);
                         });
                     }
                 }
            });
        });
        ui.add_space(4.0);
    });

    // Edit toolbar (only shown when grid mode is active)
    if state.grid.is_some() {
        egui::TopBottomPanel::top("edit_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Edit:");
                if ui.button("➕ Row").clicked() {
                    if let Some(ref mut grid) = state.grid {
                        let after = state.selected_cell.map(|(r, _)| r);
                        grid.add_row(after);
                    }
                }
                if ui.button("➖ Row").clicked() {
                    if let Some(ref mut grid) = state.grid {
                        if let Some((r, _)) = state.selected_cell {
                            grid.delete_row(r);
                            state.selected_cell = None;
                        }
                    }
                }
                ui.separator();
                if ui.button("➕ Col").clicked() {
                    if let Some(ref mut grid) = state.grid {
                        let after = state.selected_cell.map(|(_, c)| c);
                        grid.add_column(after);
                        state.num_columns = grid.num_cols();
                        state.column_widths.push(100.0);
                    }
                }
                if ui.button("➖ Col").clicked() {
                    if let Some(ref mut grid) = state.grid {
                        if let Some((_, c)) = state.selected_cell {
                            grid.delete_column(c);
                            state.num_columns = grid.num_cols();
                            if !state.column_widths.is_empty() {
                                state.column_widths.pop();
                            }
                            state.selected_cell = None;
                        }
                    }
                }
                ui.separator();
                // Undo/Redo buttons
                let can_undo = state.grid.as_ref().map(|g| g.can_undo()).unwrap_or(false);
                let can_redo = state.grid.as_ref().map(|g| g.can_redo()).unwrap_or(false);
                let undo_count = state.grid.as_ref().map(|g| g.undo_count()).unwrap_or(0);
                let redo_count = state.grid.as_ref().map(|g| g.redo_count()).unwrap_or(0);
                
                ui.add_enabled_ui(can_undo, |ui| {
                    if ui.button(format!("↩ Undo ({})", undo_count)).clicked() {
                        if let Some(ref mut grid) = state.grid {
                            grid.undo();
                        }
                    }
                });
                ui.add_enabled_ui(can_redo, |ui| {
                    if ui.button(format!("↪ Redo ({})", redo_count)).clicked() {
                        if let Some(ref mut grid) = state.grid {
                            grid.redo();
                        }
                    }
                });
                ui.separator();
                if ui.button("💾 Save As").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .add_filter("CSVit", &["csvi"])
                        .save_file()
                    {
                        if let Some(ref grid) = state.grid {
                            let csv_text = grid.to_csv();
                            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("csv");
                            if ext == "csvi" {
                                let metadata = crate::backend::csvi::CsviMetadata::new();
                                let _ = crate::backend::csvi::save_csvi(&path, &csv_text, &metadata);
                            } else {
                                let _ = std::fs::write(&path, csv_text);
                            }
                            state.filename = path.to_string_lossy().to_string();
                        }
                    }
                }
            });
        });
    }

    egui::CentralPanel::default().show(ctx, |ui| {
         ui.style_mut().text_styles = style.text_styles.clone(); // Apply font
         
         // Use grid if available, otherwise use loader
         let total_rows = if let Some(ref grid) = state.grid {
             grid.num_rows()
         } else {
             state.loader.total_records()
         };
         let num_cols = state.num_columns;
         let mut scroll_target = None;
         
         // Helper to load content - uses grid if available
         let load_content = |state: &mut EditorState, r: usize, c: usize| -> String {
              if let Some(ref grid) = state.grid {
                  grid.get_cell(r, c).cloned().unwrap_or_default()
              } else {
                  let line_content = match state.reader.get_rows(r, 1) {
                        Ok(v) => v.get(0).cloned().unwrap_or_default(),
                        Err(_) => String::new(),
                  };
                  let fields = CsvParser::parse_line(&line_content).unwrap_or_default();
                  if let Some(edit) = state.editor.get_edit(r, c) {
                      edit.clone()
                  } else {
                      fields.get(c).cloned().unwrap_or_default()
                  }
              }
         };

         // Keyboard Navigation
         if state.editing_cell.is_none() && state.edit_modal.is_none() {
             if let Some((r, c)) = state.selected_cell {
                 if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                     let next_row = (r.min(total_rows - 1) + 1).min(total_rows - 1);
                     state.selected_cell = Some((next_row, c));
                     scroll_target = Some(next_row);
                 } else if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                      let prev_row = r.saturating_sub(1);
                      state.selected_cell = Some((prev_row, c));
                      scroll_target = Some(prev_row);
                 } else if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
                      state.selected_cell = Some((r, (c + 1).min(num_cols - 1)));
                      scroll_target = Some(r);
                 } else if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
                      state.selected_cell = Some((r, c.saturating_sub(1)));
                      scroll_target = Some(r);
                 } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                      if settings.use_edit_modal {
                          let text = load_content(state, r, c);
                          state.edit_modal = Some((r, c, text));
                      } else {
                          state.editing_cell = Some((r, c));
                          state.input_buffer = load_content(state, r, c);
                      }
                 }
             } else {
                 // Initial selection on arrow key
                  if ui.input(|i| i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowLeft)) {
                      state.selected_cell = Some((0, 0));
                      scroll_target = Some(0); // Set scroll_target
                  }
             }
         }
         
         // Undo/Redo keyboard shortcuts (Ctrl+Z / Ctrl+Y)
         if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
             if let Some(ref mut grid) = state.grid {
                 grid.undo();
             }
         }
         if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Y)) {
             if let Some(ref mut grid) = state.grid {
                 grid.redo();
             }
         }

         let row_height = settings.row_height;

         match state.view_mode {
            ViewMode::Table => {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    let mut builder = TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::auto()); // Index
                    
                    for width in &state.column_widths {
                        builder = builder.column(Column::initial(*width).resizable(true));
                    }

                    if let Some(target_row) = scroll_target {
                        builder = builder.scroll_to_row(target_row, Some(egui::Align::Center));
                    }
                    
                    builder
                        .header(30.0, |mut header| {
                            header.col(|ui| { ui.strong("Row"); });
                            for i in 0..state.num_columns {
                                header.col(|ui| { ui.strong(format!("Col {}", i)); });
                            }
                        })
                        .body(|body| {
                            body.rows(row_height, total_rows, |mut row| {
                                let row_index = row.index();
                                
                                // Get fields from grid if available, otherwise from reader
                                let fields: Vec<String> = if let Some(ref grid) = state.grid {
                                    (0..state.num_columns)
                                        .map(|c| grid.get_cell(row_index, c).cloned().unwrap_or_default())
                                        .collect()
                                } else {
                                    let line_content = match state.reader.get_rows(row_index, 1) {
                                        Ok(v) => v.get(0).cloned().unwrap_or_default(),
                                        Err(_) => String::new(),
                                    };
                                    let mut fields = CsvParser::parse_line(&line_content).unwrap_or_default();
                                    while fields.len() < state.num_columns { fields.push(String::new()); }
                                    fields
                                };

                                row.col(|ui| { ui.label(egui::RichText::new(row_index.to_string()).color(egui::Color32::from_gray(100))); });
                                for (col_index, field) in fields.iter().enumerate().take(state.num_columns) {
                                    row.col(|ui| {
                                        let is_editing = state.editing_cell == Some((row_index, col_index));
                                        let is_selected = state.selected_cell == Some((row_index, col_index));
                                        
                                        if is_editing {
                                            let response = ui.text_edit_singleline(&mut state.input_buffer);
                                            if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                                if let Some(ref mut grid) = state.grid {
                                                    grid.set_cell(row_index, col_index, state.input_buffer.clone());
                                                } else {
                                                    let old_value = field.clone();
                                                    state.editor.add_edit(row_index, col_index, old_value, state.input_buffer.clone());
                                                }
                                                state.editing_cell = None;
                                            } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                                state.editing_cell = None;
                                            }
                                            response.request_focus();
                                        } else {
                                             let text = if let Some(edit) = state.editor.get_edit(row_index, col_index) {
                                                edit
                                            } else {
                                                field
                                            };
                                            
                                            // Use placeholder for empty cells to make them clickable
                                            let display_text = if text.is_empty() { " " } else { text };
                                            
                                            // Fill entire available cell space for easy clicking
                                            let available = ui.available_size();
                                            let cell_size = egui::vec2(available.x.max(80.0), row_height - 2.0);
                                            let (rect, response) = ui.allocate_exact_size(cell_size, egui::Sense::click());
                                            
                                            // Draw text within the allocated area
                                            let text_pos = rect.min + egui::vec2(4.0, (rect.height() - settings.font_size) / 2.0);
                                            ui.painter().text(
                                                text_pos,
                                                egui::Align2::LEFT_TOP,
                                                display_text,
                                                egui::FontId::proportional(settings.font_size),
                                                ui.visuals().text_color(),
                                            );
                                            
                                            // Selection Highlight
                                            if is_selected {
                                                ui.painter().rect_stroke(
                                                    response.rect,
                                                    0.0,
                                                    egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255)),
                                                    egui::StrokeKind::Middle
                                                );
                                            }

                                            if response.clicked() {
                                                state.selected_cell = Some((row_index, col_index));
                                            }
                                            
                                            if response.double_clicked() {
                                                if settings.use_edit_modal {
                                                    // Load full content for modal
                                                    // We need to re-read essentially, or copy logic.
                                                    // Since we are inside the closure, we can't easily call `load_content` helper 
                                                    // if it borrows key parts. But we have `text` here!
                                                    state.edit_modal = Some((row_index, col_index, text.clone()));
                                                } else {
                                                    state.editing_cell = Some((row_index, col_index));
                                                    state.input_buffer = text.clone();
                                                }
                                            }
                                            
                                            response.context_menu(|ui| {
                                                 if ui.button("Edit Cell").clicked() {
                                                     // Always allow explicit edit via menu
                                                     if settings.use_edit_modal {
                                                          state.edit_modal = Some((row_index, col_index, text.clone()));
                                                     } else {
                                                          state.editing_cell = Some((row_index, col_index));
                                                          state.input_buffer = text.clone();
                                                     }
                                                     ui.close();
                                                 }
                                                if ui.button("View Row as JSON").clicked() {
                                                    // Collect all fields for this row
                                                    let mut map = serde_json::Map::new();
                                                    for (i, val) in fields.iter().enumerate() {
                                                        // Ideally fetch headers. For now use Col {i}
                                                        map.insert(format!("Col {}", i), serde_json::Value::String(val.clone()));
                                                    }
                                                    let json = serde_json::to_string_pretty(&map).unwrap_or_default();
                                                    state.json_modal = Some((row_index, json));
                                                    ui.close();
                                                }
                                            });
                                        }
                                    });
                                }
                            });
                        });
                });
            }
            ViewMode::Text => {
                 egui::ScrollArea::vertical().show_rows(ui, row_height, total_rows, |ui, row_range| {
                    let len = row_range.end - row_range.start;
                    let rows = state.reader.get_rows(row_range.start, len).unwrap_or_default();
                    
                    for (i, line) in rows.iter().enumerate() {
                        let idx = row_range.start + i;
                        ui.horizontal(|ui| {
                           ui.label(egui::RichText::new(format!("{: >6} |", idx)).color(egui::Color32::from_gray(100)).monospace());
                           ui.monospace(line.trim_end());
                        });
                    }
                });
            }
            ViewMode::Graph => {
                 egui::CentralPanel::default().show(ctx, |ui| {
                     ui.horizontal(|ui| {
                        ui.label("X Axis:");
                        egui::ComboBox::from_id_salt("x_axis")
                            .selected_text(format!("Col {}", state.graph_x_col))
                            .show_ui(ui, |ui| {
                                for i in 0..state.num_columns {
                                    ui.selectable_value(&mut state.graph_x_col, i, format!("Col {}", i));
                                }
                            });
                        
                        ui.label("Y Axis:");
                         egui::ComboBox::from_id_salt("y_axis")
                            .selected_text(format!("Col {}", state.graph_y_col))
                            .show_ui(ui, |ui| {
                                for i in 0..state.num_columns {
                                    ui.selectable_value(&mut state.graph_y_col, i, format!("Col {}", i));
                                }
                            });
                        
                        if ui.button("Regenerate Graph").clicked() {
                            // Fetch data
                            let records = std::cmp::min(state.loader.total_records(), 5000); // Limit to 5000 for perfo
                            let mut data = Vec::with_capacity(records);
                            for i in 0..records {
                                if let Some(line) = state.loader.get_record_line(i) {
                                     // Need to parse quickly without `csv` reader if possible or use helper
                                     // Using CsvParser would be safer
                                    let line_str = String::from_utf8_lossy(line);
                                    let fields = CsvParser::parse_line(&line_str).unwrap_or_default();
                                    
                                    let x_str = fields.get(state.graph_x_col).cloned().unwrap_or_default();
                                    let y_str = fields.get(state.graph_y_col).cloned().unwrap_or_default();
                                    
                                    if let (Ok(x), Ok(y)) = (x_str.parse::<f64>(), y_str.parse::<f64>()) {
                                        data.push([x, y]);
                                    }
                                }
                            }
                            state.graph_data = data;
                        }
                     });
                     
                     egui_plot::Plot::new("csv_plot")
                        .show(ui, |plot_ui| {
                            plot_ui.line(egui_plot::Line::new("Data", egui_plot::PlotPoints::new(state.graph_data.clone())));
                            plot_ui.points(egui_plot::Points::new("Data Points", egui_plot::PlotPoints::new(state.graph_data.clone())).radius(3.0));
                        });
                 });
            }
         }
    });

    // Render Edit Modal
    if let Some((r, c, mut text)) = state.edit_modal.clone() {
        let mut open = true;
        egui::Window::new(format!("Edit Cell ({}, {})", r, c))
            .open(&mut open)
            .resize(|r| r.fixed_size(egui::vec2(400.0, 300.0))) 
            .show(ctx, |ui| {
                ui.add(egui::TextEdit::multiline(&mut text).desired_width(f32::INFINITY).desired_rows(10));
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        // Old value is empty since we don't track it in edit modal
                        state.editor.add_edit(r, c, String::new(), text.clone());
                        state.edit_modal = None;
                    }
                    if ui.button("Cancel").clicked() {
                        state.edit_modal = None;
                    }
                    if ui.button("Beautify JSON").clicked() {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Ok(pretty) = serde_json::to_string_pretty(&value) {
                                text = pretty;
                            }
                        }
                    }
                });
            });
        
        if open {
             // Update the state text if changed (so typing works)
             // But wait, `text` is a local clone. We need to write back to `state.edit_modal`.
             if let Some((_, _, ref mut stored_text)) = state.edit_modal {
                 *stored_text = text;
             }
        } else {
            state.edit_modal = None;
        }
    }

    // Render JSON Modal
    if let Some((idx, json)) = &state.json_modal {
        let mut open = true;
        egui::Window::new(format!("Row {} JSON", idx))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.style_mut().text_styles = style.text_styles.clone();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut json.as_str()).code_editor());
                });
            });
        if !open {
            state.json_modal = None;
        }
    }
}

fn apply_style(ctx: &egui::Context, settings: &Settings) {
    match settings.theme {
        Theme::System => {
            ctx.set_visuals(egui::Visuals::default()); 
        }
        Theme::Dark => {
            let mut visuals = egui::Visuals::dark();
            visuals.window_corner_radius = 8.0.into();
            visuals.panel_fill = egui::Color32::from_rgb(18, 18, 22);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(25, 25, 30);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(35, 35, 42);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(50, 50, 60);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(70, 130, 180);
            visuals.selection.bg_fill = egui::Color32::from_rgb(60, 100, 150);
            visuals.faint_bg_color = egui::Color32::from_rgb(30, 30, 38);
            visuals.extreme_bg_color = egui::Color32::from_rgb(12, 12, 16);
            ctx.set_visuals(visuals);
        }
        Theme::Light => {
            let mut visuals = egui::Visuals::light();
            visuals.window_corner_radius = 8.0.into();
            visuals.panel_fill = egui::Color32::from_rgb(248, 248, 252);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(240, 240, 245);
            visuals.faint_bg_color = egui::Color32::from_rgb(235, 235, 242);
            visuals.selection.bg_fill = egui::Color32::from_rgb(180, 210, 240);
            ctx.set_visuals(visuals);
        }
        Theme::Monokai => {
            let mut visuals = egui::Visuals::dark();
            visuals.window_corner_radius = 8.0.into();
            visuals.panel_fill = egui::Color32::from_rgb(39, 40, 34);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(39, 40, 34);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(49, 50, 44);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(62, 63, 55);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(166, 226, 46);
            visuals.selection.bg_fill = egui::Color32::from_rgb(73, 72, 62);
            visuals.faint_bg_color = egui::Color32::from_rgb(45, 46, 40);
            visuals.extreme_bg_color = egui::Color32::from_rgb(30, 31, 28);
            visuals.override_text_color = Some(egui::Color32::from_rgb(248, 248, 242));
            ctx.set_visuals(visuals);
        }
        Theme::Solarized => {
            let mut visuals = egui::Visuals::dark();
            visuals.window_corner_radius = 8.0.into();
            visuals.panel_fill = egui::Color32::from_rgb(0, 43, 54);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(0, 43, 54);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(7, 54, 66);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(88, 110, 117);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(38, 139, 210);
            visuals.selection.bg_fill = egui::Color32::from_rgb(38, 139, 210);
            visuals.faint_bg_color = egui::Color32::from_rgb(7, 54, 66);
            visuals.extreme_bg_color = egui::Color32::from_rgb(0, 36, 46);
            visuals.override_text_color = Some(egui::Color32::from_rgb(131, 148, 150));
            ctx.set_visuals(visuals);
        }
        Theme::Nord => {
            let mut visuals = egui::Visuals::dark();
            visuals.window_corner_radius = 8.0.into();
            visuals.panel_fill = egui::Color32::from_rgb(46, 52, 64);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(46, 52, 64);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(59, 66, 82);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(67, 76, 94);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(136, 192, 208);
            visuals.selection.bg_fill = egui::Color32::from_rgb(136, 192, 208);
            visuals.faint_bg_color = egui::Color32::from_rgb(59, 66, 82);
            visuals.extreme_bg_color = egui::Color32::from_rgb(36, 42, 54);
            visuals.override_text_color = Some(egui::Color32::from_rgb(236, 239, 244));
            ctx.set_visuals(visuals);
        }
        Theme::Dracula => {
            let mut visuals = egui::Visuals::dark();
            visuals.window_corner_radius = 8.0.into();
            visuals.panel_fill = egui::Color32::from_rgb(40, 42, 54);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(40, 42, 54);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(68, 71, 90);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(98, 101, 120);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(189, 147, 249);
            visuals.selection.bg_fill = egui::Color32::from_rgb(189, 147, 249);
            visuals.faint_bg_color = egui::Color32::from_rgb(55, 57, 70);
            visuals.extreme_bg_color = egui::Color32::from_rgb(33, 34, 44);
            visuals.override_text_color = Some(egui::Color32::from_rgb(248, 248, 242));
            ctx.set_visuals(visuals);
        }
        Theme::Catppuccin => {
            let mut visuals = egui::Visuals::dark();
            visuals.window_corner_radius = 8.0.into();
            visuals.panel_fill = egui::Color32::from_rgb(30, 30, 46);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 30, 46);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(49, 50, 68);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(69, 71, 90);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(203, 166, 247);
            visuals.selection.bg_fill = egui::Color32::from_rgb(203, 166, 247);
            visuals.faint_bg_color = egui::Color32::from_rgb(45, 45, 60);
            visuals.extreme_bg_color = egui::Color32::from_rgb(24, 24, 37);
            visuals.override_text_color = Some(egui::Color32::from_rgb(205, 214, 244));
            ctx.set_visuals(visuals);
        }
        Theme::Custom(idx) => {
            if let Some(custom) = settings.custom_themes.get(idx) {
                let mut visuals = egui::Visuals::dark();
                visuals.window_corner_radius = 8.0.into();
                visuals.panel_fill = egui::Color32::from_rgb(custom.bg_primary[0], custom.bg_primary[1], custom.bg_primary[2]);
                visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(custom.bg_primary[0], custom.bg_primary[1], custom.bg_primary[2]);
                visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(custom.bg_secondary[0], custom.bg_secondary[1], custom.bg_secondary[2]);
                visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(custom.selection[0], custom.selection[1], custom.selection[2]);
                visuals.widgets.active.bg_fill = egui::Color32::from_rgb(custom.accent[0], custom.accent[1], custom.accent[2]);
                visuals.selection.bg_fill = egui::Color32::from_rgb(custom.accent[0], custom.accent[1], custom.accent[2]);
                visuals.faint_bg_color = egui::Color32::from_rgb(
                    custom.stripe.map(|s| s[0]).unwrap_or(custom.bg_secondary[0]),
                    custom.stripe.map(|s| s[1]).unwrap_or(custom.bg_secondary[1]),
                    custom.stripe.map(|s| s[2]).unwrap_or(custom.bg_secondary[2]),
                );
                visuals.extreme_bg_color = egui::Color32::from_rgb(custom.bg_secondary[0], custom.bg_secondary[1], custom.bg_secondary[2]);
                visuals.override_text_color = Some(egui::Color32::from_rgb(custom.text_primary[0], custom.text_primary[1], custom.text_primary[2]));
                ctx.set_visuals(visuals);
            } else {
                ctx.set_visuals(egui::Visuals::dark());
            }
        }
    }
}

