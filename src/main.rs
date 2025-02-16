use eframe::{egui, App};
use eframe::emath::Vec2;
use egui::{CentralPanel, ScrollArea};

fn main() {
    let ctx = egui::Context::default();
    let mut size = ctx.used_size();
    size.x = 1300.00;
    size.y = 720.00;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_resizable(true)
            .with_inner_size(size),
        ..Default::default()
    };
    eframe::run_native(
        "CharterCSV",
        options,
        Box::new(|_cc| Ok(Box::new(CharterCsv::default()))),
    )
        .expect("Unexpected error in running the application");
}

type CsvGrid = Vec<Vec<String>>;
struct CharterCsv {
    screen: Screen,
    csv_files: Vec<(String, CsvGrid)>,
    graph_data: Option<String>,
}

impl Default for CharterCsv {
    fn default() -> Self {
        Self {
            screen: Screen::Main,
            csv_files: vec![(
                "product_sheet.csv".to_string(),
                vec![
                    vec!["id".to_string(), "product_name".to_string(), "qty".to_string(), "price".to_string()]
                ]
            )],
            graph_data: None,
        }
    }
}

enum Screen {
    Main,
    ViewCsv,
    CreateCsv { content: (String, CsvGrid) },
    EditCsv { index: usize, content: (String, CsvGrid) },
    ViewGraph { csv_file: String },
}

impl App for CharterCsv {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let screen = std::mem::replace(&mut self.screen, Screen::Main);
        match screen {
            Screen::Main => {
                self.screen = screen;
                self.show_main_screen(ctx, _frame)
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
            Screen::ViewGraph { ref csv_file } => {
                self.screen = Screen::Main;
                self.show_graph_screen(ctx, &csv_file)
            }
        }
    }
}

impl CharterCsv {
    fn show_main_screen(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Charter CSV");
                ui.label("Create charts from CSV files for analysis");
                ui.add_space(20.0);
                let menu_btn_size = egui::Vec2::new(300.0, 30.0);
                if ui.add_sized(menu_btn_size, egui::Button::new("Upload CSV File")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("CSV files", &["csv"]).pick_file() {
                        let path_as_string = path.to_str().unwrap().to_string();
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            // Convert the content to grid immediately
                            let grid: CsvGrid = content
                                .lines()
                                .map(|line| line.split(',')
                                    .map(|s| s.trim().to_string())
                                    .collect())
                                .collect();
                            self.csv_files.push((path_as_string, grid));
                        }
                    }
                }

                if ui.add_sized(menu_btn_size, egui::Button::new("View All CSV Files")).clicked() {
                    self.screen = Screen::ViewCsv;
                }

                if ui.add_sized(menu_btn_size, egui::Button::new("Create New CSV File")).clicked() {
                    self.screen = Screen::CreateCsv {
                        content: (
                            "/files".to_string(),
                            vec![vec!["".to_string()]],
                        ),
                    };
                }

                if ui.add_sized(menu_btn_size, egui::Button::new("Close Program")).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }

    fn show_csv_list(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                for (index, file) in self.csv_files.iter().enumerate() {
                    let file_name = file.0.split("\\").last().unwrap_or("No file name");
                    ui.horizontal(|ui| {
                        if ui.button("Edit").clicked() {
                            self.screen = Screen::EditCsv {
                                index,
                                content: file.clone(),
                            };
                        }

                        ui.label(file_name);
                    });
                }
            });

            if ui.button("Back").clicked() {
                self.screen = Screen::Main;
            }
        });
    }

    fn show_csv_editor(&mut self, ctx: &egui::Context, content: &mut (String, CsvGrid), edit_index: Option<usize>, ) -> Option<Screen> {
        let mut next_screen = None;
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    if let Some(index) = edit_index {
                        self.csv_files[index] = content.clone();
                    } else {
                        self.csv_files.push(content.clone());
                    }
                    next_screen = Some(Screen::Main);
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

            ScrollArea::both().show(ui, |ui| {
                let grid = &mut content.1;
                for row in grid.iter_mut() {
                    ui.horizontal(|ui| {
                        for cell in row.iter_mut() {
                            ui.add_sized(
                                Vec2::new(300.0, 0.0),
                                egui::TextEdit::singleline(cell)
                            );
                        }
                    });
                }
            });
        });

        next_screen
    }

    fn show_graph_screen(&mut self, ctx: &egui::Context, csv_file: &str) {
        CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("Graph view for: {}", csv_file));
            ui.add_space(20.0);

            ui.label("graph...");

            if ui.button("Export to PNG").clicked() {
                // todo
            }

            if ui.button("Back").clicked() {
                self.screen = Screen::ViewCsv;
            }
        });
    }
}