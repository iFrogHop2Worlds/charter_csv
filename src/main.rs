#![feature(str_as_str)]
pub use eframe::{egui, App};
use egui::{Context, ViewportBuilder};
pub mod csvqb;
pub mod charter_utilities;
pub mod charter_graphs;
mod charter_csv;
pub mod session;
mod db_manager;
mod cir_adapters;
pub mod components;

use charter_csv::CharterCsvApp;

fn main() {
    let ctx = Context::default();
    let mut size = ctx.used_size();
    size.x = 1280.0;
    size.y = 720.0;
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_resizable(true)
            .with_inner_size(size)
            .with_icon(charter_utilities::load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "charter csv alpha 0.1.0",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_theme(egui::Theme::Light);
            Ok(Box::new(CharterCsvApp::default()))
        }),
    )
        .expect("Unexpected error in running the application");
}
















