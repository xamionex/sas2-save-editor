use crate::app::SaveEditorApp;
use eframe::egui;
use egui::Ui;
use rfd::FileDialog;
use sas2_save::SaveData;
use std::fs;

impl SaveEditorApp {
    /// Sanitizes a modded save for vanilla compatibility:
    /// - Drops items whose loot_idx is outside the vanilla catalog
    /// - Clamps upgrade values to the vanilla maximum of 10
    /// - Clamps stack sizes to 999
    /// - Clears all equipped slots to avoid dangling indices
    pub fn sanitize_for_vanilla(&self, save: &mut SaveData) {
        let Some(catalog) = &self.catalog else {
            eprintln!("Cannot sanitize: loot catalog not loaded");
            return;
        };
        let max_valid_idx = catalog.loot_defs.len();

        let mut new_inventory = Vec::new();
        for item in &save.equipment.inventory_items {
            if (item.loot_idx as usize) < max_valid_idx && item.count > 0 {
                let mut sanitized = item.clone();
                if sanitized.upgrade > 10 {
                    sanitized.upgrade = 10;
                }
                if sanitized.count > 999 {
                    sanitized.count = 999;
                }
                new_inventory.push(sanitized);
            }
        }
        save.equipment.inventory_items = new_inventory;

        // Unequip everything, better than crashing on a bad index
        for slot in &mut save.equipment.equipped_items {
            *slot = -1;
        }
    }

    fn convert_to_vanilla(&mut self, save: &mut SaveData, target_version: i32) {
        if save.version <= 100 {
            self.error_message = Some("Save is already vanilla".to_string());
            return;
        }

        let start_dir = self
            .file_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf());

        let mut dialog = FileDialog::new()
            .add_filter("Salt and Sacrifice Save", &["slv"])
            .set_file_name("converted.slv");
        if let Some(dir) = start_dir {
            dialog = dialog.set_directory(dir);
        }

        let Some(path) = dialog.save_file() else {
            self.error_message = Some("Save dialog cancelled".to_string());
            return;
        };

        self.sanitize_for_vanilla(save);

        match save.to_vanilla_bytes(target_version) {
            Ok(data) => {
                if let Err(e) = fs::write(&path, data) {
                    self.error_message = Some(format!("Failed to write file: {}", e));
                } else {
                    self.error_message = None;
                    // Reload the freshly-written vanilla save into the editor
                    if let Ok(raw) = fs::read(&path) {
                        if let Ok(new_save) = SaveData::from_bytes(&raw) {
                            self.save_data = Some(new_save);
                            self.file_path = Some(path);
                            self.hash_edit_string.clear();
                            self.active_tab = super::Tab::Stats;
                            self.conversion_just_happened = true;
                        }
                    }
                }
            }
            Err(e) => self.error_message = Some(format!("Conversion failed: {}", e)),
        }
    }

    pub fn show_convert_save_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        let modded = if save.version <= 100 {
            "Vanilla"
        } else {
            "Modded"
        };
        ui.label(format!(
            "Current save version: {} ({})",
            save.version, modded
        ));

        // Version selector is stubbed out, I don't know if anyone would even play v18
        //ui.label("Select target vanilla version:");
        //ui.horizontal(|ui| {
        //    ui.selectable_value(&mut self.conversion_target_version, 18, "Version 18");
        //    ui.selectable_value(&mut self.conversion_target_version, 19, "Version 19");
        //});

        ui.separator();
        ui.label("Warning: Mod only item data will be lost. (artifact seed, rarity, etc.)");
        ui.label("The resulting save should be compatible with the vanilla game.");
        ui.label("Note: Everything will be unequipped as to not cause missing index errors.");
        ui.label("This was only tested with Saltguard. Make backups.");

        if ui.button("Convert and Save As...").clicked() {
            self.convert_to_vanilla(save, self.conversion_target_version);
        }

        ui.separator();

        if ui.button("Sanitize Current Save").clicked() {
            self.sanitize_for_vanilla(save);
        }
        ui.label("This removes invalid item count items and unequips everything.");
        ui.label(
            "It's intended to be used when you encounter your save crashing\n\
             Example: When you remove an item from your inventory.",
        );
    }
}
