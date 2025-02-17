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
type ChartGrid = Vec<Vec<(String, f32)>>;
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
    CreateChart,
    ViewChart,
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
            Screen::CreateChart => {
                self.screen = screen;
                self.create_chart_screen(ctx);
            }
            Screen::ViewChart => {
                self.screen = screen;
                self.show_chart_screen(ctx, &ChartGrid::default())
            }
        }
    }
}

impl CharterCsv {
    fn csv2grid(content: &str) -> CsvGrid {
        content
            .lines()
            .map(|line| line.split(',')
                .map(|s| s.trim().to_string())
                .collect())
            .collect()
    }
    fn grid2csv(grid: &CsvGrid) -> String {
        grid.iter()
            .map(|row| row.join(","))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn show_main_screen(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Charter CSV");
                ui.label("Create charts from CSV files for analysis");
                ui.add_space(20.0);
                let menu_btn_size = egui::Vec2::new(300.0, 30.0);
                if ui.add_sized(menu_btn_size, egui::Button::new("load CSV Files")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("CSV files", &["csv"]).pick_file() {
                        let path_as_string = path.to_str().unwrap().to_string();
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let grid: CsvGrid = CharterCsv::csv2grid(&content);
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
                            "/todo/setpath".to_string(),
                            vec![vec!["".to_string()]],
                        ),
                    };
                }

                if ui.add_sized(menu_btn_size, egui::Button::new("Create Chart")).clicked() {
                    self.screen = Screen::CreateChart;
                }

                if ui.add_sized(menu_btn_size, egui::Button::new("View All Charts")).clicked() {
                    self.screen = Screen::ViewChart;
                }

                if ui.add_sized(menu_btn_size, egui::Button::new("Close Program")).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }

    fn show_csv_list(&mut self, ctx: &egui::Context) {
        let mut files_to_remove: Option<usize> = None;
        let mut next_screen: Option<Screen> = None;

        CentralPanel::default().show(ctx, |ui| {
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

        // Handle state changes after the UI
        if let Some(index) = files_to_remove {
            self.csv_files.remove(index);
        }
        if let Some(screen) = next_screen {
            self.screen = screen;
        }
    }

    fn show_csv_editor(&mut self, ctx: &egui::Context, content: &mut (String, CsvGrid), edit_index: Option<usize>) -> Option<Screen> {
        let mut next_screen = None;
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    if let Some(index) = edit_index {
                        self.csv_files[index] = content.clone();
                    } else {
                        self.csv_files.push(content.clone());
                    }

                    if let Some(path) = rfd::FileDialog::new().add_filter(&content.0, &["csv"]).save_file() {
                        let csv_content = CharterCsv::grid2csv(&content.1);
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

                    // Only render valid rows
                    for row_idx in start_row..end_row {
                        let row = &mut grid[row_idx];
                        ui.horizontal(|ui| {
                            if start_col > 0 {
                                ui.add_space(start_col as f32 * CELL_WIDTH);
                            }
                            // Render visible cells
                            for col_idx in start_col..end_col {
                                if col_idx < row.len() {
                                    let cell = &mut row[col_idx];
                                    ui.add_sized(
                                        Vec2::new(CELL_WIDTH, ROW_HEIGHT),
                                        egui::TextEdit::singleline(cell)
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

    fn create_chart_screen(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            if ui.button("Back").clicked() {
                self.screen = Screen::Main;
            }
            ui.label("Step 1. Select CSV files:".to_string());
            ui.add_space(20.0);
            ui.label("Step 2. Select fields to chart:".to_string());
            ui.add_space(20.0);
            ui.label("fields...");
            ui.add_space(20.0);
            ui.label("Step 3. Organize comparisons:".to_string());
            ui.add_space(20.0);
            ui.label("Step 4. Select chart type to fit data:".to_string());

            if ui.button("Export Chart").clicked() {
                // todo
            }
        });
    }

    fn show_chart_screen(&mut self, ctx: &egui::Context, chart: &ChartGrid) {
        CentralPanel::default().show(ctx, |ui| {
            if ui.button("Back").clicked() {
                self.screen = Screen::Main;
            }
            ui.label("charts...");
        });
    }
}