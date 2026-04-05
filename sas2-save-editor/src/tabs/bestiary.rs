use crate::app::SaveEditorApp;
use eframe::egui;
use egui::{ScrollArea, Ui};
use sas2_save::{BestiaryBeast, SaveData};

impl SaveEditorApp {
    fn add_bestiary_details(&self, ui: &mut Ui, beast: &mut BestiaryBeast) {
        ui.horizontal(|ui| {
            ui.label("Kills:");
            ui.add(
                egui::DragValue::new(&mut beast.kills)
                    .speed(self.config.drag_value_sensitivity)
                    .range(0..=9999),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Deaths:");
            ui.add(
                egui::DragValue::new(&mut beast.deaths)
                    .speed(self.config.drag_value_sensitivity)
                    .range(0..=9999),
            );
        });
        ui.label("Drops:");
        for (drop_idx, drop) in beast.drops.iter_mut().enumerate() {
            ui.checkbox(drop, format!("Drop {}", drop_idx));
        }
    }

    pub fn show_bestiary_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        ScrollArea::vertical()
            .max_height(400.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if let Some(catalog) = &self.monster_catalog {
                    for (idx, beast) in save.bestiary.beasts.iter_mut().enumerate() {
                        let name = catalog
                            .monsters
                            .get(idx)
                            .map(|m| m.name.clone())
                            .unwrap_or_else(|| format!("Beast {}", idx));
                        ui.collapsing(format!("{} (ID: {})", name, idx), |ui| {
                            self.add_bestiary_details(ui, beast);
                        });
                    }
                } else {
                    // No catalog, show numeric IDs only
                    for (idx, beast) in save.bestiary.beasts.iter_mut().enumerate() {
                        ui.collapsing(format!("Beast {}", idx), |ui| {
                            self.add_bestiary_details(ui, beast);
                        });
                    }
                    if let Some(err) = &self.monster_catalog_error {
                        ui.colored_label(
                            egui::Color32::RED,
                            format!("Monster catalog error: {}", err),
                        );
                    }
                }
            });
    }
}
