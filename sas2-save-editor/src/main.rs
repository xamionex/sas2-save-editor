mod config;
mod catalog;
mod app;

use crate::app::SaveEditorApp;
#[cfg(not(debug_assertions))]
use hide_console::hide_console;

fn main() -> eframe::Result<()> {
    #[cfg(not(debug_assertions))]
    hide_console();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--hide-lootdef-logging") {
        sas2_save::set_loot_logging_enabled(false);
    }
    if args.iter().any(|arg| arg == "--hide-monster-logging") {
        sas2_save::set_monster_logging_enabled(false);
    }

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "SaS2 Save Editor",
        options,
        Box::new(|_cc| Ok(Box::new(SaveEditorApp::default()))),
    )
}