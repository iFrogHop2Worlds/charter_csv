use egui::epaint::TextShape;
use egui::{emath, pos2, vec2, Color32, FontId, Painter, Rect, Shape, Stroke, WidgetText};
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