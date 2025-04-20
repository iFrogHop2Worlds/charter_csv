use eframe::App;
use egui::{Ui, Button, CentralPanel, Color32, Context, IconData, Image, RichText, ScrollArea, TextEdit, TextureHandle, Vec2, Window, Frame, Margin, Id, FontId, Order, Stroke};
use crate::charter_utilities::{csv2grid, grid2csv, CsvGrid, format_graph_query, save_window_as_png, check_for_screenshot, DraggableLabel, GridLayout, grid_search, SearchResult, render_db_stats};
use crate::session::{load_sessions_from_directory, reconstruct_session, save_session, Session};
use crate::charter_graphs::{draw_bar_graph, draw_flame_graph, draw_histogram, draw_line_chart, draw_pie_chart, draw_scatter_plot};
use crate::csvqb::{process_csvqb_pipeline, Value};
pub use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use image::{ImageReader};
use std::collections::HashMap;
use std::time::Instant;
use itertools::Itertools;
use crate::db_manager::{DatabaseConfig, DatabaseSource, DatabaseType, DbManager};

pub struct CharterCsvApp {
    db_manager: Option<DbManager>,
    db_config: DatabaseConfig,
    texture: Option<TextureHandle>,
    screen: Screen,
    csv_files: Vec<(String, CsvGrid)>,
    grid_layout: Option<GridLayout>,
    csvqb_pipelines: Vec<Vec<(usize, Vec<String>)>>,
    multi_pipeline_tracker: HashMap<usize, Vec<usize>>,
    graph_data: Vec<Vec<Value>>,
    file_receiver: Receiver<(String, Vec<Vec<String>>)>,
    file_sender: Sender<(String, Vec<Vec<String>>)>,
    chart_style_prototype: String,
    sessions: Vec<Session>,
    current_session: i8,
    prev_session: i8,
    show_ss_name_popup: bool,
    edit_ss_name: String,
    time_to_hide_state: Option<Instant>,
    labels: Vec<DraggableLabel>,
    next_label_id: u32,
    show_labels: bool,
    chart_view_editing: bool,
    search_text: String,
    additional_searches: Vec<SearchResult>,
    dark_mode_enabled: bool
}

pub enum Screen {
    Main,
    ViewCsv,
    CreateCsv { content: (String, CsvGrid) },
    EditCsv { index: usize, content: (String, CsvGrid) },
    CreateChart,
    ViewChart,
    Settings
}

#[derive(Debug)]
pub struct PlotPoint {
    pub(crate) label: String,
    pub(crate) value: f64,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) depth: f32
}

impl Default for CharterCsvApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let app = Self {
            db_manager: None,
            db_config: Default::default(),
            texture: None,
            screen: Screen::Main,
            csv_files: vec![],
            grid_layout: None,
            csvqb_pipelines: vec![],
            multi_pipeline_tracker: HashMap::new(),
            graph_data: vec![],
            file_receiver: rx,
            file_sender: tx,
            chart_style_prototype: "Histogram".to_string(),
            sessions: vec![],
            current_session: -1,
            prev_session: -2,
            show_ss_name_popup: false,
            edit_ss_name: "".to_string(),
            time_to_hide_state: None,
            labels: Vec::new(),
            next_label_id: 0,
            show_labels: true,
            chart_view_editing: false,
            search_text: "".to_string(),
            additional_searches: vec![],
            dark_mode_enabled: false,
        };
        match ImageReader::open("src/sailboat.png") {
            Ok(image_reader) => {
                match image_reader.decode() {
                    Ok(image) => {
                        let image_buffer = image.to_rgba8();
                        let pixels = image_buffer.into_raw();
                        let size = [image.width(), image.height()];
                        let icon_data = IconData {
                            rgba: pixels,
                            width: size[0],
                            height: size[1],
                        };
                        Some(icon_data);
                    }
                    Err(e) => {
                        eprintln!("Failed to decode app icon: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to open app icon file: {}", e);
            }
        }
        app
    }
}

impl App for CharterCsvApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        check_for_screenshot(ctx);
        if let Ok((path, grid)) = self.file_receiver.try_recv() {
            self.csv_files.push((path, grid));
        }

        let screen = std::mem::replace(&mut self.screen, Screen::Main);
        match screen {
            Screen::Main => {
                self.screen = screen;
                self.show_main_screen(ctx)
            }
            Screen::ViewCsv => {
                self.screen = screen;
                self.show_csv_list(ctx)
            }
            Screen::CreateCsv { content } => {
                let mut content_owned = content;
                let next_screen = self.show_csv_editor(ctx, &mut content_owned, None);
                self.screen = match next_screen {
                    Some(screen) => screen,
                    None => Screen::CreateCsv { content: content_owned },
                };
            }
            Screen::EditCsv { index, content } => {
                let mut content_owned = content;
                let next_screen = self.show_csv_editor(ctx, &mut content_owned, Some(index));
                self.screen = match next_screen {
                    Some(screen) => screen,
                    None => Screen::EditCsv {
                        index,
                        content: content_owned,
                    },
                };
            }
            Screen::CreateChart => {
                self.screen = screen;
                self.create_chart_screen(ctx);
            }
            Screen::ViewChart => {
                self.screen = screen;
                self.show_chart_screen(ctx)
            }
            Screen::Settings => {
                self.screen = screen;
                self.render_settings(ctx)
            }
        }

        self.sessions = load_sessions_from_directory().expect("Failed to restore sessions");

        if self.current_session != self.prev_session && !self.sessions.is_empty() {
            if self.current_session == -1 {
                let receiver = reconstruct_session(self.sessions[0].clone());
                while let Ok((file_path, grid )) = receiver.recv() {
                    self.csv_files.push((file_path, grid));
                }
            }

            let ssi = self.current_session as usize;
            if ssi >= 0 && ssi < self.sessions.len() {
                for file in &self.sessions[ssi].selected_files {
                    self.multi_pipeline_tracker.insert(*file, vec![*file]);
                }

            }
            let selected_files = self.multi_pipeline_tracker.keys().copied().sorted().collect();
            for (_index, pipelines) in self.csvqb_pipelines.iter().enumerate() {
                for (_, fields) in pipelines.iter() {
                    let result = process_csvqb_pipeline(fields, &selected_files, &self.csv_files);
                    if !result.is_empty() {
                        self.graph_data.push(result);

                    }
                }
            }
            self.prev_session = self.current_session;
        }

    }
}
impl CharterCsvApp {
    fn show_main_screen(&mut self, ctx: &Context) {
        let frame = Frame::default()
            .fill(Color32::from_rgb(193, 200, 208));

        CentralPanel::default().frame(frame).show(ctx, |ui| {
            let texture: &mut TextureHandle = self.texture.get_or_insert_with(|| {
                match ImageReader::open("src/sailboat.png") {
                    Ok(img) => {
                        match img.decode() {
                            Ok(image) => {
                                let image_buffer = image.to_rgba8();
                                let size = [image_buffer.width() as _, image_buffer.height() as _];
                                let pixels = image_buffer.as_raw();
                                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                    size,
                                    pixels,
                                );

                                ctx.load_texture(
                                    "sailboat",
                                    color_image,
                                    egui::TextureOptions::default(),
                                )
                            }
                            Err(e) => {
                                eprintln!("Failed to decode image: {:?}", e);
                                let color_image = egui::ColorImage::new([16, 16], Color32::RED);
                                ctx.load_texture(
                                    "error_placeholder",
                                    color_image,
                                    egui::TextureOptions::default(),
                                )
                            }
                        }
                    }
                    _ => {
                        let color_image = egui::ColorImage::new([16, 16], Color32::RED);
                        ctx.load_texture(
                            "error_placeholder",
                            color_image,
                            egui::TextureOptions::default(),
                        )
                    }
                }
            });

            let top_margin: f32 = 25.0;
            ui.add_space(top_margin.max(0.0));

            ui.horizontal(|ui| {
                Frame::NONE
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE)
                    .inner_margin(Margin {
                        left: 60.0 as i8,
                        right: 10.0 as i8,
                        top: 0.0 as i8,
                        bottom: 5.0 as i8,
                    })
                    .show(ui, |ui| {
                        if ui.button("Load File").clicked() {
                            if let Some(path) = rfd::FileDialog::new().add_filter("CSV files", &["csv"]).pick_file() {
                                let path_as_string = path.to_str().unwrap().to_string();
                                let sender = self.file_sender.clone();
                                thread::spawn(move || {
                                    if let Ok(content) = std::fs::read_to_string(&path) {
                                        let grid: CsvGrid = csv2grid(&content);
                                        let _ = sender.send((path_as_string, grid));
                                    }
                                });
                            }
                        }

                        if ui.button("New File").clicked() {
                            self.screen = Screen::CreateCsv {
                                content: (
                                    "/todo/set path".to_string(),
                                    vec![vec!["".to_string()]],
                                ),
                            };
                        }

                        if ui.button("View Files").clicked() {
                            self.screen = Screen::ViewCsv;
                        }

                        if ui.button("Data Explorer").clicked() {
                            self.screen = Screen::CreateChart;
                        }

                        if ui.button("View Charts").clicked() {
                            self.screen = Screen::ViewChart;
                        }

                        if self.show_ss_name_popup {
                            Window::new("Enter Session Name")
                                .collapsible(false)
                                .resizable(false)
                                .show(ctx, |ui| {
                                    ui.text_edit_singleline(&mut self.edit_ss_name);
                                    ui.horizontal(|ui| {
                                        if ui.button("OK").clicked() {
                                            save_session(self.edit_ss_name.to_owned(), vec![], vec![], vec![]).expect("session save failed");
                                            self.edit_ss_name.clear();
                                            self.show_ss_name_popup = false;
                                        }
                                        if ui.button("Cancel").clicked() {
                                            self.show_ss_name_popup = false;
                                        }
                                    });
                                });
                        }
                        ui.add_space(ui.available_width() - 50.0);
                        if ui.add(Button::new(RichText::new("⚙").size(25.0))
                            .fill(Color32::TRANSPARENT))
                            .clicked()
                        {
                            self.screen = Screen::Settings;
                        }
                    });
            });

            ui.vertical_centered(|ui| {
                if !self.sessions.is_empty() {
                    ui.add_space(100.0);
                    ui.heading(RichText::new("sessions").color(Color32::BLACK));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 2.0 - 83.0);
                        if ui.button("New Session").clicked() {
                            self.show_ss_name_popup = true;
                        }

                        if ui.button("Save Current").clicked() {
                            let mut file_paths: Vec<String> = vec![];
                            let mut pipelines: Vec<String> = vec![];
                            for (path, _) in self.csv_files.iter() {
                                file_paths.push(path.to_string());
                            }
                            for (_index, pipeline) in self.csvqb_pipelines.iter().enumerate() {
                                for (index, query_string) in pipeline.iter() {
                                    let pipeline_str = index.to_string() + &*" ".to_string() + &*query_string.join(" ");
                                    pipelines.push(pipeline_str);
                                }
                            }
                            let ssi = self.current_session as usize;
                            let selected_files = self.multi_pipeline_tracker.keys().copied().sorted().collect();
                            save_session(self.sessions[ssi].name.to_string(), file_paths, pipelines, selected_files).expect("session save failed");
                        }
                    });
                } else {
                    ui.set_height(-160.0);
                    ui.add_space(20.0);
                    ui.add(
                        Image::new(&*texture)
                            .max_width(200.0)
                    );
                    ui.add_space(20.0);
                    ui.heading(RichText::new("Welcome to Charter CSV!").color(Color32::BLACK));
                    ui.add_space(40.0);
                    ui.label(RichText::new("Create a new session to get started.").color(Color32::BLACK));
                    ui.add_space(20.0);
                    ui.label(RichText::new("You can use the app without a session but it is best to create a session to work in.").color(Color32::BLACK));
                    ui.add_space(20.0);
                    ui.label(RichText::new("You can switch between sessions or save it for later.").color(Color32::BLACK));
                    ui.add_space(20.0);
                    ui.label(RichText::new("The Data Explorer is where you can quickly search and compare the data in your files.").color(Color32::BLACK));
                    ui.add_space(20.0);
                    ui.label(RichText::new("You can create new csv files by clicking New File or you can edit files in the View Files screen.").color(Color32::BLACK));
                    ui.add_space(20.0);
                    ui.label(RichText::new("Have fun exploring the depths of your data!.").color(Color32::BLACK));
                }

                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for (index, session) in self.sessions.iter().enumerate() {
                                let active_session = if self.current_session == index as i8 {
                                    Color32::from_rgb(0, 196, 218)
                                } else {
                                    Color32::TRANSPARENT
                                };
                                ui.push_id(index, |ui| {
                                    ui.group(|ui| {
                                        let _ = ui.group(|ui| {
                                            ui.set_width(ui.available_width() / 1.8);
                                            ui.set_height(30.0);
                                            Frame::default()
                                                .fill(active_session)
                                                .outer_margin(0.0)
                                                .inner_margin(5.0)
                                                .show(ui, |ui| {
                                                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                                                        ui.add_space(3.0);
                                                        ui.label(RichText::new(format!("name: {}", session.name)).color(Color32::BLACK));
                                                        ui.label(RichText::new(format!("files: {:?}", session.files.len())).color(Color32::BLACK));
                                                        ui.label(RichText::new(format!("pipelines: {:?}", session.pipelines.len())).color(Color32::BLACK));

                                                        if self.current_session != index as i8 {
                                                            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                                                if ui.add_sized(
                                                                    Vec2::new(180.0, 20.0),
                                                                    Button::new("Load")
                                                                        .corner_radius(12.0)
                                                                ).clicked() {
                                                                    self.current_session = index as i8;
                                                                    self.multi_pipeline_tracker.clear();
                                                                    self.csv_files.clear();
                                                                    self.csvqb_pipelines.clear();
                                                                    self.graph_data.clear();
                                                                    let receiver = reconstruct_session(self.sessions[index].clone());
                                                                    while let Ok((file_path, grid)) = receiver.recv() {
                                                                        self.csv_files.push((file_path, grid));
                                                                    }
                                                                    for (_index, pipeline) in self.sessions[index].pipelines.iter().enumerate() {
                                                                        if pipeline.is_empty() {
                                                                            println!("Warning: Empty pipeline found, skipping...");
                                                                            continue;
                                                                        }
                                                                        self.csvqb_pipelines.push(vec![(_index, pipeline.to_owned())]);
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    });
                                                });
                                        });
                                    });
                                });
                            }
                        });
                });
            });
        });
    }

    fn show_csv_list(&mut self, ctx: &Context) {
        let frame = Frame::default()
            .fill(Color32::from_rgb(193, 200, 208));

        let mut files_to_remove: Option<usize> = None;
        let mut next_screen: Option<Screen> = None;

        CentralPanel::default().frame(frame).show(ctx, |ui| {
            Frame::NONE
                .fill(Color32::from_rgb(193, 200, 208))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        if ui.button("Home").clicked() {
                            next_screen = Some(Screen::Main);
                        }

                        if ui.button("Load File").clicked() {
                            if let Some(path) = rfd::FileDialog::new().add_filter("CSV files", &["csv"]).pick_file() {
                                let path_as_string = path.to_str().unwrap().to_string();
                                let sender = self.file_sender.clone();
                                thread::spawn(move || {
                                    if let Ok(content) = std::fs::read_to_string(&path) {
                                        let grid: CsvGrid = csv2grid(&content);
                                        let _ = sender.send((path_as_string, grid));
                                    }
                                });
                            }
                        }
                        ui.add_space(ui.available_width());
                    })
                });

            ui.add_space(21.0);

            let desired_width = ui.available_width() / 3.0;
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                for (index, file) in self.csv_files.iter().enumerate() {
                    let file_name = file.0.split("\\").last().unwrap_or("No file name");
                    ui.push_id(index, |ui| {
                        let total_width = ui.available_width();
                        let padding = (total_width - desired_width) / 2.0;
                        ui.allocate_space(Vec2::new(padding, 0.0));

                        ui.group(|ui| {
                            ui.set_max_width(desired_width);
                            ui.horizontal(|ui| {
                                ui.label(file_name);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("remove").clicked() {
                                        files_to_remove = Some(index);
                                    }
                                    if ui.button("edit").clicked() {
                                        next_screen = Some(Screen::EditCsv {
                                            index,
                                            content: file.clone(),
                                        });
                                    }
                                });
                            });
                        });
                    });
                }
            });
        });

        if let Some(index) = files_to_remove {
            self.csv_files.remove(index);
        }
        if let Some(screen) = next_screen {
            self.screen = screen;
        }
    }

    fn show_csv_editor(
        &mut self,
        ctx: &Context,
        content: &mut (String, CsvGrid),
        edit_index: Option<usize>,
    ) -> Option<Screen> {
        let frame = Frame::default()
            .fill(Color32::from_rgb(193, 200, 208));
        let mut next_screen = None;
        CentralPanel::default().frame(frame).show(ctx, |ui| {
            Frame::NONE
                .fill(Color32::from_rgb(193, 200, 208))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        if ui.button("Home").clicked() {
                            next_screen = Some(Screen::Main);
                        }
                        if ui.button("Save File").clicked() {
                            if let Some(index) = edit_index {
                                self.csv_files[index] = content.clone();
                            } else {
                                self.csv_files.push(content.clone());
                            }

                            if let Some(path) = rfd::FileDialog::new().add_filter(&content.0, &["csv"]).save_file() {
                                let csv_content = grid2csv(&content.1);
                                std::fs::write(path, csv_content).expect("Failed to save the file");
                            }

                            next_screen = Some(Screen::ViewCsv);
                        }

                        if ui.button("Add Row").clicked() {
                            self.grid_layout.as_mut().expect("no grid layout found").add_row(&mut content.1);
                        }

                        if ui.button("Add Column").clicked() {
                            self.grid_layout.as_mut().expect("no grid layout found").add_column(&mut content.1);
                        }

                        ui.add(TextEdit::singleline(&mut self.search_text).desired_width(100.0));

                        let search_rect = {
                            let mut rect = None;
                            ui.push_id("search_button", |ui| {
                                let response = ui.button("Search");
                                rect = Some(response.rect);
                                if response.clicked() {
                                    if let Some(grid_layout) = self.grid_layout.as_mut() {
                                        let res = grid_search(&grid_layout, &content.1, &self.search_text);
                                        if let Some(res) = res {
                                            grid_layout.goto_grid_pos(ui, res.0.row, res.0.col, res.0.scroll_x, res.0.scroll_y);
                                            self.additional_searches = res.1;
                                        }
                                    }
                                }
                            });
                            rect.unwrap()
                        };

                        if !self.additional_searches.is_empty() {
                            let window_pos = egui::pos2(
                                search_rect.min.x + 60.0,
                                search_rect.min.y,
                            );

                            let mut style = (*ui.ctx().style()).clone();
                            style.text_styles.insert(
                                egui::TextStyle::Heading,
                                FontId::new(12.0, egui::FontFamily::Proportional)
                            );
                            style.spacing.window_margin = Margin {
                                top: 0.0 as i8,
                                bottom: 0.0 as i8,
                                left: 0.0 as i8,
                                right: 0.0 as i8,
                            };

                            ui.ctx().set_style(style);

                            Window::new("Results")
                                .fixed_pos(window_pos)
                                .default_open(true)
                                .resizable(false)
                                .frame(Frame::NONE)
                                .show(ui.ctx(), |ui| {
                                    ScrollArea::vertical()
                                        .min_scrolled_height(ui.available_height())
                                        .max_height(ui.available_height())
                                        .max_width(ui.available_width())
                                        .show(ui, |ui| {
                                            for search_result in &self.additional_searches {
                                                if ui.button(RichText::new(format!("{:?}", search_result)).color(Color32::from_rgba_unmultiplied(0, 0, 0, 100)))
                                                    .clicked() {
                                                    if let Some(grid_layout) = self.grid_layout.as_mut() {
                                                        grid_layout.goto_grid_pos(ui, search_result.row, search_result.col, search_result.scroll_x, search_result.scroll_y);
                                                    };
                                                }
                                            }
                                        })
                                });
                        }

                        ui.add_space(ui.available_width());
                    })
                });

            if self.grid_layout.is_none() {
                let grid = &mut content.1;
                self.grid_layout = Some(GridLayout::new(grid[0].len(), grid.len()));
            }

            if let Some(grid_layout) = &mut self.grid_layout {
                let grid = &mut content.1;
                grid_layout.show(ui, grid);
            }
        });

        next_screen
    }

    fn create_chart_screen(&mut self, ctx: &Context) {
        let frame = Frame::default()
            .fill(Color32::from_rgb(193, 200, 208));

        CentralPanel::default().frame(frame).show(ctx, |ui| {
            Frame::NONE
                .fill(Color32::from_rgb(193, 200, 208))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        if ui.button("Home").clicked() {
                            self.screen = Screen::Main;
                        }

                        ui.add_space(ui.available_width() / 8.0);
                        egui::ComboBox::from_label("Select File")
                            .show_ui(ui, |ui| {
                                for (index, file) in self.csv_files.iter().enumerate() {
                                    let file_name = &file.0;
                                    let mut selected = self.multi_pipeline_tracker.contains_key(&index);

                                    if ui.checkbox(&mut selected, file_name).clicked() {
                                        if selected {
                                            self.multi_pipeline_tracker.insert(index, vec![index]);
                                            self.csvqb_pipelines.push(vec![(index, vec![])]);
                                        } else {
                                            self.multi_pipeline_tracker.remove(&index);
                                            self.csvqb_pipelines.remove(index);
                                        }
                                    }
                                }
                            });
                        ui.add_space(8.0);
                        if ui.button("clear all").clicked() {
                            self.csvqb_pipelines.clear();
                            self.multi_pipeline_tracker.clear();
                            self.graph_data.clear();
                            if !self.sessions[self.current_session as usize].pipelines.is_empty() {
                                self.sessions[self.current_session as usize].pipelines.clear();
                            }
                        }

                        if ui.button("Run all").clicked() {
                            self.graph_data.clear();

                            let selected_files = &self.multi_pipeline_tracker.keys().copied().collect::<Vec<usize>>();
                            for (root, indexes) in self.multi_pipeline_tracker.iter() {
                                for (i, _) in indexes.iter().enumerate() {
                                    let result = process_csvqb_pipeline(
                                        &*self.csvqb_pipelines[*root][i].1,
                                        selected_files,
                                        &self.csv_files,
                                    );
                                    if !result.is_empty() {
                                        self.graph_data.push(result);
                                    }
                                }
                            }
                        }

                        if ui.button("View charts").clicked() {
                            self.screen = Screen::ViewChart;
                        }
                        ui.add_space(ui.available_width());
                    });
                });

            let mut csv_columns: Vec<(usize, Vec<String>)> = Vec::new();
            for index in self.multi_pipeline_tracker.keys() {
                if let Some(csv_file) = self.csv_files.get(*index) {
                    let column_titles = csv_file.1
                        .get(0)
                        .map(|row| row.clone())
                        .unwrap_or_default();
                    csv_columns.push((*index, column_titles));
                }
            }
            ui.add_space(35.0);
            ui.horizontal(|ui| {
                ui.style_mut().spacing.indent = 30.0;
                ui.vertical(|ui| {
                    ui.indent("left_margin", |ui| {
                        ScrollArea::vertical()
                            .min_scrolled_height(900.0)
                            .max_height(ui.available_height())
                            .max_width(ui.available_width())
                            .show(ui, |ui| {
                                let indices_and_pipelines: Vec<(usize, Vec<usize>)> = self.multi_pipeline_tracker
                                    .iter()
                                    .map(|(k, v)| (*k, v.clone()))
                                    .collect();
                                // pipelines gui
                                for (index, fields) in csv_columns.iter() {
                                    if let Some(pipelines) = indices_and_pipelines.iter().find(|(k, _)| k == index) {
                                        let _index = *index;
                                        for (index, pipeline_index) in pipelines.1.iter().enumerate() {
                                            ui.heading(RichText::new(format!("{} #{}", self.csv_files[*pipeline_index].0
                                                .split("\\")
                                                .last()
                                                .unwrap_or("No file name"), index + 1))
                                                .color(Color32::BLACK)
                                            );

                                            ui.horizontal(|ui| {
                                                ui.push_id(index, |ui| {
                                                    if ui.button("add pipeline").clicked() {
                                                        if self.multi_pipeline_tracker.contains_key(&_index) {
                                                            let _ = self.multi_pipeline_tracker.get_mut(&_index).expect("REASON").push(_index);

                                                            while self.csvqb_pipelines.len() <= _index {
                                                                self.csvqb_pipelines.push(vec![]);
                                                            }
                                                            self.csvqb_pipelines[_index].push((_index, vec![]));
                                                        }
                                                    };
                                                });

                                                ui.push_id(index, |ui| {
                                                    let mut delete_requested = false;
                                                    if ui.button("delete pipeline").clicked() {
                                                        delete_requested = true;
                                                    }

                                                    if delete_requested {
                                                        if let std::collections::hash_map::Entry::Occupied(mut entry) =
                                                            self.multi_pipeline_tracker.entry(_index) {
                                                            if let Some(pos) = entry.get_mut().iter().position(|&x| x == *pipeline_index) {
                                                                entry.get_mut().remove(pos);
                                                            }
                                                            if entry.get().is_empty() {
                                                                entry.remove();
                                                            }
                                                        }
                                                        if let pipeline = self.csvqb_pipelines[*pipeline_index].remove(index) {
                                                            println!("removed: {:?}", pipeline);
                                                        }

                                                        if index < self.graph_data.len() {
                                                            self.graph_data.remove(index);
                                                        }
                                                    }
                                                });
                                                ui.add_space(112.0);
                                                ui.push_id(_index.to_string() + &*index.to_string(), |ui| {
                                                    ui.horizontal(|ui| {
                                                        egui::ComboBox::from_label("graph type")
                                                            .selected_text(&self.chart_style_prototype)
                                                            .show_ui(ui, |ui| {
                                                                if ui.selectable_value(&mut self.chart_style_prototype, "Bar Graph".to_string(), "Bar Graph").clicked() {
                                                                    if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                        if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                            pipeline.1.insert(0, "Bar Graph".to_string());
                                                                        }
                                                                    } else {
                                                                        self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["Bar Graph".to_string()])]);
                                                                    }
                                                                }
                                                                if ui.selectable_value(&mut self.chart_style_prototype, "Histogram".to_string(), "Histogram").clicked() {
                                                                    if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                        if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                            pipeline.1.insert(0, "Histogram".to_string());
                                                                        }
                                                                    } else {
                                                                        self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["Histogram".to_string()])]);
                                                                    }
                                                                }
                                                                if ui.selectable_value(&mut self.chart_style_prototype, "Pie Chart".to_string(), "Pie Chart").clicked() {
                                                                    if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                        if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                            pipeline.1.insert(0, "Pie Chart".to_string());
                                                                        }
                                                                    } else {
                                                                        self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["Pie Chart".to_string()])]);
                                                                    }
                                                                }
                                                                if ui.selectable_value(&mut self.chart_style_prototype, "Scatter Plot".to_string(), "Scatter Plot").clicked() {
                                                                    if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                        if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                            pipeline.1.insert(0, "Scatter Plot".to_string());
                                                                        }
                                                                    } else {
                                                                        self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["Scatter Plot".to_string()])]);
                                                                    }
                                                                }
                                                                if ui.selectable_value(&mut self.chart_style_prototype, "Line Chart".to_string(), "Line Chart").clicked() {
                                                                    if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                        if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                            pipeline.1.insert(0, "Line Chart".to_string());
                                                                        }
                                                                    } else {
                                                                        self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["Line Chart".to_string()])]);
                                                                    }
                                                                }
                                                                if ui.selectable_value(&mut self.chart_style_prototype, "Flame Graph".to_string(), "Flame Graph(coming soon)").clicked() {
                                                                    if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                        if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                            pipeline.1.insert(0, "Flame Graph".to_string());
                                                                        }
                                                                    } else {
                                                                        self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["Flame Graph".to_string()])]);
                                                                    }
                                                                }
                                                            });
                                                    });
                                                });
                                            });


                                            ui.push_id(index, |ui| {
                                                let mut pipeline_str = if let Some(pipelines) = self.csvqb_pipelines.get_mut(*pipeline_index) {
                                                    if let Some(pipeline) = pipelines.get(index) {
                                                        if !pipeline.1.is_empty() {
                                                            pipeline.1.join(" ")
                                                        } else {
                                                            String::new()
                                                        }
                                                    } else {
                                                        String::new()
                                                    }
                                                } else {
                                                    String::new()
                                                };

                                                if ui.add_sized((ui.available_width() / 3.0, 0.0), TextEdit::singleline(&mut pipeline_str)).changed() {
                                                    while self.csvqb_pipelines[*pipeline_index].len() <= index {
                                                        self.csvqb_pipelines[*pipeline_index].push((index, Vec::new()));
                                                    }

                                                    self.csvqb_pipelines[*pipeline_index][index].1 = pipeline_str
                                                        .split_whitespace()
                                                        .map(String::from)
                                                        .collect();
                                                }
                                            });

                                            ui.push_id(index, |ui| {
                                                ui.label(RichText::new("csv columns".to_string()));
                                                ui.set_min_size(Vec2::new(ui.available_width() / 3.0, 100.0));
                                                ScrollArea::vertical()
                                                    .max_width(ui.available_width() / 3.0)
                                                    .show(ui, |ui| {
                                                        ui.horizontal_wrapped(|ui| {
                                                            for field in fields.iter() {
                                                                if ui.button(field).clicked() {
                                                                    if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() >= *pipeline_index {
                                                                        if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                            pipeline.1.push(field.to_string());
                                                                        }
                                                                    } else {
                                                                        self.csvqb_pipelines.push(vec![(*pipeline_index, vec![field.to_string()])]);
                                                                    }
                                                                }
                                                            }
                                                        });
                                                    });
                                            });
                                            ui.add_space(-20.0);
                                            ui.label(RichText::new("pipeline operators".to_string()));
                                            ui.push_id(index, |ui| {
                                                ui.set_min_size(Vec2::new(ui.available_width() / 3.0, 100.0));
                                                ui.set_max_height(25.0);
                                                ScrollArea::vertical()
                                                    .show(ui, |ui| {
                                                        ui.horizontal_wrapped(|ui| {
                                                            if ui.button("(").clicked() {
                                                                if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                    if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                        pipeline.1.push("(".to_string());
                                                                    }
                                                                } else {
                                                                    self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["(".to_string()])]);
                                                                }
                                                            }
                                                            if ui.button(")").clicked() {
                                                                if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                    if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                        pipeline.1.push(")".to_string());
                                                                    }
                                                                } else {
                                                                    self.csvqb_pipelines.push(vec![(*pipeline_index, vec![")".to_string()])]);
                                                                }
                                                            }
                                                            if ui.button("GRP").clicked() {
                                                                if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                    if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                        pipeline.1.push("GRP".to_string());
                                                                    }
                                                                } else {
                                                                    self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["GRP".to_string()])]);
                                                                }
                                                            }
                                                            if ui.button("CSUM").clicked() {
                                                                if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                    if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                        pipeline.1.push("CSUM".to_string());
                                                                    }
                                                                } else {
                                                                    self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["CSUM".to_string()])]);
                                                                }
                                                            }
                                                            if ui.button("CAVG").clicked() {
                                                                if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                    if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                        pipeline.1.push("CAVG".to_string());
                                                                    }
                                                                } else {
                                                                    self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["CAVG".to_string()])]);
                                                                }
                                                            }
                                                            if ui.button("CCOUNT").clicked() {
                                                                if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                    if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                        pipeline.1.push("CCOUNT".to_string());
                                                                    }
                                                                } else {
                                                                    self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["CCOUNT".to_string()])]);
                                                                }
                                                            }
                                                            if ui.button("MUL").clicked() {
                                                                if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                    if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                        pipeline.1.push("MUL".to_string());
                                                                    }
                                                                } else {
                                                                    self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["MUL".to_string()])]);
                                                                }
                                                            }
                                                            if ui.button("=").clicked() {
                                                                if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                    if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                        pipeline.1.push("=".to_string());
                                                                    }
                                                                } else {
                                                                    self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["=".to_string()])]);
                                                                }
                                                            }
                                                            if ui.button(">").clicked() {
                                                                if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                    if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                        pipeline.1.push(">".to_string());
                                                                    }
                                                                } else {
                                                                    self.csvqb_pipelines.push(vec![(*pipeline_index, vec![">".to_string()])]);
                                                                }
                                                            }
                                                            if ui.button("<").clicked() {
                                                                if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len() - 1 >= *pipeline_index {
                                                                    if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                        pipeline.1.push("<".to_string());
                                                                    }
                                                                } else {
                                                                    self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["<".to_string()])]);
                                                                }
                                                            }
                                                        });
                                                    });
                                            });
                                        }
                                        ui.label("─".repeat(50));
                                    }
                                }
                                ui.add_space(135.0);
                            });
                    });
                });

                ui.vertical_centered_justified(|ui| {
                    ui.heading(RichText::new("Pipeline output").color(Color32::BLACK));
                    let expression_data = &self.graph_data;
                    ScrollArea::vertical()
                        .show(ui, |ui: &mut Ui| {
                            for (index, row) in expression_data.iter().enumerate() {
                                ui.group(|ui| {
                                    Frame::NONE
                                        .fill(Color32::WHITE)
                                        .inner_margin(Margin::symmetric(60.0 as i8, 0.0 as i8))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new(format!("query #{}", index + 1)));
                                            ScrollArea::vertical()
                                                .max_height(300.0)
                                                .id_salt(index)
                                                .enable_scrolling(true)
                                                .show(ui, |ui| {
                                                    ui.label(RichText::new(format!("{:?}", row)).color(Color32::BLACK));
                                                    ui.add_space(199.0);
                                                });
                                        });
                                });
                                ui.add_space(35.0);
                            }
                            ui.add_space(335.0);
                        });
                })
            });
        });
    }

    fn show_chart_screen(&mut self, ctx: &Context) {
        let frame = Frame::default()
            .fill(Color32::from_rgb(193, 200, 208));

        let mut indices_to_remove: Vec<usize> = Vec::new();
        CentralPanel::default().frame(frame).show(ctx, |ui| {
            Frame::NONE
                .fill(Color32::from_rgb(193, 200, 208))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        if ui.button("Home").clicked() {
                            self.screen = Screen::Main;
                        }

                        if ui.button("Explorer").clicked() {
                            self.screen = Screen::CreateChart;
                        }

                        if ui.button("Edit Mode").clicked() {
                            self.chart_view_editing = !self.chart_view_editing;
                        }

                        if self.chart_view_editing {
                            if ui.button("Add Label").clicked() {
                                let new_label = DraggableLabel {
                                    text: String::new(),
                                    pos: ui.cursor().left_top(),
                                    id: self.next_label_id,
                                };
                                self.labels.push(new_label);
                                self.next_label_id += 1;
                            }
                        }

                        ui.add_space(ui.available_width());
                    })
                });

            ui.add_space(21.0);

            ScrollArea::both().show(ui, |ui| {
                for (index, graph_query) in self.graph_data.iter().enumerate() {
                    let window_id = ui.make_persistent_id(format!("chart_window_{}", index));
                    let formatted_data = Some(format_graph_query(graph_query.clone()));
                    Window::new("")
                        .id(window_id)
                        .collapsible(false)
                        .resizable(true)
                        .movable(true)
                        .default_size(Vec2::new(600.0, 320.0))
                        .min_width(200.0)
                        .min_height(170.0)
                        .default_height(320.0)
                        .show(ctx, |ui| {
                            if let Some(start_time) = self.time_to_hide_state {
                                if start_time.elapsed().as_millis() > 100 {
                                    self.time_to_hide_state = None;
                                }
                                save_window_as_png(ctx, window_id);
                            } else {
                                ui.horizontal(|ui| {
                                    if ui.button("Save as .png").clicked() {
                                        self.time_to_hide_state = Some(Instant::now());
                                    }
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("❌").clicked() {
                                            indices_to_remove.push(index);
                                        }
                                    });
                                });
                            }
                            Frame::NONE
                                .fill(ui.style().visuals.window_fill())
                                .inner_margin(Margin::symmetric(20.0 as i8, 20.0 as i8))
                                .show(ui, |ui| {
                                    if index < self.labels.len() {
                                        let label = &mut self.labels[index];
                                        let label_id = Id::new(format!("label_{}", label.id));

                                        if let Some(response) = Window::new(&label.text)
                                            .id(label_id)
                                            .title_bar(false)
                                            .resizable(true)
                                            .movable(true)
                                            .constrain(true)
                                            .max_height(40.0)
                                            .order(Order::Foreground)
                                            .current_pos(label.pos)
                                            .frame(Frame {
                                                inner_margin: Margin::same(0.0 as i8),
                                                outer_margin: Margin::same(0.0 as i8),
                                                shadow: egui::Shadow::NONE,
                                                fill: Color32::TRANSPARENT,
                                                stroke: egui::Stroke::NONE,
                                                corner_radius: Default::default(),
                                            })
                                            .collapsible(true)
                                            .show(ctx, |ui| {
                                                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                                                    ui.horizontal(|ui| {
                                                        let label_response = ui.label(&label.text);
                                                        if label_response.clicked() {
                                                            println!("maybe do something with clicks?");
                                                        }
                                                    });
                                                    if self.chart_view_editing {
                                                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                                            let response = ui.text_edit_singleline(&mut label.text);
                                                            response.changed()
                                                        });
                                                    }
                                                })
                                            })
                                        {
                                            label.pos = response.response.rect.left_top();
                                        }
                                    }
                                    match &graph_query[0] {
                                        Value::Field(graph_type) if graph_type == "Bar Graph" => {
                                            let _ = draw_bar_graph(ui, formatted_data);
                                        }
                                        Value::Field(graph_type) if graph_type == "Pie Chart" => {
                                            let _ = draw_pie_chart(ui, formatted_data);
                                        }
                                        Value::Field(graph_type) if graph_type == "Histogram" => {
                                            let _ = draw_histogram(ui, formatted_data);
                                        }
                                        Value::Field(graph_type) if graph_type == "Scatter Plot" => {
                                            let _ = draw_scatter_plot(ui, formatted_data);
                                        }
                                        Value::Field(graph_type) if graph_type == "Line Chart" => {
                                            let _ = draw_line_chart(ui, formatted_data);
                                        }
                                        Value::Field(graph_type) if graph_type == "Flame Graph" => {
                                            let _ = draw_flame_graph(ui, formatted_data);
                                        }
                                        _ => {}
                                    }
                                });
                        });
                }
            });
        });

        for &index in indices_to_remove.iter().rev() {
            self.graph_data.remove(index);
        }
    }

    fn render_settings(&mut self, ctx: &Context) {
        let frame = Frame::default()
            .fill(Color32::from_rgb(193, 200, 208));

        CentralPanel::default().frame(frame).show(ctx, |ui| {
            Frame::NONE
                .fill(Color32::from_rgb(193, 200, 208))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        if ui.button("Home").clicked() {
                            self.screen = Screen::Main;
                        }

                        if ui.button("Explorer").clicked() {
                            self.screen = Screen::CreateChart;
                        }

                        if ui.button("Charts").clicked() {
                            self.screen = Screen::ViewChart;
                        }


                        ui.add_space(ui.available_width());
                    })
                });

            ui.add_space(21.0);

            Frame::NONE
                .fill(Color32::from_rgb(193, 200, 208))
                .show(ui, |ui| {
                   ui.vertical_centered_justified(|ui| {
                       ui.heading("Settings");

                       ui.add_space(20.0);
                       ui.checkbox(&mut self.dark_mode_enabled, "Dark Mode").changed().then(|| {
                           if self.dark_mode_enabled {
                               ctx.set_theme(egui::Theme::Dark);
                           } else {
                               ctx.set_theme(egui::Theme::Light);
                           }
                       });

                       ui.add_space(10.0);
                       ui.checkbox(&mut self.db_config.enabled, "Enable Database Support");

                       if self.db_config.enabled {

                           ui.add_space(10.0);
                           ui.horizontal(|ui| {
                               ui.add_space(ui.available_width() / 2.0 - 120.0);
                               ui.label("Database Type:");
                               if ui.radio_value(&mut self.db_config.db_type, DatabaseType::SQLite, "SQLite").clicked() {
                                   println!("SQLite");
                               }
                               if ui.radio_value(&mut self.db_config.db_type, DatabaseType::PostgreSQL, "PostgreSQL").clicked() {
                                   println!("PostgreSQL");
                               }
                           });

                           ui.add_space(10.0);

                           match self.db_config.db_type {
                               DatabaseType::SQLite => {
                                   ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                       ui.horizontal(|ui| {
                                           ui.add_space(ui.available_width() / 2.0 - 120.0);
                                           ui.radio_value(
                                               &mut self.db_config.database_path,
                                               DatabaseSource::Default,
                                               "Use Built-in Database"
                                           );
                                           let current_path = self.db_config.database_path.get_path();
                                           ui.radio_value(
                                               &mut self.db_config.database_path,
                                               DatabaseSource::Custom(current_path),
                                               "I have my own database"
                                           );
                                       });

                                       if matches!(self.db_config.database_path, DatabaseSource::Custom(_)) {
                                           if ui.button("Choose SQLite Database Location").clicked() {
                                               if let Some(path) = rfd::FileDialog::new()
                                                   .add_filter("SQLite Database", &["db", "sqlite"])
                                                   .save_file() {
                                                   self.db_config.database_path = DatabaseSource::Custom(path);
                                               }
                                           }

                                           if let DatabaseSource::Custom(path) = &self.db_config.database_path {
                                               ui.add_space(5.0);
                                               ui.label(format!("Selected: {}", path.display()));
                                           }
                                       }

                                       if let Ok(conn) = rusqlite::Connection::open(self.db_config.database_path.get_path()) {

                                           if let Err(err) = DbManager::import_all_csvs(&conn, &self.csv_files) {
                                               println!("err {}", err)
                                           }

                                           if let Err(err) = render_db_stats(ui, &conn) {
                                               ui.label(format!("Error loading database stats: {}", err));
                                           }
                                       }
                                   });

                               }
                               DatabaseType::PostgreSQL => {
                                   // todo(Billy) implement PostgresSQL
                                   ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                       //specific settings
                                   });
                               }
                               DatabaseType::MongoDB => {
                                   // todo(Billy) implement MongoDB
                                   ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                       //specific settings
                                   });
                               }
                           }

                       }
                   })
                });
        });
    }
}
