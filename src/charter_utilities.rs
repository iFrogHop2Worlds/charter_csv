use egui::epaint::TextShape;
use egui::{emath, pos2, vec2, Color32, FontId, Painter, Rect, Shape, Stroke, WidgetText};
use crate::charter_csv::PlotPoint;
use crate::csvqb::Value;

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
pub fn draw_rotated_text(
    painter: &Painter,
    rect: Rect,
    data_label: &str,
    x: f32,
    bar_width: f32
) -> Vec<Shape> {
    let text = WidgetText::from(data_label);
    let galley = painter.layout_no_wrap(
        text.text().to_string(),
        FontId::default(),
        Color32::BLACK,
    );

    let pos = pos2(x + bar_width / 2.0, rect.max.y / 2.0);
    let rot = emath::Rot2::from_angle(std::f32::consts::FRAC_PI_2 * 2.99);

    let offset = vec2(-galley.size().y / 2.0, -galley.size().x / 2.0);
    let rotated_offset = rot * offset;
    let final_pos = pos + rotated_offset;

    vec![Shape::Text(TextShape {
        pos: final_pos,
        galley,
        angle: std::f32::consts::FRAC_PI_2 * 2.99,
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
                        if !row.is_empty() {
                            if let Some(last_cell) = row.last() {
                                if let Ok(last_value) = last_cell.parse::<f64>() {
                                    plot_data.push(PlotPoint {
                                        label: row.first()
                                            .map(|s| s.to_string())
                                            .unwrap_or_default(),
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

