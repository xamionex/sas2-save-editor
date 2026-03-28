mod config;
mod catalog;
mod app;

use eframe::egui;
use eframe::egui::vec2;
use crate::app::SaveEditorApp;
#[cfg(not(debug_assertions))]
use hide_console::hide_console;

fn main() -> eframe::Result<()> {
    #[cfg(not(debug_assertions))]
    hide_console();

    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "SaS2 Save Editor",
        options,
        Box::new(|_cc| Ok(Box::new(SaveEditorApp::default()))),
    )
}