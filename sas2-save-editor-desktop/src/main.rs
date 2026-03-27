#![allow(unused_imports)]

mod config;
mod catalog;
mod app;

use eframe::egui;
use crate::app::SaveEditorApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "SaS2 Save Editor",
        options,
        Box::new(|_cc| Ok(Box::new(SaveEditorApp::default()))),
    )
}