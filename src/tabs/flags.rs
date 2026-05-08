use crate::app::SaveEditorApp;
use eframe::egui;
use egui::{ScrollArea, Ui};
use sas2_save::types::ng_level;
use sas2_save::SaveData;

impl SaveEditorApp {
    pub fn show_flags_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        ui.label("Flags:");

        ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            let mut to_remove = None;
            for (i, flag) in save.flags.flags.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(flag);
                    if ui.button("🗑").clicked() {
                        to_remove = Some(i);
                    }
                });
            }
            if let Some(i) = to_remove {
                save.flags.flags.remove(i);
                ng_level::update_ng_level(&mut save.flags);
            }
        });

        ui.horizontal(|ui| {
            if ui.button("Add Flag").clicked() {
                save.flags.flags.push(String::new());
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Bounty Seed:");
            ui.add(
                egui::DragValue::new(&mut save.flags.bounty_seed)
                    .speed(self.config.drag_value_sensitivity)
                    .range(0..=999999),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Bounties Complete (bitmask):");
            ui.add(
                egui::DragValue::new(&mut save.flags.bounties_complete)
                    .speed(self.config.drag_value_sensitivity)
                    .range(0..=999999),
            );
        });

        self.add_ng_level_label(ui, save);
        ui.label("Note: NG level is derived from flags.");
        ui.label(
            "Flags that preserve across NG cycles: v$t_AREA_NOWHERE, dawnlight_saved, \
             shroud_saved, blueheart_saved, oath_saved, sheriff_saved, chaos_saved. \
             The flag $1ntr0 is automatically added if missing.",
        );
    }
}
