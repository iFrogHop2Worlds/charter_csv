use std::fmt::format;
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
    selected_csv_files: Vec<usize>,
    fields_2_compare: Vec<Vec<String>>,
    graph_data: Option<String>,
}
#[derive(Debug, Clone)]
enum Value {
    Bool(bool),
    Number(f64),
    Text(String),
    Field(String),
    Results(Vec<(String, f64)>) // For storing column data
}

#[derive(Debug)]
enum Operator {
    Sum,
    Avg,
    Count,
    GroupBy,
    Equals,
    GreaterThan,
    LessThan,
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
            selected_csv_files: vec![],
            fields_2_compare: vec![],
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

    // Aggregation Operators
    fn sum(&self, column: &str, group_by: Option<&[String]>) -> Vec<(String, f64)> {
        let mut result = std::collections::HashMap::new();

        for &file_idx in &self.selected_csv_files {
            if let Some((_, grid)) = self.csv_files.get(file_idx) {
                if grid.is_empty() { continue; }

                // Find column index
                let headers = &grid[0];
                let col_idx = match headers.iter().position(|h| h == column) {
                    Some(idx) => idx,
                    None => continue,
                };

                // Process data rows
                for row in grid.iter().skip(1) {
                    if row.len() <= col_idx { continue; }

                    let key = match group_by {
                        Some(group_cols) => {
                            let mut key = String::new();
                            for group_col in group_cols {
                                if let Some(idx) = headers.iter().position(|h| h == group_col) {
                                    if let Some(val) = row.get(idx) {
                                        key.push_str(val);
                                        key.push('|');
                                    }
                                }
                            }
                            key
                        }
                        None => "total".to_string(),
                    };

                    if let Ok(value) = row[col_idx].parse::<f64>() {
                        *result.entry(key).or_insert(0.0) += value;
                    }
                }
            }
        }

        result.into_iter().collect()
    }

    fn average(&self, column: &str, group_by: Option<&[String]>) -> Vec<(String, f64)> {
        let mut sums = std::collections::HashMap::new();
        let mut counts = std::collections::HashMap::new();

        for &file_idx in &self.selected_csv_files {
            if let Some((_, grid)) = self.csv_files.get(file_idx) {
                if grid.is_empty() { continue; }

                let headers = &grid[0];
                let col_idx = match headers.iter().position(|h| h == column) {
                    Some(idx) => idx,
                    None => continue,
                };

                for row in grid.iter().skip(1) {
                    if row.len() <= col_idx { continue; }

                    let key = match group_by {
                        Some(group_cols) => {
                            let mut key = String::new();
                            for group_col in group_cols {
                                if let Some(idx) = headers.iter().position(|h| h == group_col) {
                                    if let Some(val) = row.get(idx) {
                                        key.push_str(val);
                                        key.push('|');
                                    }
                                }
                            }
                            key
                        }
                        None => "total".to_string(),
                    };

                    if let Ok(value) = row[col_idx].parse::<f64>() {
                        *sums.entry(key.clone()).or_insert(0.0) += value;
                        *counts.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }

        sums.into_iter()
            .filter_map(|(key, sum)| {
                counts.get(&key).map(|&count| {
                    (key, sum / count as f64)
                })
            })
            .collect()
    }

    fn count(&self, column: &str, group_by: Option<&[String]>) -> Vec<(String, i32)> {
        let mut counts = std::collections::HashMap::new();

        for &file_idx in &self.selected_csv_files {
            if let Some((_, grid)) = self.csv_files.get(file_idx) {
                if grid.is_empty() { continue; }

                let headers = &grid[0];
                let col_idx = match headers.iter().position(|h| h == column) {
                    Some(idx) => idx,
                    None => continue,
                };

                for row in grid.iter().skip(1) {
                    if row.len() <= col_idx { continue; }

                    let key = match group_by {
                        Some(group_cols) => {
                            let mut key = String::new();
                            for group_col in group_cols {
                                if let Some(idx) = headers.iter().position(|h| h == group_col) {
                                    if let Some(val) = row.get(idx) {
                                        key.push_str(val);
                                        key.push('|');
                                    }
                                }
                            }
                            key
                        }
                        None => row[col_idx].clone(),
                    };
                    *counts.entry(key).or_insert(0) += 1;
                }
            }
        }
        println!("counts: {:?}", counts);
        counts.into_iter().collect()
    }

    // Comparison Operators
    fn filter_equals(&self, column: &str, value: &str) -> Vec<Vec<String>> {
        let mut result = Vec::new();

        for &file_idx in &self.selected_csv_files {
            if let Some((_, grid)) = self.csv_files.get(file_idx) {
                if grid.is_empty() { continue; }

                let headers = &grid[0];
                let col_idx = match headers.iter().position(|h| h == column) {
                    Some(idx) => idx,
                    None => continue,
                };

                result.push(headers.clone());
                for row in grid.iter().skip(1) {
                    if row.len() > col_idx && row[col_idx] == value {
                        result.push(row.clone());
                    }
                }
            }
        }

        result
    }

    fn filter_greater_than(&self, column: &str, value: f64) -> Vec<Vec<String>> {
        let mut result = Vec::new();

        for &file_idx in &self.selected_csv_files {
            if let Some((_, grid)) = self.csv_files.get(file_idx) {
                if grid.is_empty() { continue; }

                let headers = &grid[0];
                let col_idx = match headers.iter().position(|h| h == column) {
                    Some(idx) => idx,
                    None => continue,
                };

                result.push(headers.clone());
                for row in grid.iter().skip(1) {
                    if row.len() > col_idx {
                        if let Ok(num) = row[col_idx].parse::<f64>() {
                            if num > value {
                                result.push(row.clone());
                            }
                        }
                    }
                }
            }
        }

        result
    }

    fn evaluate_expression(&self, expressions: &[String]) -> Vec<Value> {
        let mut stack: Vec<Value> = vec![];
        let mut results: Vec<Value> = Vec::new();
        let mut filter_conditions: Vec<String> = Vec::new();
        let mut i = 0;

        while i < expressions.len() {
            match expressions[i].as_str() {
                "GRP" => {
                  filter_conditions.push(expressions[i + 1].clone());
                    filter_conditions.push(expressions[i + 2].clone());
                    i += 3;
                }
                "SUM" | "COUNT" | "AVG" | "MUL" => {
                    if i + 1 < expressions.len() {
                        let field = &expressions[i + 1];
                        let operation = expressions[i].as_str();

                        // Apply all accumulated filter conditions
                        let filter_condition = if !filter_conditions.is_empty() {
                            Some(filter_conditions.clone())
                        } else {
                            None
                        };

                        let result = match operation {
                            "SUM" => {
                                let sum = self.sum(field, filter_condition.as_deref());
                                Value::Number(sum.iter().map(|(_, v)| v).sum())
                            }
                            "COUNT" => {
                                let count = self.count(field, filter_condition.as_deref());
                                Value::Number(count.len() as f64)
                            }
                            "AVG" => {
                                let avg = self.average(field, filter_condition.as_deref());
                                let value = if !avg.is_empty() {
                                    avg.iter().map(|(_, v)| v).sum::<f64>() / avg.len() as f64
                                } else {
                                    0.0
                                };
                                Value::Number(value)
                            }
                            "MUL" => {
                                println!("stack: {:?}", stack);
                                if let Some(Value::Number(left)) = stack.pop() {
                                    let mul = self.sum(field, filter_condition.as_deref());
                                    println!("left: {:?}, mul: {:?}", left, mul);
                                    Value::Number(left * mul.iter().map(|(_, v)| v).product::<f64>())
                                } else {
                                    let mul = self.sum(field, filter_condition.as_deref());
                                    println!("mul: {:?}", mul);
                                    Value::Number(mul.iter().map(|(_, v)| v).product::<f64>())
                                }
                            }
                            _ => unreachable!()
                        };

                        results.push(result.clone());
                        stack.push(result);
                        println!("12stack: {:?}", stack);
                        i += 2;
                    }
                }

                ">" | "<" | "=" => {
                    if stack.len() >= 2 {
                        let right = stack.pop().unwrap();
                        let left = stack.pop().unwrap();
                         println!("right: {:?}, left: {:?}", right, left);
                        match expressions[i].as_str() {
                            ">" => {
                                let comparison = match (left, right) {
                                    (Value::Number(left), Value::Number(right)) => {
                                        Value::Bool(left > right)
                                    }

                                    _ => unreachable!()
                                };
                                results.push(comparison)
                            }
                            "<" => {
                                let comparison = match (left, right) {
                                    (Value::Number(left), Value::Number(right)) => {
                                        Value::Bool(left < right)
                                    }

                                    _ => unreachable!()
                                };
                                results.push(comparison)
                            }
                            "=" => {
                                let comparison = match (left, right) {
                                    (Value::Number(left), Value::Number(right)) => {
                                        Value::Bool(left == right)
                                    }

                                    _ => unreachable!()
                                };
                                results.push(comparison)
                            }
                            _ => unreachable!()
                        }

                        // Create filter condition based on comparison
                        // let comparison = match (left, right) {
                        //     (Value::Number(left), Value::Number(right)) => {
                        //         format!("{}", left > right);
                        //         Value::Bool(left > right)
                        //     }
                        //
                        //     _ => unreachable!()
                        // };


                        //filter_conditions.push(condition);
                    }

                    i += 1;
                }

                _ => {
                    if let Ok(num) = expressions[i].parse::<f64>() {
                        stack.push(Value::Number(num));
                    } else {
                        stack.push(Value::Field(expressions[i].clone()));
                    }
                    i += 1;
                }
            }
        }

        if results.is_empty() && !stack.is_empty() {
            results.push(stack.pop().unwrap());
        }

        results
    }

    // Render ui
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
            ScrollArea::vertical().show(ui, |ui| {
                for (index, file) in self.csv_files.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let file_name = &file.0;
                        let mut selected = self.selected_csv_files.iter().any(|(f)| f == &index);

                        if ui.checkbox(&mut selected, file_name).clicked() {
                            if selected {
                               self.selected_csv_files.push(index);
                            } else {
                                self.selected_csv_files.retain(|(f)| f != &index);
                            }
                        }
                    });
                }
            });
            ui.add_space(20.0);

            ui.label("Step 2. Select fields & build query:".to_string());
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

            ui.horizontal(|ui| {
                for (index, fields) in csv_columns.iter().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.group(|ui| {
                            ui.set_min_size(Vec2::new(380.0, 150.0));
                            ScrollArea::both()
                                .max_height(150.0)
                                .max_width(380.0)
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        for field in fields.iter() {
                                            if ui.button(field).clicked() {
                                                if self.fields_2_compare.len() > 0 && self.fields_2_compare.len()-1 >= index {
                                                    self.fields_2_compare[index].push(field.to_string());
                                                } else {
                                                    self.fields_2_compare.push(vec![field.to_string()]);
                                                }
                                                println!("Compare {:?}", self.fields_2_compare);
                                            }
                                        }
                                    });
                                });
                        });
                    });
                    if ui.button("SUM").clicked() {
                         if self.fields_2_compare.len() > 0 && self.fields_2_compare.len()-1 >= index {
                             self.fields_2_compare[index].push("SUM".to_string());
                         } else {
                             self.fields_2_compare.push(vec!["SUM".to_string()]);
                         }
                    }
                    if ui.button("AVG").clicked() {
                         if self.fields_2_compare.len() > 0 && self.fields_2_compare.len()-1 >= index {
                             self.fields_2_compare[index].push("AVG".to_string());
                         } else {
                             self.fields_2_compare.push(vec!["AVG".to_string()]);
                         }
                    }
                    if ui.button("COUNT").clicked() {
                         if self.fields_2_compare.len() > 0 && self.fields_2_compare.len()-1 >= index {
                             self.fields_2_compare[index].push("COUNT".to_string());
                         } else {
                             self.fields_2_compare.push(vec!["COUNT".to_string()]);
                         }
                    }
                    if ui.button("MUL").clicked() {
                        if self.fields_2_compare.len() > 0 && self.fields_2_compare.len()-1 >= index {
                            self.fields_2_compare[index].push("MUL".to_string());
                        } else {
                            self.fields_2_compare.push(vec!["MUL".to_string()]);
                        }
                    }
                    if ui.button("GRP").clicked() {
                         if self.fields_2_compare.len() > 0 && self.fields_2_compare.len()-1 >= index {
                             self.fields_2_compare[index].push("GRP".to_string());
                         } else {
                             self.fields_2_compare.push(vec!["GRP".to_string()]);
                         }
                    }
                    if ui.button("=").clicked() {
                         if self.fields_2_compare.len() > 0 && self.fields_2_compare.len()-1 >= index {
                             self.fields_2_compare[index].push("=".to_string());
                         } else {
                             self.fields_2_compare.push(vec!["=".to_string()]);
                         }
                    }
                    if ui.button(">").clicked() {
                         if self.fields_2_compare.len() > 0 && self.fields_2_compare.len()-1 >= index {
                             self.fields_2_compare[index].push(">".to_string());
                         } else {
                             self.fields_2_compare.push(vec![">".to_string()]);
                         }
                    }
                    if ui.button("<").clicked() {
                         if self.fields_2_compare.len() > 0 && self.fields_2_compare.len()-1 >= index {
                             self.fields_2_compare[index].push("<".to_string());
                         } else {
                             self.fields_2_compare.push(vec!["<".to_string()]);
                         }
                    }
                }
            });

            ui.add_space(20.0);
            if ui.button("reset query").clicked() {
                self.fields_2_compare.clear();
            }
            if ui.button("Execute Expression").clicked() {
                for fields in &self.fields_2_compare {
                    let result = self.evaluate_expression(fields);
                    if !result.is_empty() {
                        println!("Result: {:?}", result);
                    }
                }
            }

            ui.label("Step 3. Fit data to chart:".to_string());
            /*  todo Billy
                Given the fields selected we want to graph the data.
                This will be tricky because data could be names(Strings), roles(Strings), numbers, bools, etc..
                1. Normalize data
                2. Select chart & apply data
             */
            if ui.button("Export Chart").clicked() {
                // todo Billy
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