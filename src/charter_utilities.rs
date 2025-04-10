use egui::{UserData, emath, pos2, vec2, Color32, FontId, Id, Painter, Pos2, Rect, Shape, Stroke, WidgetText, ScrollArea, Vec2, TextEdit, Response, Sense, CursorIcon, StrokeKind};
use egui::epaint::TextShape;
use crate::charter_csv::PlotPoint;
use crate::csvqb::Value;
use rfd::FileDialog;

#[derive(Clone, Debug)]
pub struct DraggableLabel {
    pub text: String,
    pub pos: Pos2,
    pub id: u32,
}

pub type CsvGrid = Vec<Vec<String>>;

pub fn csv2grid(content: &str) -> CsvGrid {
    content
        .lines()
        .map(|line| line.split(',')
            .map(|s| s.trim().to_string())
            .collect())
        .collect()
}
pub fn grid2csv(grid: &CsvGrid) -> String {
    grid.iter()
        .map(|row| row.join(","))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug)]
pub enum TextPlacement {
    Top,
    Middle,
    Bottom,
}

pub fn draw_rotated_text(
    painter: &Painter,
    rect: Rect,
    data_label: &str,
    x: f32,
    bar_width: f32,
    rotation_degrees: f32,
    placement: TextPlacement,
) -> Vec<Shape> {
    let text = WidgetText::from(data_label);
    let galley = painter.layout_no_wrap(
        text.text().to_string(),
        FontId::proportional(12.0),
        Color32::BLACK,
    );

    let y_position = match placement {
        TextPlacement::Top => rect.min.y + 20.0,
        TextPlacement::Middle => rect.center().y,
        TextPlacement::Bottom => rect.max.y - 20.0,
    };

    let pos = pos2(x + bar_width / 2.0, y_position);

    // Convert degrees to radians
    let rotation_angle = (rotation_degrees.clamp(0.0, 360.0) * std::f32::consts::PI) / 180.0;
    let rot = emath::Rot2::from_angle(rotation_angle);

    let x_adjustment = 0.0;
    let y_adjustment = match placement {
        TextPlacement::Top => 0.0,
        TextPlacement::Middle => -10.0,
        TextPlacement::Bottom => -20.0,
    };

    let offset = vec2(
        -galley.size().x + x_adjustment,
        galley.size().y + y_adjustment
    );

    let rotated_offset = rot * offset;
    let final_pos = pos + rotated_offset;

    vec![Shape::Text(TextShape {
        pos: final_pos,
        galley,
        angle: rotation_angle,
        underline: Stroke::NONE,
        fallback_color: Color32::BLACK,
        override_text_color: Some(Color32::BLACK),
        opacity_factor: 1.0,
    })]
}
pub fn load_icon() -> egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("sailboat.png");
        let image = image::load_from_memory(icon)
            .expect("Failed to load icon")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

pub fn format_graph_query(graph_data: Vec<Value>) -> Vec<PlotPoint> {
    if graph_data.is_empty() {
        return Vec::new(); // Return empty vector if input is empty
    }

    let mut plot_data: Vec<PlotPoint> = Vec::new();
    let mut i = 0;
    while i < graph_data.len() {
        match &graph_data[i] {
            Value::Number(num) => {
                if i + 1 < graph_data.len() {
                    if let Value::Field(label) = &graph_data[i + 1] {
                        println!("gd -----> \n {:?}", &graph_data);
                        plot_data.push(PlotPoint {
                            label: label.to_string(),
                            value: *num,
                            x: i as f64,
                            y: *num,
                            depth: 0.0,
                        });
                        i += 2;
                    } else {
                        println!("Expected Field after Number");
                        i += 1;
                    }
                } else {
                    println!("Incomplete data: Number without a corresponding Field");
                    i += 1;
                }
            }
            Value::QueryResult(query_result) => {
                if query_result.is_empty() {
                    println!("QueryResult is empty");
                    i += 1;
                    continue;
                }

                if query_result.len() > 1 {
                    for (idx, row) in query_result.iter().enumerate().skip(1) {
                        let _row = row[..row.len() - 1].to_vec();
                        let label = _row.join("-");
                        if !row.is_empty() {
                            if let Some(last_cell) = row.last() {
                                if let Ok(last_value) = last_cell.parse::<f64>() {
                                    plot_data.push(PlotPoint {
                                        label,//: row[0].to_string(),
                                        value: last_value,
                                        x: (idx - 1) as f64,
                                        y: last_value,
                                        depth: 0.0,
                                    });
                                }
                            }
                        }
                    }
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    plot_data
}

pub fn save_window_as_png(ctx: &egui::Context, window_id: Id) {
    let mut screenshot_data = UserData::default();
    let _ = screenshot_data.data.insert(std::sync::Arc::new(window_id));
    let screenshot_cmd = egui::ViewportCommand::Screenshot(screenshot_data);
    let window_rect = ctx.available_rect();
    let scale = ctx.pixels_per_point();
    let window_pos = ctx.screen_rect().max;

    ctx.data_mut(|data| {
        data.insert_temp(
            Id::new("waiting_for_screenshot"),
            (true, window_id, window_rect, scale, window_pos)
        );
    });

    let viewport_id = egui::ViewportId::default();
    ctx.send_viewport_cmd_to(viewport_id, screenshot_cmd);
}

pub fn check_for_screenshot(ctx: &egui::Context) {
    let (waiting, target_id, _, scale, _) = ctx.data(|data| {
        data.get_temp::<(bool, Id, Rect, f32, Pos2)>(Id::new("waiting_for_screenshot"))
            .unwrap_or((false, Id::NULL, Rect::NOTHING, 1.0, Pos2::ZERO))
    });

    if waiting {
        let (window_pos, width, height) = ctx.memory(|mem| {
            mem.area_rect(target_id)
                .map(|rect| {
                    let top_left = rect.left_top();
                    let bottom_right = rect.right_bottom();
                    let width = (bottom_right.x - top_left.x) * scale;
                    let height = (bottom_right.y - top_left.y) * scale;
                    (top_left, width as usize, height as usize)
                })
                .unwrap_or((Pos2::ZERO, 0, 0))
        });

        let x = (window_pos.x.round()  * scale) as usize;
        let y = (window_pos.y.round()  * scale) as usize;

        ctx.input(|i| {
            for event in &i.raw.events {
                if let egui::Event::Screenshot { image, user_data, .. } = event {
                    if let Some(data) = user_data.data.as_ref() {
                        if let Some(window_id) = data.downcast_ref::<Id>() {
                            if window_id == &target_id {

                                if x >= image.size[0] || y >= image.size[1] {
                                    eprintln!("Invalid crop coordinates: outside image bounds");
                                    return;
                                }

                                let mut cropped_image = egui::ColorImage::new(
                                    [width, height],
                                    Color32::TRANSPARENT
                                );

                                let max_width = width.min(image.size[0] - x);
                                let max_height = height.min(image.size[1] - y);

                                for dy in 0..max_height {
                                    for dx in 0..max_width {
                                        let src_idx = (y + dy) * image.size[0] + (x + dx);
                                        let dst_idx = dy * width + dx;
                                        if src_idx < image.pixels.len() && dst_idx < cropped_image.pixels.len() {
                                            cropped_image.pixels[dst_idx] = image.pixels[src_idx];
                                        }
                                    }
                                }

                                if let Some(path) = FileDialog::new()
                                    .add_filter("PNG Image", &["png"])
                                    .set_file_name("graph.png")
                                    .save_file()
                                {
                                    let image_clone = cropped_image;
                                    let mut ctx_clone = ctx.clone();

                                    std::thread::spawn(move || {
                                        if let Err(e) = image::save_buffer(
                                            &path,
                                            image_clone.as_raw(),
                                            width as u32,
                                            height as u32,
                                            image::ColorType::Rgba8,
                                        ) {
                                            eprintln!("Failed to save image: {}", e);
                                        }
                                        ctx_clone.data_mut(|data| {
                                            data.remove::<(bool, Id, Rect, f32, Pos2)>(
                                                Id::new("waiting_for_screenshot")
                                            );
                                        });
                                        ctx_clone.request_repaint();
                                    });
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}

// grid layouts csv editor
pub struct GridLayout {
    col_widths: Vec<f32>,
    row_heights: Vec<f32>,
    dragging: Option<(usize, bool)>,
    min_size: f32,
    max_size: f32,
}

impl GridLayout {
    pub(crate) fn new(cols: usize, rows: usize) -> Self {
        Self {
            col_widths: vec![100.0; cols],
            row_heights: vec![30.0; rows],
            dragging: None,
            min_size: 20.0,
            max_size: 150.0,
        }
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui, grid: &mut Vec<Vec<String>>) {
        ScrollArea::both()
            .auto_shrink([false; 2])
            .show_viewport(ui, |ui, viewport| {
                if grid.is_empty() { return; }

                let total_width: f32 = self.col_widths.iter().take(grid[0].len()).sum();
                let total_height: f32 = self.row_heights.iter().take(grid.len()).sum();
                ui.set_min_size(Vec2::new(total_width, total_height));

                let mut accumulated_height = 0.0;
                let start_row = {
                    let mut idx = 0;
                    while idx < grid.len() && accumulated_height <= viewport.min.y {
                        accumulated_height += self.row_heights[idx];
                        idx += 1;
                    }
                    idx.saturating_sub(1)
                };

                let mut height_in_view = 0.0;
                let mut visible_rows = 0;
                let mut row_idx = start_row;
                while row_idx < grid.len() && height_in_view < viewport.height() + self.row_heights[row_idx] {
                    height_in_view += self.row_heights[row_idx];
                    visible_rows += 1;
                    row_idx += 1;
                }

                let start_col = (viewport.min.x / self.col_widths[0]).floor().max(0.0) as usize;
                let visible_cols = (viewport.width() / self.col_widths[0]).ceil() as usize + 1;
                let end_row = (start_row + visible_rows).min(grid.len());
                let end_col = (start_col + visible_cols).min(grid[0].len());

                let top_offset = self.row_heights.iter().take(start_row).sum::<f32>();
                ui.add_space(top_offset);

                ui.spacing_mut().item_spacing.y = -3.0;
                ui.spacing_mut().item_spacing.x = -3.0;
                for row_idx in start_row..end_row {
                    ui.horizontal(|ui| {
                        let left_offset = self.col_widths.iter().take(start_col).sum::<f32>();
                        if start_col > 0 {
                            ui.add_space(left_offset);
                        }

                        for col_idx in start_col..end_col {
                            let cell = &mut grid[row_idx][col_idx];


                            //Todo (Billy) Using multiline throws off spacing calculations for cells in the grid using sinlgline for now for predictable calculations..

                            // let frame = egui::Frame::NONE
                            //     .fill(Color32::WHITE);
                            // let response = frame.show(ui, |ui| {
                            //     ScrollArea::vertical()
                            //         .show(ui, |ui| {
                            //             ui.add(
                            //                 TextEdit::multiline(cell)
                            //                     .min_size(vec2(self.col_widths[col_idx], self.row_heights[row_idx]))
                            //             )
                            //         })
                            //         .inner
                            // }).response;

                            let response = ui.add_sized(
                                Vec2::new(self.col_widths[col_idx], self.row_heights[row_idx]),
                                TextEdit::singleline(cell),
                            );
                             ui.painter().rect_stroke(response.rect, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Outside);

                            if col_idx < end_col {
                                let resizer_width = 6.0;
                                let resizer_rect = Rect::from_min_size(
                                    response.rect.right_top() + Vec2::new(-resizer_width/2.0, 0.0),
                                    Vec2::new(resizer_width, self.row_heights[row_idx])
                                );

                                let painter = ui.painter_at(resizer_rect);
                                let response = ui.allocate_rect(resizer_rect, Sense::drag());
                                if response.hovered() {
                                    painter.rect_filled(resizer_rect, 0.0, Color32::from_gray(180));
                                    ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                                }
                                if response.dragged() {
                                    self.dragging = Some((col_idx, true));
                                }
                            }
                        }

                        if row_idx < end_row - 1 {
                            let resizer_height = 6.0;
                            let row_width = self.col_widths.iter().skip(start_col).take(end_col - start_col).sum::<f32>();
                            let resizer_rect = Rect::from_min_size(
                                ui.min_rect().min + Vec2::new(left_offset, self.row_heights[row_idx] - resizer_height/2.0),
                                Vec2::new(row_width, resizer_height)
                            );

                            let painter = ui.painter_at(resizer_rect);
                            let response = ui.allocate_rect(resizer_rect, Sense::drag());
                            if response.hovered() {
                                painter.rect_filled(resizer_rect, 0.0, Color32::from_gray(180));
                                ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeVertical);
                            }
                            if response.dragged() {
                                self.dragging = Some((row_idx, false));
                            }
                        }
                    });
                }

                let bottom_space = total_height - self.row_heights.iter().take(end_row).sum::<f32>();
                if bottom_space > 0.0 {
                    ui.add_space(bottom_space);
                }

                if let Some((idx, is_vertical)) = self.dragging {
                    if !ui.input(|i| i.pointer.primary_down()) {
                        self.dragging = None;
                    } else {
                        let delta = ui.input(|i| i.pointer.delta());
                        if is_vertical {
                            self.adjust_column_width(idx, delta.x);
                        } else {
                            self.adjust_row_height(idx, delta.y);
                        }
                    }
                }
            });
    }

    fn adjust_column_width(&mut self, idx: usize, delta: f32) {
        if idx >= self.col_widths.len() { return; }
        self.col_widths[idx] = (self.col_widths[idx] + delta).max(10.0);
    }

    fn adjust_row_height(&mut self, idx: usize, delta: f32) {
        if idx >= self.row_heights.len() { return; }
        self.row_heights[idx] = (self.row_heights[idx] + delta).max(10.0);
    }

    pub fn add_column(&mut self, grid: &mut Vec<Vec<String>>) {
        for row in grid.iter_mut() {
            row.push(String::new());
        }
        self.col_widths.push(100.0);
    }

    pub fn add_row(&mut self, grid: &mut Vec<Vec<String>>) {
        if let Some(first_row) = grid.first() {
            let new_row = vec![String::new(); first_row.len()];
            grid.push(new_row);
            self.row_heights.push(20.0);
        }
    }
}

