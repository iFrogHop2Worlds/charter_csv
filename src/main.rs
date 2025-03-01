#![feature(str_as_str)]

pub use eframe::{egui, App};
use egui::{Context, ViewportBuilder};
pub mod csvqb;
pub mod charter_utilities;
mod charter_csv;
use charter_csv::CharterCsv;

fn main() {
    let ctx = Context::default();
    let mut size = ctx.used_size();
    size.x = 1211.00;
    size.y = 720.00;
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_resizable(true)
            .with_inner_size(size)
            .with_icon(charter_utilities::load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "CharterCSV",
        options,
        Box::new(|cc| {
            // let mut visuals = cc.egui_ctx.style().visuals.clone();
            // visuals.window_fill = egui::Color32::from_rgb(32, 32, 32);
            // visuals.panel_fill = egui::Color32::from_rgb(32, 32, 32);
            // visuals.override_text_color = Some(egui::Color32::BLACK);
            // cc.egui_ctx.set_visuals(visuals);

            Ok(Box::new(CharterCsv::default()))
        }),
    )
        .expect("Unexpected error in running the application");
}
















