use eframe::emath::{pos2, vec2, Align2, Rect};
use eframe::epaint::{Color32, FontId, Stroke};
use egui::{RichText, ScrollArea, Sense, Shape};
use egui::epaint::{PathShape};
use crate::charter_csv::PlotPoint;
use crate::charter_utilities::{draw_rotated_text, TextPlacement};

pub fn draw_bar_graph(ui: &mut egui::Ui, formatted_data: Option<Vec<PlotPoint>>) -> Option<egui::Response> {
    if let Some(graph_data) = &formatted_data {
        let bar_width = 75.0;
        let bar_spacing = 2.0;
        let total_width = (graph_data.len() as f32) * (bar_width + bar_spacing);
        let left_padding = 60.0;
        let bottom_padding = 60.0;

        ui.add_space(10.0); // Top padding
        ScrollArea::horizontal()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let available_height: f64 = (ui.available_height() - bottom_padding) as f64;
                let content_size = vec2(
                    total_width.max(ui.available_width()) + left_padding,
                    available_height as f32 + bottom_padding
                );

                let (response, painter) = ui.allocate_painter(
                    content_size,
                    Sense::hover(),
                );

                let rect = response.rect;
                let values: Vec<f64> = graph_data.iter()
                    .map(|data| data.value)
                    .collect();

                let max_value = values.iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(&1.0);

                let label_font = FontId::proportional(16.0);

                painter.text(
                    pos2(rect.min.x + 20.0, rect.center().y),
                    Align2::CENTER_CENTER,
                    "Y",
                    label_font.clone(),
                    Color32::BLACK,
                );

                painter.text(
                    pos2(
                        rect.center().x,
                        rect.max.y - (bottom_padding / 2.0)
                    ),
                    Align2::CENTER_CENTER,
                    "X",
                    label_font,
                    Color32::BLACK,
                );


                let axes_color = Color32::BLACK;
                let axis_thickness = 2.0;

                painter.line_segment(
                    [
                        pos2(rect.min.x + left_padding, rect.max.y - bottom_padding),
                        pos2(rect.min.x + left_padding, rect.min.y),
                    ],
                    Stroke::new(axis_thickness, axes_color),
                );

                painter.line_segment(
                    [
                        pos2(rect.min.x + left_padding, rect.max.y - bottom_padding),
                        pos2(rect.max.x, rect.max.y - bottom_padding),
                    ],
                    Stroke::new(axis_thickness, axes_color),
                );

                for (i, (data, value)) in graph_data.iter().zip(values.iter()).enumerate() {
                    let value_normalized = value / max_value;
                    let height = value_normalized * (available_height - 20.0);
                    let x = rect.min.x + left_padding + (i as f32 * (bar_width + bar_spacing));

                    let bar_rect = Rect::from_min_size(
                        pos2(x, rect.max.y - bottom_padding - height as f32),
                        vec2(bar_width, height as f32),
                    );
                    painter.rect_filled(bar_rect, 0.0, Color32::from_rgb(65, 155, 220));

                    painter.text(
                        pos2(x + bar_width / 2.0, bar_rect.min.y - 5.0),
                        Align2::CENTER_BOTTOM,
                        format!("{:.0}", value),
                        FontId::default(),
                        Color32::BLACK,
                    );

                    let shapes = draw_rotated_text(
                        &painter,
                        rect,
                        &data.label,
                        x,
                        bar_width,
                        90.0,
                        TextPlacement::Middle
                    );
                    ui.painter_at(rect).extend(shapes);
                }

                Some(response)
            }).inner
    } else {
        None
    }
}

pub fn draw_pie_chart(ui: &mut egui::Ui, formatted_data: Option<Vec<PlotPoint>>) -> Option<egui::Response> {
    if let Some(graph_data) = &formatted_data {
        let bottom_padding = 20.0;
        let left_padding = 20.0;
        let legend_width = 200.0;
        let available_width = ui.available_width() - legend_width;
        let available_height = ui.available_height() - 20.0;
        let available_size = available_width.min(available_height);

        let content_size = vec2(
            available_size + legend_width + left_padding,
            available_size + bottom_padding
        );

        let (response, painter) = ui.allocate_painter(
            content_size,
            Sense::hover(),
        );

        let rect = response.rect;
        let center = pos2(
            rect.min.x + (available_size / 2.0),
            rect.center().y
        );
        let radius = (available_size / 2.5).min(available_size / 2.0);

        let total: f64 = graph_data.iter()
            .map(|data| data.value)
            .sum();

        let mut start_angle = 0.0f32;
        let stroke = Stroke::new(1.0, Color32::BLACK);

        let legend_area = Rect::from_min_size(
            pos2(center.x + radius + 20.0, rect.min.y),
            vec2(legend_width - 20.0, available_size)
        );

        let mut legend_ui = ui.child_ui(legend_area, egui::Layout::top_down_justified(egui::Align::LEFT), None);

        ScrollArea::vertical().show(&mut legend_ui, |ui| {
            for (i, data) in graph_data.iter().enumerate() {
                let percentage = (data.value / total) as f32;
                let angle = percentage * std::f32::consts::TAU;

                let colour_set = vec![
                    Color32::from_rgb(255, 0, 0),    // Red
                    Color32::from_rgb(0, 255, 0),    // Green
                    Color32::from_rgb(0, 0, 255),    // Blue
                    Color32::from_rgb(128, 0, 128),  // Purple
                    Color32::from_rgb(255, 192, 203), // Pink
                    Color32::from_rgb(192, 192, 192), // Silver
                    Color32::from_rgb(255, 215, 0),  // Gold
                ];

                let base_color = colour_set[i % colour_set.len()];

                let hue = if i >= colour_set.len() {
                    (i as f32 * 0.618034) % 1.0
                } else {
                    0.0
                };

                let color = Color32::from_rgb(
                    (base_color.r() as f32 * hue.cos().abs() + 255.0 * (1.0 - hue.cos().abs())) as u8,
                    (base_color.g() as f32 * hue.sin().abs() + 255.0 * (1.0 - hue.sin().abs())) as u8,
                    (base_color.b() as f32 * (1.0 - hue).abs() + 255.0 * hue.abs()) as u8,
                );

                let mut points = Vec::new();
                points.push(center);

                let steps = 32;
                for j in 0..=steps {
                    let current_angle = start_angle + (angle * j as f32 / steps as f32);
                    points.push(pos2(
                        center.x + radius * current_angle.cos(),
                        center.y + radius * current_angle.sin(),
                    ));
                }

                painter.add(Shape::Path(PathShape::convex_polygon(
                    points,
                    color,
                    Stroke::NONE,
                )));
                // mark new section begin
                painter.add(Shape::LineSegment {
                    points: [
                        center,
                        pos2(
                            center.x + radius * start_angle.cos(),
                            center.y + radius * start_angle.sin(),
                        )
                    ],
                    stroke,
                });

                // legend
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, color);
                    ui.label(RichText::new(format!("{}: {:.1}%", data.label, percentage * 100.0))
                        .size(12.0));
                });

                start_angle += angle;
            }
            // close section
            painter.add(Shape::LineSegment {
                points: [
                    center,
                    pos2(
                        center.x + radius * (start_angle).cos(),
                        center.y + radius * (start_angle).sin(),
                    )
                ],
                stroke,
            });
        });

        Some(response)
    } else {
        None
    }
}


pub fn draw_histogram(ui: &mut egui::Ui, formatted_data: Option<Vec<PlotPoint>>) -> Option<egui::Response> {
    ScrollArea::horizontal()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            if let Some(graph_data) = &formatted_data {
                let left_padding = 60.0;
                let bottom_padding = 60.0;
                let top_padding = 20.0;
                let bar_spacing = 2.0;
                let values: Vec<f64> = graph_data.iter()
                    .map(|data| data.value)
                    .collect();
                let max_value = values.iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(&1.0);
                let bin_count = 10;
                let min_value = values.iter().cloned().fold(f64::INFINITY, f64::min);
                let bin_width = (max_value - min_value) / bin_count as f64;
                let mut bins = vec![0.0; bin_count];
                let bar_width = ui.available_width() / bin_count as f32;
                let total_width = (bar_width + bar_spacing) * bin_count as f32;
                let available_height = (ui.available_height() - bottom_padding - top_padding) as f64;

                for &value in &values {
                    let bin_index = ((value - min_value) / bin_width).floor() as usize;
                    let bin_index = bin_index.min(bin_count - 1);
                    bins[bin_index] += 1.0;
                }

                let content_size = vec2(
                    total_width.max(ui.available_width()) + left_padding,
                    (available_height as f32) + bottom_padding + top_padding
                );

                let (response, painter) = ui.allocate_painter(
                    content_size,
                    Sense::hover(),
                );

                let rect = response.rect;

                painter.text(
                    pos2(rect.min.x + 15.0, rect.center().y),
                    Align2::CENTER_CENTER,
                    "Freq",
                    FontId::proportional(14.0),
                    Color32::BLACK,
                );

                let stroke = Stroke::new(1.0, Color32::BLACK);
                painter.line_segment(
                    [
                        pos2(rect.min.x + left_padding, rect.min.y + top_padding),
                        pos2(rect.min.x + left_padding, rect.max.y - bottom_padding),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        pos2(rect.min.x + left_padding, rect.max.y - bottom_padding),
                        pos2(rect.max.x, rect.max.y - bottom_padding),
                    ],
                    stroke,
                );

                // Draw bins
                for (i, &frequency) in bins.iter().enumerate() {
                    let frequency_normalized = frequency / bins.iter().cloned().fold(1.0, f64::max);
                    let height = frequency_normalized * available_height;

                    let bar_rect = Rect::from_min_size(
                        pos2(
                            rect.min.x + left_padding + (i as f32 * (bar_width + bar_spacing)),
                            rect.max.y - bottom_padding - (height as f32)
                        ),
                        vec2(bar_width, height as f32),
                    );

                    painter.rect_filled(bar_rect, 0.0, Color32::from_rgb(135, 206, 250));

                    painter.text(
                        pos2(
                            rect.min.x + left_padding + (i as f32 * (bar_width + bar_spacing)) + bar_width / 2.0,
                            rect.max.y - bottom_padding - (height as f32) - 5.0
                        ),
                        Align2::CENTER_BOTTOM,
                        format!("{:.0}", frequency),
                        FontId::default(),
                        Color32::BLACK,
                    );

                    let shapes = draw_rotated_text(
                        &painter,
                        rect,
                        &graph_data[i].label,
                        rect.min.x + left_padding + (i as f32 * (bar_width + bar_spacing)) + bar_width / 4.0,
                        bar_width,
                        16.0,
                        TextPlacement::Bottom
                    );
                    painter.extend(shapes);

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
            let left_padding = 10.0;
            let bottom_padding = 20.0;
            let available_height: f64 = (ui.available_height() - bottom_padding) as f64;
            let content_size = vec2(
                ui.available_width() + left_padding,
                available_height as f32 + bottom_padding
            );

            let (response, painter) = ui.allocate_painter(
                content_size,
                Sense::hover(),
            );

            let rect = response.rect;
            let plot_width = rect.width() - left_padding * 2.0;
            let plot_height = available_height as f32;

            let x_values: Vec<f64> = graph_data.iter().map(|data| data.x).collect();
            let y_values: Vec<f64> = graph_data.iter().map(|data| data.y).collect();

            let x_min = x_values.iter().copied().fold(f64::INFINITY, f64::min);
            let x_max = x_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let y_min = y_values.iter().copied().fold(f64::INFINITY, f64::min);
            let y_max = y_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

            let x_scale = plot_width / (x_max - x_min) as f32;
            let y_scale = plot_height / (y_max - y_min) as f32;

            painter.add(Shape::line_segment(
                [
                    pos2(rect.min.x + left_padding, rect.min.y + left_padding + plot_height),
                    pos2(rect.min.x + left_padding + plot_width, rect.min.y + left_padding + plot_height),
                ],
                Stroke::new(1.0, Color32::BLACK),
            ));
            painter.add(Shape::line_segment(
                [
                    pos2(rect.min.x + left_padding, rect.min.y + left_padding),
                    pos2(rect.min.x + left_padding, rect.min.y + left_padding + plot_height),
                ],
                Stroke::new(1.0, Color32::BLACK),
            ));

            for point in graph_data {
                let screen_x = rect.min.x + left_padding + (point.x - x_min) as f32 * x_scale;
                let screen_y = rect.min.y + left_padding + plot_height - (point.y - y_min) as f32 * y_scale;

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
            let left_padding = 10.0;
            let bottom_padding = 20.0;
            let available_height: f64 = (ui.available_height() - bottom_padding) as f64;
            let content_size = vec2(
                ui.available_width() + left_padding,
                available_height as f32 + bottom_padding
            );

            let (response, painter) = ui.allocate_painter(
                content_size,
                Sense::hover(),
            );

            let rect = response.rect;
            let plot_width = rect.width() - left_padding * 2.0;
            let plot_height = available_height as f32;

            let x_values: Vec<f64> = graph_data.iter().map(|data| data.x).collect();
            let y_values: Vec<f64> = graph_data.iter().map(|data| data.y).collect();

            let x_min = x_values.iter().copied().fold(f64::INFINITY, f64::min);
            let x_max = x_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let y_min = y_values.iter().copied().fold(f64::INFINITY, f64::min);
            let y_max = y_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

            let x_scale = plot_width / (x_max - x_min) as f32;
            let y_scale = plot_height / (y_max - y_min) as f32;

            painter.add(Shape::line_segment(
                [
                    pos2(rect.min.x + left_padding, rect.min.y + left_padding + plot_height),
                    pos2(rect.min.x + left_padding + plot_width, rect.min.y + left_padding + plot_height),
                ],
                Stroke::new(1.0, Color32::BLACK),
            ));
            painter.add(Shape::line_segment(
                [
                    pos2(rect.min.x + left_padding, rect.min.y + left_padding),
                    pos2(rect.min.x + left_padding, rect.min.y + left_padding + plot_height),
                ],
                Stroke::new(1.0, Color32::BLACK),
            ));

            if graph_data.len() > 1 {
                for pair in graph_data.windows(2) {
                    if let [start_point, end_point] = pair {
                        let start_screen_x = rect.min.x + left_padding + (start_point.x - x_min) as f32 * x_scale;
                        let start_screen_y = rect.min.y + left_padding + plot_height - (start_point.y - y_min) as f32 * y_scale;
                        let end_screen_x = rect.min.x + left_padding + (end_point.x - x_min) as f32 * x_scale;
                        let end_screen_y = rect.min.y + left_padding + plot_height - (end_point.y - y_min) as f32 * y_scale;

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
                let screen_x = rect.min.x + left_padding + (point.x - x_min) as f32 * x_scale;
                let screen_y = rect.min.y + left_padding + plot_height - (point.y - y_min) as f32 * y_scale;

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
            let left_padding = 10.0;
            let bottom_padding = 10.0;
            let max_depth = graph_data.iter().map(|point| point.depth).fold(0.0, f32::max);
            let total_value = graph_data.iter().map(|point| point.value as f32).sum::<f32>();
            let total_width = total_value + (left_padding * 2.0);
            let available_height: f32 = ui.available_height() -  bottom_padding;
            let content_size = vec2(
                total_width.max(ui.available_width()) + left_padding,
                available_height + bottom_padding
            );

            let (response, painter) = ui.allocate_painter(
                content_size,
                Sense::hover(),
            );

            let rect = response.rect;
            let block_height = 30.0;
            
            for point in graph_data {
                let block_width = (point.value as f32 / total_value) * (content_size.x - left_padding * 2.0);
                let y_position = rect.min.y + left_padding + (max_depth - point.depth) * block_height;
                let x_position = rect.min.x + left_padding + (point.x as f32 / total_value) * (content_size.x - left_padding * 2.0);

                let block_rect = Rect::from_min_size(
                    pos2(x_position, y_position),
                    vec2(block_width, block_height - 2.0),
                );

                let hue = (point.depth * 0.1) % 1.0;
                let color = Color32::from_rgb(
                    (255.0 * hue.sin().abs()) as u8,
                    (255.0 * (hue + 0.33).sin().abs()) as u8,
                    (255.0 * (hue + 0.67).sin().abs()) as u8,
                );

                painter.rect_filled(block_rect, 2.0, color);

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

