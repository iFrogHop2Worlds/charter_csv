use crate::charter_graphs::{draw_bar_graph, draw_flame_graph, draw_histogram, draw_line_chart, draw_pie_chart, draw_scatter_plot};
use crate::charter_utilities::{check_for_screenshot, cir_parser, custom_divider, grid2csv, grid_search, render_db_stats, save_window_as_png, CsvGrid, DraggableLabel, GridLayout, SearchResult};
use crate::cir_adapters::sqlite_cir_adapter;
use crate::csvqb::{csvqb_to_cir, CIR};
use crate::db_manager::{DatabaseConfig, DatabaseSource, DatabaseType, DbManager, };
use crate::session::{load_session_files_from_db, load_sessions_from_db, reconstruct_session, retrieve_session_list, save_session_to_database, update_current_session, Session};
use eframe::App;
use egui::{emath, vec2, Align, Align2, Button, CentralPanel, Color32, Context, FontId, Frame, IconData, Id, Image, LayerId, Margin, Order, PointerButton, Pos2, RichText, ScrollArea, Sense, Shape, Stroke, TextEdit, TextureHandle, Ui, Vec2, Window};
use image::ImageReader;
use itertools::Itertools;
use std::collections::HashMap;
use std::io::BufRead;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
pub use std::thread;
use std::time::Instant;
use rayon::prelude::*;
use crate::components::optimized_load_csv_button::CsvLoaderButton;
use egui::emath::Rot2 as Rotation2D;
use egui::epaint::TextShape;

// Application is still in early development App state is scheduled for a refactor soon.
pub struct CharterCsvApp {
    db_manager: Option<DbManager>,
    db_config: DatabaseConfig,
    texture: Option<TextureHandle>,
    screen: Screen,
    csv_files: Vec<(String, CsvGrid)>,
    grid_layout: Option<GridLayout>,
    csvqb_pipelines: Vec<Vec<(usize, Vec<String>)>>,
    multi_pipeline_tracker: HashMap<usize, Vec<usize>>,
    graph_data: Vec<Vec<CIR>>,
    file_receiver: Receiver<(String, Vec<Vec<String>>)>,
    file_sender: Sender<(String, Vec<Vec<String>>)>,
    chart_style_prototype: String,
    sessions: Vec<Session>,
    current_session: usize,
    prev_session: usize,
    show_ss_name_popup: bool,
    edit_ss_name: String,
    time_to_hide_state: Option<Instant>,
    labels: Vec<DraggableLabel>,
    next_label_id: u32,
    show_labels: bool,
    chart_view_editing: bool,
    search_text: String,
    additional_searches: Vec<SearchResult>,
    dark_mode_enabled: bool,
    query_mode: DatabaseType,
    divider_position: f32
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
            current_session: 0,
            prev_session: 100000,
            show_ss_name_popup: false,
            edit_ss_name: "".to_string(),
            time_to_hide_state: None,
            labels: Vec::new(),
            next_label_id: 1,
            show_labels: true,
            chart_view_editing: false,
            search_text: "".to_string(),
            additional_searches: vec![],
            dark_mode_enabled: false,
            query_mode: DatabaseType::CsvQB,
            divider_position: 555.0,
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
            self.csv_files.push((path.clone(), grid.clone()));
            if let Ok(mut conn) = rusqlite::Connection::open(self.db_config.database_path.get_path()) {
                if let Err(err) = DbManager::import_all_csvs(&mut conn, &vec![(path, grid)]) {
                    println!("err {}", err)
                }

                update_current_session(
                    &self.csv_files,
                    &mut self.csvqb_pipelines,
                    &mut self.sessions,
                    &mut self.multi_pipeline_tracker,
                    self.current_session,
                    &self.query_mode,
                    conn
                );
            }
        }

        let screen = std::mem::replace(&mut self.screen, Screen::Main);
        match screen {
            Screen::Main => {
                self.screen = screen;
                self.show_main_screen(ctx)
            }
            Screen::ViewCsv => {
                self.screen = screen;
                self.file_manager_screen(ctx)
            }
            Screen::CreateCsv { content } => {
                let mut content_owned = content;
                let next_screen = self.csv_editor_screen(ctx, &mut content_owned, None);
                self.screen = match next_screen {
                    Some(screen) => screen,
                    None => Screen::CreateCsv { content: content_owned },
                };
            }
            Screen::EditCsv { index, content } => {
                let mut content_owned = content;
                let next_screen = self.csv_editor_screen(ctx, &mut content_owned, Some(index));
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
                self.data_explorer_screen(ctx);
            }
            Screen::ViewChart => {
                self.screen = screen;
                self.chart_display_screen(ctx)
            }
            Screen::Settings => {
                self.screen = screen;
                self.settings_screen(ctx)
            }
        }

        // todo (Billy) Think of a better way to gaurd this so we don't continuously load sessions, not hurting perf but I don't like it.
        // if self.current_session == 0 && self.prev_session == 100000 {
            if let Ok(conn) = rusqlite::Connection::open(self.db_config.database_path.get_path()) {
                self.sessions = load_sessions_from_db(&conn).expect("Failed to load sessions");
            }
        //     self.prev_session = 99999
        // }

        // deprecated local text file based sessions
        //self.sessions = load_sessions_from_directory().expect("Failed to restore sessions");

        if self.current_session != self.prev_session && !self.sessions.is_empty() {
            if let Ok(mut conn) = rusqlite::Connection::open(self.db_config.database_path.get_path()) {
                if let Ok(files) = load_session_files_from_db(&mut conn, &*self.sessions[self.current_session].name) {
                    for (file_path, grid) in files {
                        self.csv_files.push((file_path, grid));
                    }
                }

                if let Err(err) = DbManager::import_all_csvs(&mut conn, &self.csv_files) {
                    println!("err {}", err)
                }
           }



            let ssi = self.current_session;
            if ssi >= 0 && ssi < self.sessions.len() {
                let query_mode = self.sessions[ssi].query_mode.clone();
                self.query_mode = query_mode;
            }
            let selected_files: Vec<usize> = self.multi_pipeline_tracker.keys().copied().sorted().collect();

            // we run session queries here to set graph data
            for (_index, pipelines) in self.csvqb_pipelines.iter().enumerate() {
                for (i, _) in pipelines.iter().enumerate() {

                    if self.db_config.db_type.is(DatabaseType::SQLite) {
                        let combined_query = self.csvqb_pipelines[_index][i].1.join(" ");
                        let conn_path = self.db_config.database_path.get_path();
                        let _ = sqlite_cir_adapter(combined_query, conn_path, self.graph_data.as_mut());

                    } else if self.db_config.db_type.is(DatabaseType::CsvQB) {
                        let result = csvqb_to_cir(
                            &*self.csvqb_pipelines[_index][i].1,
                            &selected_files,
                            &self.csv_files,
                        );
                        if !result.is_empty() {
                            self.graph_data.push(result);
                        }
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
                        let csv_loader = CsvLoaderButton::new();
                        // Or with custom config:
                        // let csv_loader = CsvLoaderButton::with_config(ChunkConfig {
                        //     chunk_size: 16 * 1024 * 1024,
                        //     max_chunks_in_memory: 8,
                        // });

                        csv_loader.show(
                            ui,
                            self.file_sender.clone(),
                        );

                        if ui.button("New File").clicked() {
                            self.screen = Screen::CreateCsv {
                                content: (
                                    "/todo/set path".to_string(),
                                    vec![vec!["".to_string()]],
                                ),
                            };
                        }

                        if ui.button("File Manager").clicked() {
                            self.screen = Screen::ViewCsv;
                        }

                        if ui.button("Data Explorer").clicked() {
                            self.screen = Screen::CreateChart;
                        }

                        if ui.button("View Charts").clicked() {
                            self.screen = Screen::ViewChart;
                        }
                        
                        if ui.button("print_state").clicked() {
                            println!("CSV Files: {:?}", self.csv_files);
                            println!("\nCSVQB Pipelines: {:?}", self.csvqb_pipelines);
                            println!("\nGraph Data: {:?}", self.graph_data);
                            println!("\nMulti Pipeline Tracker: {:?}", self.multi_pipeline_tracker);
                            println!("\nSessions: {:?}", self.sessions);
                            println!("\nCurrent Session: {:?}", self.current_session);
                            println!("\nPrevious Session: {:?}", self.prev_session);
                            println!("\nQuery Mode: {:?}", self.query_mode);
                            println!("\nShow SS Name Popup: {:?}", self.show_ss_name_popup);
                        }

                        if self.show_ss_name_popup {
                            Window::new("Enter Session Name")
                                .collapsible(false)
                                .resizable(false)
                                .show(ctx, |ui| {
                                    ui.text_edit_singleline(&mut self.edit_ss_name);
                                    ui.horizontal(|ui| {
                                        if ui.button("OK").clicked() {
                                            if let Ok(conn) = rusqlite::Connection::open(self.db_config.database_path.get_path()) {
                                                let session = Session {
                                                    name: self.edit_ss_name.to_owned(),
                                                    files: vec![],
                                                    pipelines: vec![],
                                                    selected_files: vec![],
                                                    query_mode: self.query_mode.clone(),

                                                };
                                                if let Err(err) = save_session_to_database( conn, vec![session]) {
                                                    println!("{}", format!("Error saving session to sql lite db: {}", err));
                                                }
                                            }
                                            //save_session(self.edit_ss_name.to_owned(), vec![], vec![], vec![], &self.query_mode).expect("session save failed");
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
                if let Ok(conn) = rusqlite::Connection::open(self.db_config.database_path.get_path()) {
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
                                update_current_session(
                                    &self.csv_files,
                                    &mut self.csvqb_pipelines,
                                    &mut self.sessions,
                                    &mut self.multi_pipeline_tracker,
                                    self.current_session,
                                    &self.query_mode,
                                    conn
                                );
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
                        ui.add_space(10.0);
                        if ui.button("New Session").clicked() {
                            self.show_ss_name_popup = true;
                        }
                        ui.add_space(20.0);
                        ui.label(RichText::new("You can use the app without a session but it is best to create a session to work in.").color(Color32::BLACK));
                        ui.add_space(20.0);
                        ui.label(RichText::new("You can switch between sessions or save it for later.").color(Color32::BLACK));
                        ui.add_space(20.0);
                        ui.label(RichText::new("The Data Explorer is where you can quickly search and compare the data in your files.").color(Color32::BLACK));
                        ui.add_space(20.0);
                        ui.label(RichText::new("You can create new csv files by clicking New File or you can edit files in the File Manager screen.").color(Color32::BLACK));
                        ui.add_space(20.0);
                        ui.label(RichText::new("Have fun exploring the depths of your data!.").color(Color32::BLACK));
                    }
                }

                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            if let Ok(conn) = rusqlite::Connection::open(self.db_config.database_path.get_path()) {
                                let sessions = retrieve_session_list(&conn).expect("Failed to retrieve session list");
                                for (index, session) in sessions.iter().enumerate() {

                                    let active_session = if self.current_session == index {
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
                                                        ui.with_layout(egui::Layout::top_down_justified(Align::LEFT), |ui| {
                                                            ui.add_space(3.0);
                                                            ui.horizontal(|ui| {
                                                                ui.label(RichText::new(format!("name: {}", session.name)).color(Color32::BLACK));
                                                                ui.add_space(ui.available_width() - 120.0);
                                                                ui.label(RichText::new(format!("working in {}", session.query_mode)).color(Color32::BLACK));
                                                            });
                                                            ui.label(RichText::new(format!("files: {:?}", session.file_count)).color(Color32::BLACK));
                                                            ui.label(RichText::new(format!("pipelines: {:?}", session.pipeline_count)).color(Color32::BLACK));

                                                            if self.current_session != index {
                                                                ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
                                                                    if ui.add_sized(
                                                                        Vec2::new(180.0, 20.0),
                                                                        Button::new("Load")
                                                                            .corner_radius(12.0)
                                                                    ).clicked() {
                                                                        self.current_session = index;
                                                                        self.multi_pipeline_tracker.clear();
                                                                        self.csv_files.clear();
                                                                        self.csvqb_pipelines.clear();
                                                                        self.graph_data.clear();

                                                                        let receiver = reconstruct_session(self.sessions[index].clone());

                                                                        while let Ok((file_path, grid)) = receiver.recv() {
                                                                            self.csv_files.push((file_path, grid));
                                                                        }
                                                                        let mut grouped_pipelines: Vec<Vec<(usize, Vec<String>)>> = Vec::new();
                                                                        let mut temp_map: HashMap<usize, Vec<(usize, Vec<String>)>> = HashMap::new();

                                                                        for (_index, pipeline) in self.sessions[index].pipelines.iter().enumerate() {
                                                                            if pipeline.is_empty() {
                                                                                println!("Warning: Empty pipeline found, skipping...");
                                                                                continue;
                                                                            }

                                                                            for query in pipeline {
                                                                                if let Some(query_str) = query.to_string().split_once(' ') {
                                                                                    let (number, remainder) = query_str;    
                                                                                    if let Ok(index) = number.parse::<usize>() {

                                                                                        self.multi_pipeline_tracker.entry(index)
                                                                                            .and_modify(|pipes| pipes.push(index))
                                                                                            .or_insert_with(|| vec![index]);

                                                                                        temp_map
                                                                                            .entry(index)
                                                                                            .or_insert_with(Vec::new)
                                                                                            .push((index, remainder.split(' ').map(|s| s.to_string()).collect()));
                                                                                    }
                                                                                }
                                                                            }
                                                                        }

                                                                        let mut indices: Vec<usize> = temp_map.keys().cloned().collect();
                                                                        indices.sort();
                                                                        for index in indices {
                                                                            if let Some(pipeline_group) = temp_map.get(&index) {
                                                                                grouped_pipelines.push(pipeline_group.clone());
                                                                            }
                                                                        }

                                                                        if !grouped_pipelines.is_empty() {
                                                                            self.csvqb_pipelines = grouped_pipelines;
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
                            }

                        });

                });
            });
        });
    }

    fn file_manager_screen(&mut self, ctx: &Context) {
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
                        let csv_loader = CsvLoaderButton::new();
                        // Or with custom config:
                        // let csv_loader = CsvLoaderButton::with_config(ChunkConfig {
                        //     chunk_size: 16 * 1024 * 1024,
                        //     max_chunks_in_memory: 8,
                        // });

                        csv_loader.show(
                            ui,
                            self.file_sender.clone(),
                        );
                        ui.add_space(ui.available_width());
                    })
                });

            ui.add_space(21.0);

            let desired_width = ui.available_width() / 3.0;
            ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
                for (index, file) in self.sessions[self.current_session].files.iter().enumerate() {
                    let file_name = file.split("\\")
                        .last()
                        .unwrap_or("No file name")
                        .replace(['-', ' '], "_");
                    ui.push_id(index, |ui| {
                        let total_width = ui.available_width();
                        let padding = (total_width - desired_width) / 2.0;
                        ui.allocate_space(Vec2::new(padding, 0.0));

                        let file_loaded_inmemory = self.csv_files.get(index)
                            .map(|(files, _)| {
                                files.split("\\").last().unwrap_or("No file name") == file_name })
                            .unwrap_or(false);

                        let mut frame = Frame::group(ui.style());
                        if file_loaded_inmemory {
                            frame = frame
                                .fill(Color32::from_rgb(150, 200, 150))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(100, 150, 100)));
                        }

                        frame.show(ui, |ui| {
                            ui.set_max_width(desired_width);
                            ui.horizontal(|ui| {
                                ui.label(&file_name);
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    while self.csv_files.len() <= index {
                                        self.csv_files.push(("".to_string(), CsvGrid::new()));
                                    }
                                    if ui.button("unload").clicked() {
                                        files_to_remove = Some(index);
                                    }

                                    if ui.button("Load from DB").clicked() {
                                        //let sender = self.file_sender.clone();
                                        let db_path = self.db_config.database_path.get_path().to_owned();
                                        let file_name = file_name.trim_end_matches(".csv").to_string();

                                        if let Ok(mut conn) = rusqlite::Connection::open(&db_path) {
                                            //thread::spawn(move || {
                                                if let Ok((path, grid)) = DbManager::load_file_from_db(&mut conn, &*file_name) {
                                                    println!("file path: {}", path);
                                                    self.csv_files[index] = (path, grid);
                                                }   

                                            //});
                                        }

                                    }

                                    if file_loaded_inmemory {
                                        if ui.button("edit").clicked() {
                                            next_screen = Some(Screen::EditCsv {
                                                index,
                                                content: self.csv_files[index].clone(),
                                            });
                                        }
                                    }

                                });
                            });
                        });
                    });
                }
            });
        });

        if let Some(index) = files_to_remove {
            if index < self.csv_files.len() {
                let file = format!("_{}_", self.csv_files[index].0.clone());
                self.csv_files[index] = (file, CsvGrid::new());
            }
        }
        if let Some(screen) = next_screen {
            self.screen = screen;
        }
    }

    fn csv_editor_screen(
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

    fn data_explorer_screen(&mut self, ctx: &Context) {
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
                                for (index, file) in self.sessions[self.current_session].files.iter().enumerate() {
                                    let file_name = file.split('\\').last().and_then(|f| f.split('.').next()).unwrap_or("No file name");
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
                            if !self.sessions[self.current_session].pipelines.is_empty() {
                                self.sessions[self.current_session].pipelines.clear();
                            }
                        }

                        if ui.button("Run all").clicked() {
                            self.graph_data.clear();

                            let selected_files = &self.multi_pipeline_tracker.keys().copied().collect::<Vec<usize>>();
                            for (root, indexes) in self.multi_pipeline_tracker.iter() {
                                for (i, _) in indexes.iter().enumerate() {
                                    match self.query_mode {
                                        DatabaseType::CsvQB => {
                                            let result = csvqb_to_cir(
                                                &*self.csvqb_pipelines[*root][i].1,
                                                selected_files,
                                                &self.csv_files,
                                            );
                                            if !result.is_empty() {
                                                self.graph_data.push(result);
                                            }
                                        }
                                        DatabaseType::SQLite => {
                                            let combined_query = self.csvqb_pipelines[*root][i].1.join(" ");
                                            let conn_path = self.db_config.database_path.get_path();
                                            let _ = sqlite_cir_adapter(combined_query, conn_path, self.graph_data.as_mut());
                                        }
                                        DatabaseType::PostgreSQL => {
                                            println!("coming soon")
                                        }
                                        DatabaseType::MongoDB => {
                                            println!("coming soon")
                                        }
                                    }
                                }
                            }
                        }

                        if ui.button("View charts").clicked() {
                            self.screen = Screen::ViewChart;
                        }
                        

                        ui.add_space(8.0);
                        egui::ComboBox::from_label("Query Mode")
                            .selected_text(format!("{:?}", self.query_mode))
                            .show_ui(ui, |ui| {
                                for db_type in &[DatabaseType::CsvQB, DatabaseType::SQLite, DatabaseType::PostgreSQL, DatabaseType::MongoDB] {
                                    if ui.selectable_value(&mut self.query_mode, db_type.clone(), format!("{:?}", db_type)).clicked() {
                                        self.query_mode = db_type.clone();
                                        println!("Changed query mode to: {:?}", db_type);
                                    }
                                }
                            });

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
                                ui.set_width(self.divider_position);
                                let indices_and_pipelines: Vec<(usize, Vec<usize>)> = self.multi_pipeline_tracker
                                    .iter()
                                    .map(|(k, v)| (*k, v.clone()))
                                    .collect();

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

                                                if ui.add_sized((ui.available_width() - 20.0, 0.0), TextEdit::singleline(&mut pipeline_str)).changed() {
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
                                                    .max_width(ui.available_width() - 20.0)
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
                                                    .max_width(ui.available_width() - 20.0)
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
                                            ui.separator();
                                        }
                                    }
                                }
                                ui.add_space(135.0);
                            });
                    });
                });


                let (response, new_x) = custom_divider(ui, self.divider_position);
                if response.dragged() {
                    self.divider_position = new_x;
                }


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

    fn chart_display_screen(&mut self, ctx: &Context) {
        let frame = Frame::default()
            .fill(Color32::from_rgb(193, 200, 208));

        let mut indices_to_remove: Vec<usize> = Vec::new();
        let mut labels_to_remove: Vec<usize> = Vec::new();

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
                                    drag_start: None,
                                    rotation: 0.0,
                                };
                                self.labels.push(new_label);
                                self.next_label_id += 1;
                            }
                        }

                        ui.add_space(ui.available_width());
                    })
                });

            ui.add_space(21.0);

            if !self.labels.is_empty() {
                for (index, label) in self.labels.iter_mut().enumerate() {
                    let label_id = Id::new(format!("label_{}", label.id));
                    let mut stroke = Stroke::NONE;

                    if self.chart_view_editing {
                        stroke = Stroke::new(1.0, Color32::BLACK);
                    }

                    if let Some(response) = Window::new(format!("window_{}", label.id))
                        .id(label_id)
                        .title_bar(false)
                        .resizable(false)
                        .movable(true)
                        .constrain(true)
                        .max_height(220.0)
                        .order(Order::Foreground)
                        .current_pos(label.pos)
                        .frame(Frame {
                            inner_margin: Margin::same(0.0 as i8),
                            outer_margin: Margin::same(0.0 as i8),
                            shadow: egui::Shadow::NONE,
                            fill: Color32::TRANSPARENT,
                            stroke,
                            corner_radius: Default::default(),
                        })
                        .show(ctx, |ui| {
                            ui.with_layout(egui::Layout::left_to_right(Align::BOTTOM), |ui| {
                                if self.chart_view_editing {
                                    let response = ui.text_edit_singleline(&mut label.text);
                                    response.changed();

                                    if ui.button("❌").clicked() {
                                        labels_to_remove.push(index);
                                    }

                                    let rot_btn = ui.add(Button::new("↻").sense(Sense::drag()));

                                    if rot_btn.drag_started() {
                                        label.drag_start = Some(ui.input(|i| i.pointer.hover_pos()).unwrap_or_default());
                                    }
                                    if rot_btn.dragged() {
                                        if let Some(start_pos) = label.drag_start {
                                            let current_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or_default();
                                            let delta = current_pos - start_pos;

                                            let rotation_speed = 0.5;
                                            label.rotation += delta.x * rotation_speed;

                                            label.drag_start = Some(current_pos);
                                        }
                                    }
                                    if rot_btn.drag_released() {
                                        label.drag_start = None;
                                    }
                                }

                                let painter = ui.painter();
                                let galley = painter.layout_no_wrap(
                                    label.text.clone(),
                                    FontId::proportional(12.0),
                                    ui.style().visuals.text_color(),
                                );

                                let rotation_angle = (label.rotation * std::f32::consts::PI) / 180.0;
                                let center = ui.min_rect().center();

                                painter.add(Shape::Text(TextShape {
                                    pos: center - vec2(0.0, 100.0),
                                    galley,
                                    angle: rotation_angle,
                                    underline: Stroke::NONE,
                                    fallback_color: ui.style().visuals.text_color(),
                                    override_text_color: None,
                                    opacity_factor: 1.0,
                                }));
                            })
                        })
                    {
                        label.pos = response.response.rect.left_top();
                    }
                }
            }

            ScrollArea::both().show(ui, |ui| {
                for (index, graph_query) in self.graph_data.iter().enumerate() {
                    let window_id = ui.make_persistent_id(format!("chart_window_{}", index));
                    let formatted_data = Some(cir_parser(graph_query.clone()));
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
                                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
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

                                    match &graph_query[0] {
                                        CIR::Field(graph_type) if graph_type == "Bar Graph" => {
                                            let _ = draw_bar_graph(ui, formatted_data);
                                        }
                                        CIR::Field(graph_type) if graph_type == "Pie Chart" => {
                                            let _ = draw_pie_chart(ui, formatted_data);
                                        }
                                        CIR::Field(graph_type) if graph_type == "Histogram" => {
                                            let _ = draw_histogram(ui, formatted_data);
                                        }
                                        CIR::Field(graph_type) if graph_type == "Scatter Plot" => {
                                            let _ = draw_scatter_plot(ui, formatted_data);
                                        }
                                        CIR::Field(graph_type) if graph_type == "Line Chart" => {
                                            let _ = draw_line_chart(ui, formatted_data);
                                        }
                                        CIR::Field(graph_type) if graph_type == "Flame Graph" => {
                                            let _ = draw_flame_graph(ui, formatted_data);
                                        }
                                        _ => {}
                                    }

                                });
                        });
                }
            });
        });


        // cleanup items tagged for removal
        for &index in indices_to_remove.iter().rev() {
            self.graph_data.remove(index);
        }
        for &index in labels_to_remove.iter().rev() {
            self.labels.remove(index);
        }
    }

    fn settings_screen(&mut self, ctx: &Context) {
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
                                   ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
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


                                           if let Err(err) = render_db_stats(ui, &conn) {
                                               ui.label(format!("Error loading database stats: {}", err));
                                           }
                                       }
                                   });

                               }
                               DatabaseType::PostgreSQL => {
                                   // todo(Billy) implement PostgresSQL
                                   ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
                                       //specific settings
                                   });
                               }
                               DatabaseType::MongoDB => {
                                   // todo(Billy) implement MongoDB
                                   ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
                                       //specific settings
                                   });
                               }
                               _ => {}
                           }

                       }
                   })
                });
        });
    }
}
