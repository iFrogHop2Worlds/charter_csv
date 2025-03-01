#![feature(str_as_str)]

pub use eframe::{egui, App};
use egui::{Context, ViewportBuilder};
pub mod csvqb;
pub mod utilities;
mod charter_csv;
use charter_csv::CharterCsv;

fn main() {
    let ctx = Context::default();
    let mut size = ctx.used_size();
    size.x = 780.00;
    size.y = 420.00;
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
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
















