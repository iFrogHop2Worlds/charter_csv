use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use eframe::{egui, App};
use eframe::emath::Vec2;
use eframe::epaint::Color32;
use egui::{CentralPanel, ScrollArea};

fn main() {
    let ctx = egui::Context::default();
    let mut size = ctx.used_size();
    size.x = 780.00;
    size.y = 420.00;
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
    csvqb_pipeline: Vec<Vec<String>>,
    graph_data: Vec<Value>,
    file_receiver: Receiver<(String, Vec<Vec<String>>)>,
    file_sender: Sender<(String, Vec<Vec<String>>)>,
}
#[derive(Debug)]
struct PlotPoint {
    label: String,
    value: f64,
}
#[derive(Debug, Clone)]
enum Value {
    Bool(bool),
    Number(f64),
    Text(String),
    Field(String),
    QueryResult(Vec<Vec<(String)>>) // For storing column data
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
        let (tx, rx) = mpsc::channel();
        Self {
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
        if let Ok((path, grid)) = self.file_receiver.try_recv() {
            self.csv_files.push((path, grid));
        }
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

    fn col_sum(&self, column: &str, group_by: Option<&[String]>) -> Vec<(String, f64)> {
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

    fn col_average(&self, column: &str, group_by: Option<&[String]>) -> Vec<(String, f64)> {
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

    fn col_count(&self, column: &str, group_by: Option<&[String]>) -> Vec<Vec<String>> {
        let mut counts = std::collections::HashMap::new();
        let mut query_grid = Vec::new();
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

                let mut header_row = Vec::new();
                if let Some(group_cols) = group_by {
                    header_row.extend(group_cols.iter().cloned());
                } else {
                    header_row.push(column.to_string());
                }

                header_row.push("count".to_string());
                query_grid.push(header_row.clone());
                let _ = header_row.pop();

                for (key, count) in counts.drain() {
                    let mut row = Vec::new();
                    if key.contains('|') {
                        let values: Vec<&str> = key.split('|').filter(|s| !s.is_empty()).collect();
                        row.extend(values.iter().map(|&s| s.to_string()));
                    } else {
                        row.push(key);
                    }

                    row.push(count.to_string());
                    query_grid.push(row);
                }
            }
        }

        //counts.into_iter().collect()
        query_grid
    }

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
    // todo several more operators to implement and tighten up current implementations (experimental)
    fn process_csvqb_pipeline(&self, qb_pipeline: &[String]) -> Vec<Value> {
        let mut stack: Vec<Value> = vec![];
        let mut results: Vec<Value> = Vec::new();
        let mut capture_group: Vec<String> = Vec::new();
        let mut i = 0;

        while i < qb_pipeline.len() {
            match qb_pipeline[i].as_str() {
                "GRP" => {
                    while i + 1 < qb_pipeline.len() {
                        if ["GRP", "CSUM", "CCOUNT", "CAVG", "CMUL", "MUL", "=", "<", ">"].contains(&qb_pipeline[i + 1].as_str()) {
                            break;
                        }
                        capture_group.push(qb_pipeline[i + 1].clone());
                        i+=1
                    }
                    i+=1
                }
                "CSUM" | "CCOUNT" | "CAVG" | "CMUL" => {
                    if i + 1 < qb_pipeline.len() {
                        let field = &qb_pipeline[i + 1];
                        let operation = qb_pipeline[i].as_str();

                        let filter_condition = if !capture_group.is_empty() {
                            Some(capture_group.clone())
                        } else {
                            None
                        };

                        let result = match operation {
                            "CSUM" => {
                                let sum = self.col_sum(field, filter_condition.as_deref());
                                Value::Number(sum.iter().map(|(_, v)| v).sum())
                            }
                            "CCOUNT" => {
                                let counts = self.col_count(field, filter_condition.as_deref());
                                Value::QueryResult(counts)
                            }
                            "CAVG" => {
                                let avg = self.col_average(field, filter_condition.as_deref());
                                let value = if !avg.is_empty() {
                                    avg.iter().map(|(_, v)| v).sum::<f64>() / avg.len() as f64
                                } else {
                                    0.0
                                };
                                Value::Number(value)
                            }
                            "CMUL" => {
                                println!("stack: {:?}", stack);
                                if let Some(Value::Number(left)) = stack.pop() {
                                    let mul = self.col_sum(field, filter_condition.as_deref());
                                    println!("left: {:?}, mul: {:?}", left, mul);
                                    Value::Number(left * mul.iter().map(|(_, v)| v).product::<f64>())
                                } else {
                                    let mul = self.col_sum(field, filter_condition.as_deref());
                                    println!("mul: {:?}", mul);
                                    Value::Number(mul.iter().map(|(_, v)| v).product::<f64>())
                                }
                            }

                            _ => unreachable!()
                        };

                        results.push(result.clone());
                        stack.push(result);
                        i+=1
                    }
                }
                "MUL" => {
                    println!("stack: {:?}", stack);
                    if let (Some(Value::Number(right)), Some(Value::Number(left))) = (stack.pop(), stack.pop()) {
                        stack.push(Value::Number(left * right));
                        results.push(Value::Number(left * right));
                    } else {
                        println!("err in MUL");
                        break;
                    }
                    i+=1;
                }

                ">" | "<" | "=" => {
                    if stack.len() >= 2 {
                        let right = stack.pop().unwrap();
                        let left = stack.pop().unwrap();
                         println!("right: {:?}, left: {:?}", right, left);
                        match qb_pipeline[i].as_str() {
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
                    }

                    i+=1
                }
                "(" | ")" => {
                    if qb_pipeline[i] == "(" {
                        while i < qb_pipeline.len() {
                            if qb_pipeline[i] == ")" {
                                break
                            }
                            let result = self.process_csvqb_pipeline(&qb_pipeline[i+1..]);
                            println!("result: {},  {:?}", i, result);
                            println!("stack: {},  {:?}", i, stack);
                            if !result.is_empty() {
                                results.push(result[0].clone());
                                break;
                            }
                            i+=1
                        }
                    }
                    i+=1
                }

                _ => {
                    if let Ok(num) = qb_pipeline[i].parse::<f64>() {
                        stack.push(Value::Number(num));
                    }
                    else {
                        results.push(Value::Field(qb_pipeline[i].clone()));
                    }
                    i+=1
                }
            }
        }

        if results.is_empty() && !stack.is_empty() {
            results.push(stack.pop().unwrap());
        }

        results
    }

    //experimental
    fn fit_to_graph(&self) -> Vec<PlotPoint> {
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
                        let sender = self.file_sender.clone();
                        thread::spawn(move || {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                let grid: CsvGrid = CharterCsv::csv2grid(&content);
                                let _ = sender.send((path_as_string, grid));
                            }
                        });
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
                                                if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                                                    self.csvqb_pipeline[index].push(field.to_string());
                                                } else {
                                                    self.csvqb_pipeline.push(vec![field.to_string()]);
                                                }
                                                println!("Compare {:?}", self.csvqb_pipeline);
                                            }
                                        }
                                    });
                                });
                        });
                    });
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
                    if ui.button("CMUL").clicked() {
                        if self.csvqb_pipeline.len() > 0 && self.csvqb_pipeline.len()-1 >= index {
                            self.csvqb_pipeline[index].push("CMUL".to_string());
                        } else {
                            self.csvqb_pipeline.push(vec!["CMUL".to_string()]);
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
                }
            });

            ui.add_space(20.0);
            if ui.button("reset query").clicked() {
                self.csvqb_pipeline.clear();
            }

            if ui.button("Execute Expression").clicked() {
                for fields in self.csvqb_pipeline.iter() {
                    let result = self.process_csvqb_pipeline(fields);
                    if !result.is_empty() {
                        println!("Result: {:?}", &result);
                        self.graph_data = result;
                    }
                }
            }

            ui.label("Step 3. Fit data to chart:".to_string());

            if ui.button("view chart").clicked() {
                self.screen = Screen::ViewChart;
            }


        });
    }

    fn show_chart_screen(&mut self, ctx: &egui::Context, chart: &ChartGrid) {
        CentralPanel::default().show(ctx, |ui| {
            if ui.button("Back").clicked() {
                self.screen = Screen::CreateChart;
            }
            let mut fit_data_to_graph = Some(self.fit_to_graph());

            ui.horizontal(|ui| {
                if let Some(graph_data) = fit_data_to_graph {
                    let available_width = ui.available_width();
                    let available_height:f64 = 600.0;
                    let bar_spacing = 2.0;
                    let values: Vec<f64> = graph_data.iter()
                        .map(|data| data.value)
                        .collect();
                    let max_value = values.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&1.0);
                    let bar_width = (available_width / graph_data.len() as f32) - bar_spacing;

                    let (response, painter) = ui.allocate_painter(
                        egui::vec2(available_width, (available_height + 40.0) as f32), // Extra height for labels
                        egui::Sense::hover(),
                    );

                    let rect = response.rect;

                    painter.text(
                        egui::pos2(rect.min.x - 40.0, rect.min.y + (available_height / 2.0) as f32),
                        egui::Align2::CENTER_CENTER,
                        "Count", // todo make dynamic labels
                        egui::FontId::default(),
                        Color32::WHITE,
                    );

                    for (i, (data, value)) in graph_data.iter().zip(values.iter()).enumerate() {
                        let value_normalized = value / max_value;
                        let height = value_normalized * available_height;
                        let x = rect.min.x + (i as f32 * (bar_width + bar_spacing));

                        let bar_rect = egui::Rect::from_min_size(
                            egui::pos2(x, rect.max.y - (height - 20.0) as f32),
                            egui::vec2(bar_width, height as f32),
                        );

                        painter.rect_filled(bar_rect, 0.0, Color32::from_rgb(65, 155, 220));

                        painter.text(
                            egui::pos2(x + bar_width / 2.0, bar_rect.min.y - 5.0),
                            egui::Align2::CENTER_BOTTOM,
                            format!("{:.0}", value),
                            egui::FontId::default(),
                            Color32::WHITE,
                        );

                        painter.text(
                            egui::pos2(x + bar_width / 2.0, rect.max.y - 10.0),
                            egui::Align2::CENTER_BOTTOM,
                            &data.label,
                            egui::FontId::default(),
                            Color32::WHITE,
                        );
                    }
                }
            });


            if ui.button("Export Chart").clicked() {
                // todo Billy
            }
        });
    }
}



