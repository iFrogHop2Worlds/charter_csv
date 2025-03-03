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
                for (idx, row) in query_result.iter().skip(1).enumerate() {
                    if row.len() < headers.len() {
                        println!("{}", "Mismatch in row and column sizes in QueryResult".to_string());
                    }

                    if let Ok(last_value) = row.last().unwrap().parse::<f64>() {
                        plot_data.push(PlotPoint {
                            label: row[0].to_string(),
                            value: last_value,
                            x: idx as f64,
                            y: last_value,
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

    plot_data
}

