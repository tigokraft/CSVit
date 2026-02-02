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
                graph_x_col: 0,
                graph_y_col: 1,
                graph_data: Vec::new(),
            })
        } else {
            AppState::Welcome
        };

        // TODO: Configure fonts/styles for Shadcn look
        // We'll do this in update or a separate setup function if needed.
        
        Self { 
            state,
        Self { 
            state,
            settings: Settings::load(),
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
                        graph_x_col: 0,
                        graph_y_col: 1,
                        graph_data: Vec::new(),
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
                    
                    ui.separator();
                    ui.label("Appearance");
                    ui.add(egui::Slider::new(&mut self.settings.font_size, 10.0..=24.0).text("Font Size"));
                    ui.add(egui::Slider::new(&mut self.settings.row_height, 20.0..=60.0).text("Row Height"));
                    
                    ui.separator();
                    ui.label("Behavior");
                    ui.checkbox(&mut self.settings.use_edit_modal, "Use Popup for Editing");
                    
                    ui.separator();
                    ui.horizontal(|ui| {
                         if ui.button("Save Settings").clicked() {
                             self.settings.save();
                         }
                         if ui.button("Delete Config").clicked() {
                             Settings::reset();
                             self.settings = Settings::load(); // Reload defaults
                         }
                    });
                });
             if !open {
                 self.show_settings = false;
                 self.settings.save(); // Auto-save on close
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
                                graph_x_col: 0,
                                graph_y_col: 1,
                                graph_data: Vec::new(),
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

    egui::CentralPanel::default().show(ctx, |ui| {
         ui.style_mut().text_styles = style.text_styles.clone(); // Apply font
         let total_rows = state.loader.total_records();
         let num_cols = state.num_columns;
         let mut scroll_target = None;
         
         // Helper to load content
         let load_content = |state: &mut EditorState, r: usize, c: usize| -> String {
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
                            plot_ui.line(egui_plot::Line::new(egui_plot::PlotPoints::new(state.graph_data.clone())));
                            plot_ui.points(egui_plot::Points::new(egui_plot::PlotPoints::new(state.graph_data.clone())).radius(3.0));
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
                        state.editor.add_edit(r, c, text.clone());
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
