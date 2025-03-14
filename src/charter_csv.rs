use eframe::App;
use egui::{Ui, Button, CentralPanel, Color32, Context, IconData, Image, RichText, ScrollArea, TextEdit, TextureHandle, Vec2, ViewportCommand};
use crate::charter_utilities::{csv2grid, grid2csv, CsvGrid, format_graph_query};
use crate::charter_graphs::{draw_bar_graph, draw_flame_graph, draw_histogram, draw_line_chart, draw_pie_chart, draw_scatter_plot};
use crate::csvqb::{process_csvqb_pipeline, Value};
pub use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use image::{ImageReader};
use crate::session::{load_sessions_from_directory, reconstruct_session, save_session, Session};
use std::collections::HashMap;
use itertools::Itertools;

pub struct CharterCsvApp {
    texture: Option<TextureHandle>,
    screen: Screen,
    csv_files: Vec<(String, CsvGrid)>,
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
    edit_ss_name: String
}

pub enum Screen {
    Main,
    ViewCsv,
    CreateCsv { content: (String, CsvGrid) },
    EditCsv { index: usize, content: (String, CsvGrid) },
    CreateChart,
    ViewChart,
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
            texture: None,
            screen: Screen::Main,
            csv_files: vec![],
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
                    self.multi_pipeline_tracker.insert(*file, vec![]);
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
        let frame = egui::Frame::default()
            .fill(Color32::from_rgb(211, 211, 211));

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

            let total_size = ui.available_size();
            let _ = ui.allocate_ui(Vec2::new(total_size.x, total_size.y), |ui| {
                ui.vertical_centered(|_ui| {});
                ui.min_rect().height()
            }).inner;

            let top_margin: f32 = 25.0;
            ui.add_space(top_margin.max(0.0));

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    egui::Frame::NONE
                        .fill(Color32::TRANSPARENT)
                        .stroke(egui::Stroke::NONE)
                        .inner_margin(egui::Margin {
                                left: 60.0 as i8,
                                right: 10.0 as i8,
                                top: 20.0 as i8,
                                bottom: 5.0 as i8,
                            })
                        .show(ui, |ui| {
                            ui.add(
                                Image::new(&*texture)
                                    .max_width(200.0)
                            );
                            ui.add_space(75.0);
                            ui.heading(RichText::new("Charter CSV").color(Color32::BLACK));
                            ui.label(RichText::new("navigate your data with speed and precision").color(Color32::BLACK));
                            ui.add_space(20.0);

                            let menu_btn_size = Vec2::new(300.0, 30.0);
                            if ui.add_sized(menu_btn_size, Button::new("Load File")).clicked() {
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

                            if ui.add_sized(menu_btn_size, Button::new("View Files")).clicked() {
                                self.screen = Screen::ViewCsv;
                            }

                            if ui.add_sized(menu_btn_size, Button::new("Create CSV File")).clicked() {
                                self.screen = Screen::CreateCsv {
                                    content: (
                                        "/todo/set path".to_string(),
                                        vec![vec!["".to_string()]],
                                    ),
                                };
                            }

                            if ui.add_sized(menu_btn_size, Button::new("Data Explorer")).clicked() {
                                self.screen = Screen::CreateChart;
                            }

                            if ui.add_sized(menu_btn_size, Button::new("View Charts")).clicked() {
                                self.screen = Screen::ViewChart;
                            }

                            if ui.add_sized(menu_btn_size, Button::new("Save Session")).clicked() {
                                let mut file_paths:Vec<String> = vec![];
                                let mut pipelines:Vec<String> = vec![];
                                for (path, _) in self.csv_files.iter() {
                                    file_paths.push(path.to_string());
                                }

                                for (_index, pipeline) in self.csvqb_pipelines.iter().enumerate() {
                                    for(index, query_string) in pipeline.iter() {
                                        let pipeline_str = index.to_string() + &*" ".to_string() + &*query_string.join(" ");
                                        pipelines.push(pipeline_str);
                                    }
                                }

                                let ssi = self.current_session as usize;
                                let selected_files = self.multi_pipeline_tracker.keys().copied().sorted().collect();
                                save_session(self.sessions[ssi].name.to_string(), file_paths, pipelines, selected_files).expect("session save failed");
                            }

                            if ui.add_sized(menu_btn_size, Button::new("New Session")).clicked() {
                                self.show_ss_name_popup = true;
                            }

                            if self.show_ss_name_popup {
                                egui::Window::new("Enter Session Name")
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
                            if ui.add_sized(menu_btn_size, Button::new("Close Program")).clicked() {
                                ctx.send_viewport_cmd(ViewportCommand::Close);
                            }
                        });
                });

                ui.vertical_centered_justified(|ui| {
                    ui.add_space(ui.available_height() / 3.0);
                    ui.heading(RichText::new("sessions").color(Color32::BLACK));
                    ui.add_space(60.0);
                    ui.vertical_centered(|ui| {
                        ScrollArea::vertical()
                            .max_height(620.0)
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                for (index, session) in self.sessions.iter().enumerate() {
                                    let name_color = if self.current_session == index as i8 {
                                        Color32::from_rgb(34, 139, 34)
                                    } else {
                                        Color32::BLACK
                                    };
                                    ui.push_id(index, |ui| {
                                        ui.group(|ui| {
                                            let _ = ui.group(|ui| {
                                                ui.set_width(ui.available_width() / 1.4);
                                                egui::Frame::default()
                                                    .show(ui, |ui| {
                                                        if ui.add(Button::new("load session")).clicked() {
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
                                                        ui.label(RichText::new(format!("session name: {}", session.name)).color(name_color));
                                                        ui.label(RichText::new(format!("session data: {:?}", session.files)).color(Color32::BLACK));
                                                        ui.label(RichText::new(format!("session pipelines: {:?}", session.pipelines)).color(Color32::BLACK));
                                                        ui.add_space(12.0);
                                                    });
                                            });
                                        });
                                    });
                                }
                            });
                    });
                })
            })
        });
    }

    fn show_csv_list(&mut self, ctx: &Context) {
        let frame = egui::Frame::default()
            .fill(Color32::from_rgb(211, 211, 211));

        let mut files_to_remove: Option<usize> = None;
        let mut next_screen: Option<Screen> = None;

        CentralPanel::default().frame(frame).show(ctx, |ui| {
            egui::Frame::NONE
                .fill(Color32::from_rgb(192, 192, 192))
                .show(ui, |ui| {
                   ui.horizontal_top(|ui| {
                       if ui.add_sized((100.0, 35.0), Button::new("Home")).clicked() {
                           next_screen = Some(Screen::Main);
                       }
                       ui.add_space(ui.available_width());
                   })
                });

            ui.add_space(21.0);
            for (index, file) in self.csv_files.iter().enumerate() {
                let file_name = file.0.split("\\").last().unwrap_or("No file name");
                ui.push_id(index, |ui| {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.set_min_size(Vec2::new(ui.available_width(), 0.0));
                            ui.label(file_name);
                            if ui.button("edit").clicked() {
                                next_screen = Some(Screen::EditCsv {
                                    index,
                                    content: file.clone(),
                                });
                            }
                            if ui.button("delete").clicked() {
                                files_to_remove = Some(index);
                            }
                        })
                    });
                });
            }

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
        edit_index: Option<usize>
    ) -> Option<Screen> {
        let frame = egui::Frame::default()
            .fill(Color32::from_rgb(211, 211, 211));
        let mut next_screen = None;
        CentralPanel::default().frame(frame).show(ctx, |ui| {
            egui::Frame::NONE
                .fill(Color32::from_rgb(192, 192, 192))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        if ui.add_sized((100.0, 35.0), Button::new("Home")).clicked() {
                            next_screen = Some(Screen::Main);
                        }
                        if ui.add_sized((100.0, 35.0), Button::new("Save")).clicked() {
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

                        if ui.add_sized((100.0, 35.0), Button::new("Add Row")).clicked() {
                            content.1.push(vec!["".to_string(); content.1.get(0).map_or(0, |row| row.len())]);
                        }

                        if ui.add_sized((100.0, 35.0), Button::new("Add Column")).clicked() {
                            for row in &mut content.1 {
                                row.push("".to_string());
                            }
                        }
                        
                        ui.add_space(ui.available_width());
                    })
                });

            ScrollArea::both()
                .auto_shrink([false; 2])
                .show_viewport(ui, |ui, viewport| {
                    let grid = &mut content.1;
                    if grid.is_empty() {
                        return;
                    }

                    const ROW_HEIGHT: f32 = 30.0;
                    const CELL_WIDTH: f32 = 300.0;

                    let total_width = grid[0].len() as f32 * CELL_WIDTH;
                    let total_height = grid.len() as f32 * ROW_HEIGHT;

                    ui.set_min_size(Vec2::new(total_width, total_height));

                    let start_row = (viewport.min.y / ROW_HEIGHT).floor().max(0.0) as usize;
                    let visible_rows = (viewport.height() / ROW_HEIGHT).ceil() as usize + 1;
                    let end_row = (start_row + visible_rows).min(grid.len());

                    let start_col = (viewport.min.x / CELL_WIDTH).floor().max(0.0) as usize;
                    let visible_cols = (viewport.width() / CELL_WIDTH).ceil() as usize + 1;
                    let end_col = (start_col + visible_cols).min(grid[0].len());

                    let top_offset = start_row as f32 * ROW_HEIGHT;
                    ui.add_space(top_offset);

                    for row_idx in start_row..end_row {
                        let row = &mut grid[row_idx];
                        ui.horizontal(|ui| {
                            if start_col > 0 {
                                ui.add_space(start_col as f32 * CELL_WIDTH);
                            }

                            for col_idx in start_col..end_col {
                                if col_idx < row.len() {
                                    let cell = &mut row[col_idx];
                                    ui.add_sized(
                                        Vec2::new(CELL_WIDTH, ROW_HEIGHT),
                                        TextEdit::singleline(cell)
                                    );
                                }
                            }
                        });
                    }

                    let bottom_space = total_height - (end_row as f32 * ROW_HEIGHT);
                    if bottom_space > 0.0 {
                        ui.add_space(bottom_space);
                    }
                });
        });

        next_screen
    }

    fn create_chart_screen(&mut self, ctx: &Context) {
        let frame = egui::Frame::default()
            .fill(Color32::from_rgb(211, 211, 211));

        CentralPanel::default().frame(frame).show(ctx, |ui| {
            egui::Frame::NONE
                .fill(Color32::from_rgb(192, 192, 192))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        if ui.add_sized((100.0, 35.0), Button::new("Home")).clicked() {
                            self.screen = Screen::Main;
                        }

                        ui.add_space(112.0);
                        ui.horizontal(|ui| {
                            ui.group(|ui| {
                                ui.set_min_size(Vec2::new(100.0, 20.0));
                                egui::ComboBox::from_label("Select File")
                                    .width(120.0)
                                    .show_ui(ui, |ui| {
                                        for (index, file) in self.csv_files.iter().enumerate() {
                                            let file_name = &file.0;
                                            let mut selected = self.multi_pipeline_tracker.contains_key(&index);

                                            if ui.checkbox(&mut selected, file_name).clicked() {
                                                if selected {
                                                    println!("selected file: {}", index);
                                                    self.multi_pipeline_tracker.insert(index, vec![index]);
                                                    self.csvqb_pipelines.push(vec![(index, vec![])]);
                                                } else {
                                                    self.multi_pipeline_tracker.remove(&index);
                                                    self.csvqb_pipelines.remove(index);
                                                }
                                                //self.multi_pipeline_tracker.insert(index,vec![]);
                                                // while self.csvqb_pipelines.len() <= index {
                                                //     self.csvqb_pipelines.insert(index, vec![]);
                                                // }


                                                println!("selected files: {:?}", self.multi_pipeline_tracker.keys());
                                            }
                                        }
                                    })
                            });


                        });

                        if ui.add_sized((100.0, 35.0), Button::new("reset query")).clicked() {
                            self.csvqb_pipelines.clear();
                            self.multi_pipeline_tracker.clear();
                            self.graph_data.clear();
                            if !self.sessions[self.current_session as usize].pipelines.is_empty(){
                               self.sessions[self.current_session as usize].pipelines.clear();
                            }
                        }

                        if ui.add_sized((100.0, 35.0), Button::new("Execute Expression")).clicked() {
                            self.graph_data.clear();
                            let selected_files = self.multi_pipeline_tracker.keys().copied().sorted().collect();
                            for pipe in self.csvqb_pipelines.iter() {
                                for fields in pipe.iter() {
                                    let result = process_csvqb_pipeline(&*fields.1, &selected_files, &self.csv_files);
                                    if !result.is_empty() {
                                        self.graph_data.push(result);
                                    }
                                }

                            }
                        }

                        if ui.add_sized((100.0, 35.0), Button::new("view chart")).clicked() {
                            self.screen = Screen::ViewChart;
                        }
                        ui.add_space(ui.available_width());
                    });
                });

            let mut csv_columns: Vec<(usize, Vec<String>)> = Vec::new();
            for index in self.multi_pipeline_tracker.keys() {
                println!("CSVCOL index: {}", index);
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
                        for (index, fields) in csv_columns.iter() {
                            let _index = index;
                            if self.multi_pipeline_tracker.contains_key(&index) {
                                println!("multi pipeline tracker: {:?}", self.multi_pipeline_tracker);
                                for (index,pipeline_index) in self.multi_pipeline_tracker.get(&index).expect("failed").iter().enumerate() {
                                    println!("pipeline index: {}:{:?}",_index, pipeline_index);
                                    println!("csvqb_pipelines {:?}", self.csvqb_pipelines);
                                    ui.heading(RichText::new(format!("{} #{}", self.csv_files[*pipeline_index].0
                                        .split("\\")
                                        .last()
                                        .unwrap_or("No file name"), index + 1))
                                        .color(Color32::BLACK)
                                    );

                                    ui.push_id(index , |ui| {
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
                                            // while self.csvqb_pipelines.len() < _index {
                                            //     self.csvqb_pipelines.push(Vec::new());
                                            // }
                                            // I think this functionality needs to move to add pipeline
                                            while self.csvqb_pipelines[*pipeline_index].len() <= index {
                                                self.csvqb_pipelines[*pipeline_index].push((index, Vec::new()));
                                            }

                                            self.csvqb_pipelines[*pipeline_index][index].1 = pipeline_str
                                                .split_whitespace()
                                                .map(String::from)
                                                .collect();
                                        }
                                    });
                                    ui.push_id(index , |ui| {
                                        ui.group(|ui| {
                                            ui.set_min_size(Vec2::new(ui.available_width()/3.0, 100.0));
                                            ScrollArea::both()
                                                .max_height(100.0)
                                                .max_width(ui.available_width()/3.0)
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
                                    });
                                    ui.push_id(index , |ui| {
                                        ui.group(|ui| {
                                            ui.set_min_size(Vec2::new(300.0, 33.0));
                                            ScrollArea::both()
                                                .max_height(100.0)
                                                .max_width(300.0)
                                                .show(ui, |ui| {
                                                    ui.horizontal_wrapped(|ui| {
                                                        if ui.button("(").clicked() {
                                                            if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len()-1 >= *pipeline_index {
                                                                if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                    pipeline.1.push("(".to_string());
                                                                }
                                                            } else {
                                                                self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["(".to_string()])]);
                                                            }
                                                        }
                                                        if ui.button(")").clicked() {
                                                            if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len()-1 >= *pipeline_index {
                                                                if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                    pipeline.1.push(")".to_string());
                                                                }
                                                            } else {
                                                                self.csvqb_pipelines.push(vec![(*pipeline_index, vec![")".to_string()])]);
                                                            }
                                                        }
                                                        if ui.button("GRP").clicked() {
                                                            if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len()-1 >= *pipeline_index {
                                                                if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                    pipeline.1.push("GRP".to_string());
                                                                }
                                                            } else {
                                                                self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["GRP".to_string()])]);
                                                            }
                                                        }
                                                        if ui.button("CSUM").clicked() {
                                                            if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len()-1 >= *pipeline_index {
                                                                if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                    pipeline.1.push("CSUM".to_string());
                                                                }
                                                            } else {
                                                                self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["CSUM".to_string()])]);
                                                            }
                                                        }
                                                        if ui.button("CAVG").clicked() {
                                                            if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len()-1 >= *pipeline_index {
                                                                if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                    pipeline.1.push("CAVG".to_string());
                                                                }
                                                            } else {
                                                                self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["CAVG".to_string()])]);
                                                            }
                                                        }
                                                        if ui.button("CCOUNT").clicked() {
                                                            if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len()-1 >= *pipeline_index {
                                                                if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                    pipeline.1.push("CCOUNT".to_string());
                                                                }
                                                            } else {
                                                                self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["CCOUNT".to_string()])]);
                                                            }
                                                        }
                                                        if ui.button("MUL").clicked() {
                                                            if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len()-1 >= *pipeline_index {
                                                                if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                    pipeline.1.push("MUL".to_string());
                                                                }
                                                            } else {
                                                                self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["MUL".to_string()])]);
                                                            }
                                                        }
                                                        if ui.button("=").clicked() {
                                                            if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len()-1 >= *pipeline_index {
                                                                if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                    pipeline.1.push("=".to_string());
                                                                }
                                                            } else {
                                                                self.csvqb_pipelines.push(vec![(*pipeline_index, vec!["=".to_string()])]);
                                                            }
                                                        }
                                                        if ui.button(">").clicked() {
                                                            if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len()-1 >= *pipeline_index {
                                                                if let Some(pipeline) = self.csvqb_pipelines[*pipeline_index].get_mut(index) {
                                                                    pipeline.1.push(">".to_string());
                                                                }
                                                            } else {
                                                                self.csvqb_pipelines.push(vec![(*pipeline_index, vec![">".to_string()])]);
                                                            }
                                                        }
                                                        if ui.button("<").clicked() {
                                                            if self.csvqb_pipelines.len() > 0 && self.csvqb_pipelines.len()-1 >= *pipeline_index {
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
                                    });
                                }
                            }


                            ui.push_id(index, |ui| {
                                if ui.button("add pipeline").clicked(){

                                    if self.multi_pipeline_tracker.contains_key(&index)  {
                                        let _ = self.multi_pipeline_tracker.get_mut(&index).expect("REASON").push(*index);
                                    }

                                    // if !self.multi_pipeline_tracker.contains_key(&index) {
                                    //     self.multi_pipeline_tracker.insert(_index,vec![]);
                                    // }


                                    println!("pipe tracker: {:?}", self.multi_pipeline_tracker);
                                };
                            });
                            ui.add_space(35.0);
                        }
                    });
                });
                ui.vertical_centered_justified(|ui| {
                    ui.heading(RichText::new("Pipeline output").color(Color32::BLACK));
                    let expression_data = &self.graph_data;
                    ScrollArea::vertical().show(ui, |ui: &mut Ui| {
                        for row in expression_data {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(format!("{:?}", row)).color(Color32::BLACK));
                            });
                            ui.add_space(12.0);
                        }
                    });
                })
            });
        });
    }

    fn show_chart_screen(&mut self, ctx: &Context) {
        let frame = egui::Frame::default()
            .fill(Color32::from_rgb(211, 211, 211));

        CentralPanel::default().frame(frame).show(ctx, |ui| {

            egui::Frame::NONE
                .fill(Color32::from_rgb(192, 192, 192))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        if ui.add_sized((100.0, 35.0), Button::new("Home")).clicked() {
                            self.screen = Screen::Main;
                        }
                        if ui.add_sized((100.0, 35.0), Button::new("Explorer")).clicked() {
                            self.screen = Screen::CreateChart;
                        }

                        ui.add_space(ui.available_width());
                    })
                });

            ui.add_space(21.0);

            ui.horizontal(|ui| {
                egui::ComboBox::from_label("Select Chart")
                    .selected_text(&self.chart_style_prototype)
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut self.chart_style_prototype, "Bar Graph".to_string(), "Bar Graph").clicked() {}
                        if ui.selectable_value(&mut self.chart_style_prototype, "Histogram".to_string(), "Histogram").clicked() {}
                        if ui.selectable_value(&mut self.chart_style_prototype, "Pie Chart".to_string(), "Pie Chart").clicked() {}
                        if ui.selectable_value(&mut self.chart_style_prototype, "Scatter Plot".to_string(), "Scatter Plot").clicked() {}
                        if ui.selectable_value(&mut self.chart_style_prototype, "Line Chart".to_string(), "Line Chart").clicked() {}
                        if ui.selectable_value(&mut self.chart_style_prototype, "Flame Graph".to_string(), "Flame Graph").clicked() {}
                    });
            });

            ScrollArea::both().show(ui, |ui| {
                for graph_query in self.graph_data.iter().skip(1) {
                    println!("{:?}", graph_query);
                    let formatted_data = Some(format_graph_query(graph_query.clone()));
                    match self.chart_style_prototype.as_str() {
                        "Bar Graph" => {
                            let _ = draw_bar_graph(ui, formatted_data);
                        }
                        "Pie Chart" => {
                            let _ = draw_pie_chart(ui, formatted_data);
                        }
                        "Histogram" => {
                            let _ = draw_histogram(ui, formatted_data);
                        }
                        "Scatter Plot" => {
                            let _ = draw_scatter_plot(ui, formatted_data);
                        }
                        "Line Chart" => {
                            let _ = draw_line_chart(ui, formatted_data);
                        }
                        "Flame Graph" => {
                            let _ = draw_flame_graph(ui, formatted_data);
                        }
                        _ => {}
                    }
                }
            });


            if ui.button("Export Chart").clicked() {
                // todo Billy
            }
        });
    }
}


