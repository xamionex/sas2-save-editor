use crate::app::SaveEditorApp;
use eframe::egui;
use egui::Ui;
use sas2_save::types::faction::PlayerFaction;
use sas2_save::types::ng_level;
use sas2_save::SaveData;

impl SaveEditorApp {
    pub fn show_faction_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        let current_faction = PlayerFaction::from_flags(&save.flags.flags);
        let mut selected = current_faction;

        ui.label("Faction:");
        egui::ComboBox::from_label("")
            .selected_text(current_faction.name())
            .show_ui(ui, |ui| {
                for faction in PlayerFaction::get_all() {
                    ui.selectable_value(&mut selected, *faction, faction.name());
                }
            });

        if selected != current_faction {
            selected.apply_to_flags(&mut save.flags.flags);
            // Recompute ng_level in case a faction flag overlaps with an NG flag
            ng_level::update_ng_level(&mut save.flags);
        }

        ui.separator();
        ui.label("Faction is determined by the presence of certain flags:");
        ui.label("dawnlight_saved -> Dawnlight Order");
        ui.label("shroud_saved -> Shrouded Alliance");
        ui.label("blueheart_saved -> Blueheart Runners");
        ui.label("sheriff_saved -> Sheriff Inquisitors");
        ui.label("oath_saved -> Oathbound Watchers");
        ui.label("chaos_saved -> Chaos Eaters");
        ui.label("(No flag means No Faction)");
    }
}
