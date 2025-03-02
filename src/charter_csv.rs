use eframe::App;
use egui::{Ui, pos2, vec2, Align2, Button, CentralPanel, Color32, Context, FontId, IconData, Image, Rect, RichText, ScrollArea, Sense, TextEdit, TextureHandle, Vec2, ViewportCommand};
use crate::charter_utilities::{csv2grid, draw_rotated_text, grid2csv, CsvGrid};
use crate::csvqb::{process_csvqb_pipeline, Value};
pub use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use image::ImageReader;
use image::GenericImageView;


pub struct CharterCsv {
    texture: Option<TextureHandle>,
    pub screen: Screen,
    pub csv_files: Vec<(String, CsvGrid)>,
    pub selected_csv_files: Vec<usize>,
    pub csvqb_pipeline: Vec<Vec<String>>,
    pub graph_data: Vec<Value>,
    pub file_receiver: Receiver<(String, Vec<Vec<String>>)>,
    pub file_sender: Sender<(String, Vec<Vec<String>>)>,
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
    label: String,
    value: f64,
}

impl Default for CharterCsv {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            texture: None,
            screen: Screen::Main,
            csv_files: vec![(
                "product_sheet.csv".to_string(),
                vec![
                    vec!["id".to_string(), "product_name".to_string(), "qty".to_string(), "price".to_string()]
                ]
            )],
            selected_csv_files: vec![],
            csvqb_pipeline: vec![],
            graph_data: vec![],
            file_receiver: rx,
            file_sender: tx,
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

impl App for CharterCsv {
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
    }
}
impl CharterCsv {
    fn show_main_screen(&mut self, ctx: &Context) {
        let frame = egui::Frame::default()
            .fill(egui::Color32::from_rgb(67, 143, 173));

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
                                let color_image = egui::ColorImage::new([16, 16], egui::Color32::RED);
                                ctx.load_texture(
                                    "error_placeholder",
                                    color_image,
                                    egui::TextureOptions::default(),
                                )
                            }
                        }
                    }
                    _ => {
                        let color_image = egui::ColorImage::new([16, 16], egui::Color32::RED);
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
                ui.vertical_centered(|ui| {});
                ui.min_rect().height()
            }).inner;

            let top_margin: f32 = 25.0;
            ui.add_space(top_margin.max(0.0));

            ui.vertical_centered(|ui| {
                ui.add(
                    Image::new(&*texture)
                        .max_width(200.0)
                );
                ui.add_space(20.0);
                ui.heading(RichText::new("Charter CSV").color(egui::Color32::BLACK));
                ui.label(RichText::new("navigate your data with speed and precision").color(egui::Color32::BLACK));
                ui.add_space(20.0);

                let menu_btn_size = Vec2::new(300.0, 30.0);
                if ui.add_sized(menu_btn_size, Button::new("load CSV Files")).clicked() {
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

                if ui.add_sized(menu_btn_size, Button::new("View All CSV Files")).clicked() {
                    self.screen = Screen::ViewCsv;
                }

                if ui.add_sized(menu_btn_size, Button::new("Create New CSV File")).clicked() {
                    self.screen = Screen::CreateCsv {
                        content: (
                            "/todo/setpath".to_string(),
                            vec![vec!["".to_string()]],
                        ),
                    };
                }

                if ui.add_sized(menu_btn_size, Button::new("Create Chart")).clicked() {
                    self.screen = Screen::CreateChart;
                }

                if ui.add_sized(menu_btn_size, Button::new("View All Charts")).clicked() {
                    self.screen = Screen::ViewChart;
                }

                if ui.add_sized(menu_btn_size, Button::new("Close Program")).clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            });
        });
    }

    fn show_csv_list(&mut self, ctx: &Context) {
        let frame = egui::Frame::default()
            .fill(egui::Color32::from_rgb(67, 143, 173));
        let mut files_to_remove: Option<usize> = None;
        let mut next_screen: Option<Screen> = None;

        CentralPanel::default().frame(frame).show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                for (index, file) in self.csv_files.iter().enumerate() {
                    let file_name = file.0.split("\\").last().unwrap_or("No file name");
                    ui.horizontal(|ui| {
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
                    });
                }
            });

            if ui.button("Back").clicked() {
                next_screen = Some(Screen::Main);
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
            .fill(egui::Color32::from_rgb(67, 143, 173));
        let mut next_screen = None;
        CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
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
                    content.1.push(vec!["".to_string(); content.1.get(0).map_or(0, |row| row.len())]);
                }

                if ui.button("Add Column").clicked() {
                    for row in &mut content.1 {
                        row.push("".to_string());
                    }
                }

                if ui.button("Back").clicked() {
                    next_screen = Some(Screen::Main);
                }
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
            .fill(egui::Color32::from_rgb(67, 143, 173));
        CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                if ui.button("Back").clicked() {
                    self.screen = Screen::Main;
                }
                ui.menu_button("Files", |ui| {
                    ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (index, file) in self.csv_files.iter().enumerate() {
                            let file_name = &file.0;
                            let mut selected = self.selected_csv_files.iter().any(|(f)| f == &index);

                            if ui.checkbox(&mut selected, file_name).clicked() {
                                if selected {
                                    self.selected_csv_files.push(index);
                                } else {
                                    self.selected_csv_files.retain(|(f)| f != &index);
                                }
                            }
                        }
                    });
                });
                if ui.button("reset query").clicked() {
                    self.csvqb_pipeline.clear();
                }
                if ui.button("Execute Expression").clicked() {
                    for fields in self.csvqb_pipeline.iter() {
                        let result = process_csvqb_pipeline(fields, &self.selected_csv_files, &self.csv_files);
                        println!("Result: {:?}", &result);;
                        if !result.is_empty() {
                            println!("Result: {:?}", &result);
                            self.graph_data = result;
                        }
                    }
                }
                if ui.button("view chart").clicked() {
                    self.screen = Screen::ViewChart;
                }
            });

            ui.add_space(20.0);


            let mut csv_columns: Vec<Vec<String>> = Vec::new();
            for (index) in self.selected_csv_files.iter() {
                if let Some(csv_file) = self.csv_files.get(*index) {
                    let column_titles = csv_file.1
                        .get(0)
                        .map(|row| row.clone())
                        .unwrap_or_default();
                    csv_columns.push(column_titles);
                }
            }
            ui.add_space(35.0);
            ui.horizontal(|ui| {
                ui.style_mut().spacing.indent = 30.0;
                ui.vertical(|ui| {
                    ui.indent("left_margin", |ui| {
                        for (index, fields) in csv_columns.iter().enumerate() {
                            ui.heading(RichText::new(format!("{}, query #{}", self.csv_files[index].0.split("\\").last().unwrap_or("No file name"), index + 1)).color(Color32::BLACK));
                            ui.push_id(index, |ui| {
                                let mut pipeline_str = self.csvqb_pipeline.get(index)
                                    .map(|pipeline| pipeline.join(" "))
                                    .unwrap_or_default();

                                if ui.text_edit_singleline(&mut pipeline_str).changed() {
                                    self.csvqb_pipeline.insert(index, pipeline_str.split_whitespace().map(String::from).collect());
                                }
                            });
                            ui.push_id(index, |ui| {
                                ui.group(|ui| {
                                    ui.set_min_size(Vec2::new(300.0, 100.0));
                                    ScrollArea::both()
                                        .max_height(100.0)
                                        .max_width(300.0)
                                        .show(ui, |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                for field in fields.iter() {
                                                    if ui.button(field).clicked() {
                                                        if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                                                            self.csvqb_pipeline[index].push(field.to_string());
                                                        } else {
                                                            self.csvqb_pipeline.push(vec![field.to_string()]);
                                                        }

                                                    }
                                                }
                                            });
                                        });
                                });
                            });
                            ui.push_id(index, |ui| {
                                ui.group(|ui| {
                                    ui.set_min_size(Vec2::new(300.0, 33.0));
                                    ScrollArea::both()
                                        .max_height(100.0)
                                        .max_width(300.0)
                                        .show(ui, |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                if ui.button("(").clicked() {
                                                    if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                                                        self.csvqb_pipeline[index].push("(".to_string());
                                                    } else {
                                                        self.csvqb_pipeline.push(vec!["(".to_string()]);
                                                    }
                                                }
                                                if ui.button(")").clicked() {
                                                    if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                                                        self.csvqb_pipeline[index].push(")".to_string());
                                                    } else {
                                                        self.csvqb_pipeline.push(vec![")".to_string()]);
                                                    }
                                                }
                                                if ui.button("GRP").clicked() {
                                                    if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                                                        self.csvqb_pipeline[index].push("GRP".to_string());
                                                    } else {
                                                        self.csvqb_pipeline.push(vec!["GRP".to_string()]);
                                                    }
                                                }
                                                if ui.button("CSUM").clicked() {
                                                    if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                                                        self.csvqb_pipeline[index].push("CSUM".to_string());
                                                    } else {
                                                        self.csvqb_pipeline.push(vec!["CSUM".to_string()]);
                                                    }
                                                }
                                                if ui.button("CAVG").clicked() {
                                                    if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                                                        self.csvqb_pipeline[index].push("CAVG".to_string());
                                                    } else {
                                                        self.csvqb_pipeline.push(vec!["CAVG".to_string()]);
                                                    }
                                                }
                                                if ui.button("CCOUNT").clicked() {
                                                    if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                                                        self.csvqb_pipeline[index].push("CCOUNT".to_string());
                                                    } else {
                                                        self.csvqb_pipeline.push(vec!["CCOUNT".to_string()]);
                                                    }
                                                }
                                                if ui.button("MUL").clicked() {
                                                    if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                                                        self.csvqb_pipeline[index].push("MUL".to_string());
                                                    } else {
                                                        self.csvqb_pipeline.push(vec!["MUL".to_string()]);
                                                    }
                                                }
                                                if ui.button("=").clicked() {
                                                    if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                                                        self.csvqb_pipeline[index].push("=".to_string());
                                                    } else {
                                                        self.csvqb_pipeline.push(vec!["=".to_string()]);
                                                    }
                                                }
                                                if ui.button(">").clicked() {
                                                    if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                                                        self.csvqb_pipeline[index].push(">".to_string());
                                                    } else {
                                                        self.csvqb_pipeline.push(vec![">".to_string()]);
                                                    }
                                                }
                                                if ui.button("<").clicked() {
                                                    if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                                                        self.csvqb_pipeline[index].push("<".to_string());
                                                    } else {
                                                        self.csvqb_pipeline.push(vec!["<".to_string()]);
                                                    }
                                                }
                                            });
                                        });
                                });
                            });
                            ui.add_space(35.0);
                        }
                    });
                });
                ui.vertical_centered_justified(|ui| {
                    ui.heading(RichText::new("Pipeline output").color(Color32::BLACK));
                    let formatted_data = &self.graph_data;  
                    ScrollArea::vertical().show(ui, |ui: &mut Ui| {
                        for row in formatted_data {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(format!("{:?}", row)).color(Color32::BLACK));
                            });
                        }
                    });
                })
            });
        });
    }

    fn show_chart_screen(&mut self, ctx: &Context) {
        let frame = egui::Frame::default()
            .fill(egui::Color32::from_rgb(67, 143, 173));

        CentralPanel::default().frame(frame).show(ctx, |ui| {
            if ui.button("Back").clicked() {
                self.screen = Screen::CreateChart;
            }
            let mut formatted_data = Some(self.format_graph_query());
            ScrollArea::horizontal().show(ui, |ui|{
                if let Some(graph_data) = formatted_data {
                    let available_width = ui.available_width() * (ui.available_width() / graph_data.len() as f32);
                    let available_height:f64 = 600.0;
                    let bar_spacing = 2.0;
                    let values: Vec<f64> = graph_data.iter()
                        .map(|data| data.value)
                        .collect();
                    let max_value = values.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&1.0);
                    let bar_width = (ui.available_width() / graph_data.len() as f32);
                    let (response, painter) = ui.allocate_painter(
                        vec2(available_width, (available_height + 40.0) as f32),
                        Sense::hover(),
                    );
                    let rect = response.rect;
                    painter.text(
                        pos2(rect.min.x - 40.0, rect.min.y + (available_height / 2.0) as f32),
                        Align2::CENTER_CENTER,
                        "Count", // todo make dynamic labels
                        FontId::default(),
                        Color32::BLACK,
                    );
                    for (i, (data, value)) in graph_data.iter().zip(values.iter()).enumerate() {
                        let value_normalized = value / max_value;
                        let height = value_normalized * available_height;
                        let x = rect.min.x + (i as f32 * (bar_width + bar_spacing));
                        let bar_rect = Rect::from_min_size(
                            pos2(x, rect.max.y - (height - 20.0) as f32),
                            vec2(bar_width, height as f32),
                        );
                        painter.rect_filled(bar_rect, 0.0, Color32::from_rgb(65, 155, 220));
                        painter.text(
                            pos2(x + bar_width / 2.0, bar_rect.min.y - 5.0),
                            Align2::CENTER_BOTTOM,
                            format!("{:.0}", value),
                            FontId::default(),
                            Color32::BLACK,
                        );
                        let shapes = draw_rotated_text(&painter, rect, &data.label, x, bar_width);
                        ui.painter_at(rect).extend(shapes);
                    }
                }
            });

            if ui.button("Export Chart").clicked() {
                // todo Billy
            }
        });
    }

    // experimenting graph data
    fn format_graph_query(&self) -> Vec<PlotPoint> {
        let mut plot_data: Vec<PlotPoint> = Vec::new();

        let mut i = 0;
        while i < self.graph_data.len() {
            match &self.graph_data[i] {
                Value::Number(num) => {
                    if i + 1 < self.graph_data.len() {
                        if let Value::Field(label) = &self.graph_data[i + 1] {
                            plot_data.push(PlotPoint {
                                label: label.clone(),
                                value: *num,
                            });
                            i += 2;
                        } else {
                            println!("{}", "Expected Field after Number".to_string());
                        }
                    } else {
                        println!("{}", "Incomplete data: Number without a corresponding Field".to_string());
                    }
                }
                Value::QueryResult(query_result) => {
                    if query_result.is_empty() {
                        println!("{}", "QueryResult is empty".to_string());
                    }

                    let headers = &query_result[0];
                    for row in query_result.iter().skip(1) {
                        if row.len() < headers.len() {
                            println!("{}", "Mismatch in row and column sizes in QueryResult".to_string());
                        }

                        let label = row[..row.len() - 1].join(" ");
                        if let Ok(last_value) = row.last().unwrap().parse::<f64>() {
                            plot_data.push(PlotPoint {
                                label,
                                value: last_value,
                            });
                        } else {
                            println!("{}", "Failed to parse last column value as a number in QueryResult".to_string());
                        }
                    }
                }
                _ => {}
            }

            i += 1;
        }

        // todo experimental labeling
        // let axis_labels = if let Some(Value::QueryResult(query_result)) = self.graph_data.iter().find(|v| matches!(v, Value::QueryResult(_))) {
        //     if !query_result.is_empty() {
        //         let headers = &query_result[0];
        //         if headers.len() > 1 {
        //             (headers[..headers.len() - 1].join(" "), headers.last().unwrap().to_string())
        //         } else {
        //             ("X".to_string(), "Y".to_string()) // todo let user define x, y
        //         }
        //     } else {
        //         ("X".to_string(), "Y".to_string()) // Default if malformed QueryResult
        //     }
        // } else {
        //     ("X".to_string(), "Y".to_string()) // Default if no QueryResult is present
        // };

        // // Mock visual
        // println!("Plotting graph with:");
        // println!("X-Axis Label: {}", axis_labels.0);
        // println!("Y-Axis Label: {}", axis_labels.1);
        //
        // for point in &plot_data {
        //     println!("Label: {}, Value: {}", point.label, point.value);
        // }

        return plot_data;
    }
}