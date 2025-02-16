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

struct CharterCsv {
    screen: Screen,
    csv_files: Vec<(String, String)>,
    graph_data: Option<String>,
}

impl Default for CharterCsv {
    fn default() -> Self {
        Self {
            screen: Screen::Main,
            csv_files: vec![("test.csv".to_string(), "Some val, Some val, Some, val..".to_string())],
            graph_data: None,
        }
    }
}

enum Screen {
    Main,
    ViewCsv,
    CreateCsv { content: (String, String) },
    EditCsv { index: usize, content: (String, String) },
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
                            self.csv_files.push((path_as_string, content));
                        }
                    }
                }

                if ui.add_sized(menu_btn_size, egui::Button::new("View All CSV Files")).clicked() {
                    self.screen = Screen::ViewCsv;
                }

                if ui.add_sized(menu_btn_size, egui::Button::new("Create New CSV File")).clicked() {
                    self.screen = Screen::CreateCsv {
                        content: ("/files".to_string(), String::new()),
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

    fn show_csv_editor(&mut self, ctx: &egui::Context, content: &mut (String, String), edit_index: Option<usize>) -> Option<Screen> {
        let mut next_screen = None;

        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    if let Some(index) = edit_index {
                        self.csv_files[index] = content.clone();
                    } else {
                        self.csv_files.push(("filename".to_string(), "contents".to_string()));
                    }
                    next_screen = Some(Screen::Main);
                }

                if ui.button("Back").clicked() {
                    next_screen = Some(Screen::Main);
                }
            });

            ScrollArea::both().show(ui, |ui| {
                let rows: Vec<Vec<String>> = content
                    .1
                    .lines()
                    .map(|line| line.split(',').map(String::from).collect())
                    .collect();
                for row in rows {
                    ui.horizontal(|ui| {
                        for cell in row {
                            let mut cell_content = cell.clone();
                            ui.add_sized(Vec2::new(300.0, 0.0), egui::TextEdit::singleline(&mut cell_content));
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