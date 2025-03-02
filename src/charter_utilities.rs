use egui::epaint::TextShape;
use egui::{emath, pos2, vec2, Align2, Color32, FontId, Painter, Rect, ScrollArea, Sense, Shape, Stroke, WidgetText};
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

pub fn format_graph_query(graph_data:Vec<Value>) -> Vec<PlotPoint> {
    let mut plot_data: Vec<PlotPoint> = Vec::new();

    let mut i = 0;
    while i < graph_data.len() {
        match &graph_data[i] {
            Value::Number(num) => {
                if i + 1 < graph_data.len() {
                    if let Value::Field(label) = &graph_data[i + 1] {
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

pub fn draw_bar_graph(ui: &mut egui::Ui, formatted_data: Option<Vec<PlotPoint>>) -> Option<egui::Response> {
    ScrollArea::horizontal().show(ui, |ui| {
        if let Some(graph_data) = &formatted_data {
            let available_width = ui.available_width() * (ui.available_width() / graph_data.len() as f32);
            let available_height: f64 = 600.0;
            let bar_spacing = 2.0;

            let values: Vec<f64> = graph_data.iter()
                .map(|data| data.value)
                .collect();

            let max_value = values.iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&1.0);

            let bar_width = (ui.available_width() / graph_data.len() as f32);

            let (response, painter) = ui.allocate_painter(
                vec2(available_width, (available_height + 40.0) as f32),
                Sense::hover(),
            );

            let rect = response.rect;

            // Draw Y-axis label
            painter.text(
                pos2(rect.min.x - 40.0, rect.min.y + (available_height / 2.0) as f32),
                Align2::CENTER_CENTER,
                "Count",
                FontId::default(),
                Color32::BLACK,
            );

            // Draw bars and labels
            for (i, (data, value)) in graph_data.iter().zip(values.iter()).enumerate() {
                let value_normalized = value / max_value;
                let height = value_normalized * available_height;
                let x = rect.min.x + (i as f32 * (bar_width + bar_spacing));

                // Draw bar
                let bar_rect = Rect::from_min_size(
                    pos2(x, rect.max.y - (height - 20.0) as f32),
                    vec2(bar_width, height as f32),
                );
                painter.rect_filled(bar_rect, 0.0, Color32::from_rgb(65, 155, 220));

                // Draw value text
                painter.text(
                    pos2(x + bar_width / 2.0, bar_rect.min.y - 5.0),
                    Align2::CENTER_BOTTOM,
                    format!("{:.0}", value),
                    FontId::default(),
                    Color32::BLACK,
                );

                // Draw rotated label
                let shapes = draw_rotated_text(&painter, rect, &data.label, x, bar_width);
                ui.painter_at(rect).extend(shapes);
            }

            Some(response)
        } else {
            None
        }
    }).inner
}