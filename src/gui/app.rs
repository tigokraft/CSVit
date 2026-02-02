use std::sync::Arc;
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use crate::backend::loader::CsvLoader;
use crate::backend::paged_reader::PagedReader;
use crate::backend::editor::EditBuffer;
use crate::backend::parser::CsvParser;



#[derive(PartialEq, Clone, Copy)]
pub enum Theme {
    System,
    Dark,
    Light,
}

#[derive(Clone)]
pub struct Settings {
    pub theme: Theme,
    pub font_size: f32,
    pub row_height: f32,
    pub use_edit_modal: bool,
}

#[derive(PartialEq)]
pub enum ViewMode {
    Table,
    Text,
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
}

impl GuiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, loader: Option<Arc<CsvLoader>>, filename: Option<String>) -> Self {
        let state = if let (Some(loader), Some(name)) = (loader, filename) {
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
            })
        } else {
            AppState::Welcome
        };

        // TODO: Configure fonts/styles for Shadcn look
        // We'll do this in update or a separate setup function if needed.
        
        Self { 
            state,
            settings: Settings { 
                theme: Theme::System,
                font_size: 14.0,
                row_height: 24.0,
                use_edit_modal: false,
            },
            show_settings: false,
        }
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("CSV", &["csv"]).pick_file() {
            let path_str = path.to_string_lossy().to_string();
            self.state = AppState::Loading(path_str.clone());
            
            // In a real app we'd spawn a thread. For now, block to load (it's fast due to mmap).
            // Actually, let's just load it here.
            match CsvLoader::new(&path) {
                Ok(loader) => {
                    let arc_loader = Arc::new(loader);
                    self.state = AppState::Editor(EditorState {
                        loader: arc_loader.clone(),
                        reader: PagedReader::new(arc_loader.clone()),
                        editor: EditBuffer::new(),
                        view_mode: ViewMode::Table,
                        input_buffer: String::new(),
                        editing_cell: None,
                        filename: path_str,
                        word_wrap: false,
                        json_modal: None,
                        num_columns: arc_loader.num_columns(),
                        column_widths: arc_loader.estimate_column_widths(),
                        selected_cell: None,
                        edit_modal: None,
                    });
                }
                Err(e) => {
                    self.state = AppState::Error(format!("Failed to load file: {}", e));
                }
            }
        }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_style(ctx, &self.settings); 

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
             ui.horizontal(|ui| {
                 ui.menu_button("File", |ui| {
                     if ui.button("Open").clicked() {
                         self.open_file_dialog();
                         ui.close_menu();
                     }
                 });
                 if ui.button("Settings").clicked() {
                     self.show_settings = true;
                 }
             });
        });

        // Settings Window
        if self.show_settings {
             let mut open = true;
             egui::Window::new("Settings")
                .open(&mut open)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Theme");
                    egui::ComboBox::from_id_salt("theme_selector")
                        .selected_text(match self.settings.theme {
                            Theme::System => "System",
                            Theme::Dark => "Dark",
                            Theme::Light => "Light",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.settings.theme, Theme::System, "System");
                            ui.selectable_value(&mut self.settings.theme, Theme::Dark, "Dark");
                            ui.selectable_value(&mut self.settings.theme, Theme::Light, "Light");
                        });
                });
             if !open {
                 self.show_settings = false;
             }
        }

        // Handle Drag & Drop
        if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
            let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
            if let Some(file) = dropped_files.first() {
                if let Some(path) = &file.path {
                     let path_str = path.to_string_lossy().to_string();
                     self.state = AppState::Loading(path_str.clone());
                     match CsvLoader::new(path) {
                        Ok(loader) => {
                            let arc_loader = Arc::new(loader);
                            self.state = AppState::Editor(EditorState {
                                loader: arc_loader.clone(),
                                reader: PagedReader::new(arc_loader.clone()),
                                editor: EditBuffer::new(),
                                view_mode: ViewMode::Table,
                                input_buffer: String::new(),
                                editing_cell: None,
                                filename: path_str,
                                word_wrap: false,
                                json_modal: None,
                                num_columns: arc_loader.num_columns(),
                                column_widths: arc_loader.estimate_column_widths(),
                                selected_cell: None,
                                edit_modal: None,
                            });
                        }
                        Err(e) => {
                            self.state = AppState::Error(format!("Failed to load file: {}", e));
                        }
                    }
                }
            }
        }

        let mut next_state = None;

        match &mut self.state {
            AppState::Welcome => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.heading(egui::RichText::new("CSVit").size(40.0).strong());
                        ui.label(egui::RichText::new("High performance editor for large files").size(16.0).color(egui::Color32::from_gray(150)));
                        ui.add_space(40.0);
                        
                        if ui.add(egui::Button::new(egui::RichText::new("Open File").size(18.0))
                            .min_size(egui::vec2(200.0, 50.0))
                            .corner_radius(4.0)
                        ).clicked() {
                            self.open_file_dialog();
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
                render_editor(state, ctx);
            }
        }

        if let Some(s) = next_state {
            self.state = s;
        }
    }
}

fn render_editor(state: &mut EditorState, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("CSVit").strong());
            ui.label(egui::RichText::new(&state.filename).color(egui::Color32::from_gray(150)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                 ui.selectable_value(&mut state.view_mode, ViewMode::Table, "Table");
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

    egui::CentralPanel::default().show(ctx, |ui| {
         let total_rows = state.loader.total_records();
         let num_cols = state.num_columns;
         let mut scroll_target = None;
         
         // Keyboard Navigation
         if state.editing_cell.is_none() {
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
                      state.editing_cell = Some((r, c));
                      // Load content into buffer
                      let line_content = match state.reader.get_rows(r, 1) {
                            Ok(v) => v.get(0).cloned().unwrap_or_default(),
                            Err(_) => String::new(),
                      };
                      let fields = CsvParser::parse_line(&line_content).unwrap_or_default();
                      let text = if let Some(edit) = state.editor.get_edit(r, c) {
                          edit.clone()
                      } else {
                          fields.get(c).cloned().unwrap_or_default()
                      };
                      state.input_buffer = text;
                 }
             } else {
                 // Initial selection on arrow key
                  if ui.input(|i| i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowLeft)) {
                      state.selected_cell = Some((0, 0));
                      scroll_target = Some(0); // Set scroll_target
                  }
             }
         }

         let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
         // Increase row height if wrapping? No, virtual lists usually need fixed height or estimation.
         // For now, let's keep fixed height but allow internal wrapping if space per cell permits?
         // Actually, if we wrap, row height varies. `TableBuilder` supports `rows` with fixed height.
         // We might need `body.heterogeneous_rows` if we really want variable height, but that's expensive to calc.
         // Let's stick to fixed height for now, but allow wrapping within that height (e.g. 2 lines?). 
         // Or just basic wrapping.
         let row_height = if state.word_wrap { text_height * 2.0 + 12.0 } else { text_height + 12.0 };

         match state.view_mode {
            ViewMode::Table => {
                // TableBuilder inside ScrollArea for horizontal scrolling if needed?
                // Actually TableBuilder has `.scroll()`.
                
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
                                let line_content = match state.reader.get_rows(row_index, 1) {
                                    Ok(v) => v.get(0).cloned().unwrap_or_default(),
                                    Err(_) => String::new(),
                                };
                                let mut fields = CsvParser::parse_line(&line_content).unwrap_or_default();
                                while fields.len() < state.num_columns { fields.push(String::new()); }

                                row.col(|ui| { ui.label(egui::RichText::new(row_index.to_string()).color(egui::Color32::from_gray(100))); });
                                for (col_index, field) in fields.iter().enumerate().take(state.num_columns) {
                                    row.col(|ui| {
                                        let is_editing = state.editing_cell == Some((row_index, col_index));
                                        let is_selected = state.selected_cell == Some((row_index, col_index));
                                        
                                        if is_editing {
                                            let response = ui.text_edit_singleline(&mut state.input_buffer);
                                            if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                                state.editor.add_edit(row_index, col_index, state.input_buffer.clone());
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
                                            
                                            let mut label = egui::Label::new(text).sense(egui::Sense::click());
                                            if state.word_wrap {
                                                label = label.wrap();
                                            } else {
                                                label = label.truncate();
                                            }

                                            let response = ui.add(label);
                                            
                                            // Selection Highlight
                                            if is_selected {
                                                ui.painter().rect_stroke(
                                                    response.rect,
                                                    0.0,
                                                    egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255)),
                                                    egui::StrokeKind::Middle
                                                );
                                                // Removed scroll_to_me to prevent glitching. 
                                                // Scroll is handled by TableBuilder via scroll_target.
                                            }

                                            if response.clicked() {
                                                state.selected_cell = Some((row_index, col_index));
                                            }

                                            if response.double_clicked() {
                                                state.editing_cell = Some((row_index, col_index));
                                                state.input_buffer = text.clone();
                                            }
                                            
                                            response.context_menu(|ui| {
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
         }
    });

    // Render Modal
    if let Some((idx, json)) = &state.json_modal {
        let mut open = true;
        egui::Window::new(format!("Row {} JSON", idx))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
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
            // Respect system, so don't force visuals unless you want to override some specifics
            // Reset to default then we can tweak
             ctx.set_visuals(egui::Visuals::default()); 
        }
        Theme::Dark => {
            let mut visuals = egui::Visuals::dark();
            visuals.window_corner_radius = 8.0.into();
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(20, 20, 25); 
            ctx.set_visuals(visuals);
        }
        Theme::Light => {
             ctx.set_visuals(egui::Visuals::light());
        }
    }
}
