use eframe::emath::{pos2, vec2, Align2, Rect};
use eframe::epaint::{Color32, FontId, Stroke};
use egui::{ScrollArea, Sense, Shape};
use egui::epaint::{PathShape};
use crate::charter_csv::PlotPoint;
use crate::charter_utilities::draw_rotated_text;

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

            let bar_width = ui.available_width() / graph_data.len() as f32;

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

pub fn draw_pie_chart(ui: &mut egui::Ui, formatted_data: Option<Vec<PlotPoint>>) -> Option<egui::Response> {
    if let Some(graph_data) = &formatted_data {
        let size = 400.0;  // Fixed size for the pie chart
        let (response, painter) = ui.allocate_painter(
            vec2(size, size),
            Sense::hover(),
        );
        let rect = response.rect;
        let center = rect.center();
        let radius = size / 2.5;

        let total: f64 = graph_data.iter()
            .map(|data| data.value)
            .sum();

        let mut start_angle = 0.0f32;
        let mut legend_y = rect.min.y;

        let stroke = Stroke::new(1.0, Color32::BLACK);

        for (i, data) in graph_data.iter().enumerate() {
            let percentage = (data.value / total) as f32;
            let angle = percentage * std::f32::consts::TAU;

            let hue = (i as f32 * 0.618034) % 1.0;
            let color = Color32::from_rgb(
                (255.0 * hue.sin().abs()) as u8,
                (255.0 * (hue + 0.33).sin().abs()) as u8,
                (255.0 * (hue + 0.67).sin().abs()) as u8,
            );

            // Create points for the sector
            let mut points = Vec::new();
            points.push(center); // Start from center

            // Add points to create the arc
            let steps = 32; // Number of points to approximate the arc
            for j in 0..=steps {
                let current_angle = start_angle + (angle * j as f32 / steps as f32);
                points.push(pos2(
                    center.x + radius * current_angle.cos(),
                    center.y + radius * current_angle.sin(),
                ));
            }

            // Draw the sector
            painter.add(Shape::Path(PathShape::convex_polygon(
                points,
                color,
                Stroke::NONE,
            )));

            // Draw the radial line
            painter.add(Shape::LineSegment {
                points: [
                    center,
                    pos2(
                        center.x + radius * (start_angle + angle/2.0).cos(),
                        center.y + radius * (start_angle + angle/2.0).sin(),
                    )
                ],
                stroke,
            });

            // Draw legend
            let legend_rect = Rect::from_min_size(
                pos2(rect.max.x + 10.0, legend_y),
                vec2(20.0, 20.0),
            );
            painter.rect_filled(legend_rect, 0.0, color);

            painter.text(
                pos2(legend_rect.max.x + 10.0, legend_y + 10.0),
                Align2::LEFT_CENTER,
                format!("{}: {:.1}%", data.label, percentage * 100.0),
                FontId::default(),
                Color32::BLACK,
            );

            start_angle += angle;
            legend_y += 25.0;
        }

        Some(response)
    } else {
        None
    }
}


pub fn draw_histogram(ui: &mut egui::Ui, formatted_data: Option<Vec<PlotPoint>>) -> Option<egui::Response> {
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

            let bin_count = 10; // Number of bins for the histogram
            let min_value = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let bin_width = (max_value - min_value) / bin_count as f64;

            let mut bins = vec![0.0; bin_count];
            for &value in &values {
                let bin_index = ((value - min_value) / bin_width).floor() as usize;
                let bin_index = bin_index.min(bin_count - 1); // Clamp to ensure we don't go out of bounds
                bins[bin_index] += 1.0;
            }

            let bar_width = ui.available_width() / bin_count as f32;

            let (response, painter) = ui.allocate_painter(
                vec2(available_width, (available_height + 40.0) as f32),
                Sense::hover(),
            );

            let rect = response.rect;

            // Draw Y-axis label
            painter.text(
                pos2(rect.min.x - 40.0, rect.min.y + (available_height / 2.0) as f32),
                Align2::CENTER_CENTER,
                "Frequency",
                FontId::default(),
                Color32::BLACK,
            );

            // Draw Histrogram bins
            for (i, &frequency) in bins.iter().enumerate() {
                let frequency_normalized = frequency / bins.iter().cloned().fold(1.0, f64::max);
                let height = frequency_normalized * available_height;
                let x = rect.min.x + (i as f32 * (bar_width + bar_spacing));

                // Draw bin (bar)
                let bar_rect = Rect::from_min_size(
                    pos2(x, rect.max.y - (height - 20.0) as f32),
                    vec2(bar_width, height as f32),
                );
                painter.rect_filled(bar_rect, 0.0, Color32::from_rgb(135, 206, 250));

                // Draw frequency value text
                painter.text(
                    pos2(x + bar_width / 2.0, bar_rect.min.y - 5.0),
                    Align2::CENTER_BOTTOM,
                    format!("{:.0}", frequency),
                    FontId::default(),
                    Color32::BLACK,
                );

                // Draw bin range label
                let bin_start = min_value + (i as f64 * bin_width);
                let bin_end = bin_start + bin_width;
                let label = if i == bin_count - 1 {
                    format!("[{:.1}, {:.1}]", bin_start, bin_end)
                } else {
                    format!("[{:.1}, {:.1})", bin_start, bin_end)
                };
                painter.text(
                    pos2(x + bar_width / 2.0, rect.max.y + 5.0),
                    Align2::CENTER_TOP,
                    label,
                    FontId::default(),
                    Color32::BLACK,
                );
            }

            Some(response)
        } else {
            None
        }
    }).inner
}


pub fn draw_scatter_plot(ui: &mut egui::Ui, formatted_data: Option<Vec<PlotPoint>>) -> Option<egui::Response> {
    ScrollArea::horizontal().show(ui, |ui| {
        if let Some(graph_data) = &formatted_data {
            let padding = 10.0;
            let (response, painter) = ui.allocate_painter(
                vec2(ui.available_width(), 600.0),
                Sense::hover(),
            );

            let rect = response.rect;
            let plot_width = rect.width() - padding * 2.0;
            let plot_height = rect.height() - padding * 2.0;

            let x_values: Vec<f64> = graph_data.iter().map(|data| data.x).collect();
            let y_values: Vec<f64> = graph_data.iter().map(|data| data.y).collect();

            let x_min = x_values.iter().copied().fold(f64::INFINITY, f64::min);
            let x_max = x_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let y_min = y_values.iter().copied().fold(f64::INFINITY, f64::min);
            let y_max = y_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

            let x_scale = plot_width / (x_max - x_min) as f32;
            let y_scale = plot_height / (y_max - y_min) as f32;

            // Draw X-axis and Y-axis
            painter.add(Shape::line_segment(
                [
                    pos2(rect.min.x + padding, rect.min.y + padding + plot_height),
                    pos2(rect.min.x + padding + plot_width, rect.min.y + padding + plot_height),
                ],
                Stroke::new(1.0, Color32::BLACK),
            ));
            painter.add(Shape::line_segment(
                [
                    pos2(rect.min.x + padding, rect.min.y + padding),
                    pos2(rect.min.x + padding, rect.min.y + padding + plot_height),
                ],
                Stroke::new(1.0, Color32::BLACK),
            ));

            // Draw scatter points
            for point in graph_data {
                let screen_x = rect.min.x + padding + (point.x - x_min) as f32 * x_scale;
                let screen_y = rect.min.y + padding + plot_height - (point.y - y_min) as f32 * y_scale;

                painter.add(Shape::circle_filled(
                    pos2(screen_x, screen_y),
                    4.0,
                    Color32::from_rgb(30, 144, 255),
                ));
            }

            Some(response)
        } else {
            None
        }
    }).inner
}

pub fn draw_line_chart(ui: &mut egui::Ui, formatted_data: Option<Vec<PlotPoint>>) -> Option<egui::Response> {
    ScrollArea::horizontal().show(ui, |ui| {
        if let Some(graph_data) = &formatted_data {
            let padding = 10.0;
            let (response, painter) = ui.allocate_painter(
                vec2(ui.available_width(), 600.0),
                Sense::hover(),
            );

            let rect = response.rect;
            let plot_width = rect.width() - padding * 2.0;
            let plot_height = rect.height() - padding * 2.0;

            let x_values: Vec<f64> = graph_data.iter().map(|data| data.x).collect();
            let y_values: Vec<f64> = graph_data.iter().map(|data| data.y).collect();

            let x_min = x_values.iter().copied().fold(f64::INFINITY, f64::min);
            let x_max = x_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let y_min = y_values.iter().copied().fold(f64::INFINITY, f64::min);
            let y_max = y_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

            let x_scale = plot_width / (x_max - x_min) as f32;
            let y_scale = plot_height / (y_max - y_min) as f32;

            // Draw X-axis and Y-axis
            painter.add(Shape::line_segment(
                [
                    pos2(rect.min.x + padding, rect.min.y + padding + plot_height),
                    pos2(rect.min.x + padding + plot_width, rect.min.y + padding + plot_height),
                ],
                Stroke::new(1.0, Color32::BLACK),
            ));
            painter.add(Shape::line_segment(
                [
                    pos2(rect.min.x + padding, rect.min.y + padding),
                    pos2(rect.min.x + padding, rect.min.y + padding + plot_height),
                ],
                Stroke::new(1.0, Color32::BLACK),
            ));

            if graph_data.len() > 1 {
                // Draw line chart
                for pair in graph_data.windows(2) {
                    if let [start_point, end_point] = pair {
                        let start_screen_x = rect.min.x + padding + (start_point.x - x_min) as f32 * x_scale;
                        let start_screen_y = rect.min.y + padding + plot_height - (start_point.y - y_min) as f32 * y_scale;
                        let end_screen_x = rect.min.x + padding + (end_point.x - x_min) as f32 * x_scale;
                        let end_screen_y = rect.min.y + padding + plot_height - (end_point.y - y_min) as f32 * y_scale;

                        painter.add(Shape::line_segment(
                            [
                                pos2(start_screen_x, start_screen_y),
                                pos2(end_screen_x, end_screen_y),
                            ],
                            Stroke::new(2.0, Color32::from_rgb(30, 144, 255)),
                        ));
                    }
                }
            }

            // Draw points
            for point in graph_data {
                let screen_x = rect.min.x + padding + (point.x - x_min) as f32 * x_scale;
                let screen_y = rect.min.y + padding + plot_height - (point.y - y_min) as f32 * y_scale;

                painter.add(Shape::circle_filled(
                    pos2(screen_x, screen_y),
                    4.0,
                    Color32::from_rgb(30, 144, 255),
                ));
            }

            Some(response)
        } else {
            None
        }
    }).inner
}

pub fn draw_flame_graph(ui: &mut egui::Ui, formatted_data: Option<Vec<PlotPoint>>) -> Option<egui::Response> {
    ScrollArea::horizontal().show(ui, |ui| {
        if let Some(graph_data) = &formatted_data {
            //println!("========================{:?}", graph_data);
            let padding = 10.0;
            let available_width = ui.available_width() - (padding * 2.0);
            let available_height = 600.0;
            let block_height = 30.0;

            let (response, painter) = ui.allocate_painter(
                vec2(available_width + padding * 2.0, available_height + padding * 2.0),
                Sense::hover(),
            );

            let rect = response.rect;

            // Calculate max depth and total value for scaling
            let max_depth = graph_data.iter().map(|point| point.depth).fold(0.0, f32::max);
            let total_value = graph_data.iter().map(|point| point.value as f32).sum::<f32>();

            // Draw blocks
            for point in graph_data {
                let block_width = (point.value as f32 / total_value) * available_width;
                let y_position = rect.min.y + padding + (max_depth - point.depth) * block_height;
                let x_position = rect.min.x + padding + (point.x as f32 / total_value) * available_width;

                // Create block rectangle
                let block_rect = Rect::from_min_size(
                    pos2(x_position, y_position),
                    vec2(block_width, block_height - 2.0), // -2 for spacing
                );

                // Generate color based on depth
                let hue = (point.depth * 0.1) % 1.0;
                let color = Color32::from_rgb(
                    (255.0 * hue.sin().abs()) as u8,
                    (255.0 * (hue + 0.33).sin().abs()) as u8,
                    (255.0 * (hue + 0.67).sin().abs()) as u8,
                );

                // Draw block
                painter.rect_filled(block_rect, 2.0, color);

                // Draw label if block is wide enough
                if block_width > 40.0 {
                    painter.text(
                        pos2(x_position + 5.0, y_position + block_height / 2.0),
                        Align2::LEFT_CENTER,
                        &point.label,
                        FontId::default(),
                        Color32::BLACK,
                    );
                }
            }

            Some(response)
        } else {
            None
        }
    }).inner
}

