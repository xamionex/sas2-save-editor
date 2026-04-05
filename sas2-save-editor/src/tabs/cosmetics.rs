use crate::app::SaveEditorApp;
use eframe::egui;
use egui::Ui;
use sas2_save::cosmetics::{
    AncestryCatalog, BeardCatalog, ClassCatalog, ColorCatalog, CrimeCatalog, EyeCatalog,
    HairCatalog, SexCatalog,
};
use sas2_save::SaveData;

impl SaveEditorApp {
    pub fn show_cosmetics_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        type NameFn = fn(usize) -> Option<&'static str>;

        // Hair has a custom ordering rather than the plain 0..len() range
        let hair_choices: Vec<usize> = HairCatalog::get_ordered_indices();

        for slot_idx in 0..save.cosmetics.len() {
            let value = &mut save.cosmetics[slot_idx];

            let (label, name_fn, choices): (&str, NameFn, Vec<usize>) = match slot_idx {
                0 => (
                    "Sex",
                    SexCatalog::name as NameFn,
                    (0..SexCatalog::len()).collect(),
                ),
                1 => (
                    "Ancestry",
                    AncestryCatalog::name as NameFn,
                    (0..AncestryCatalog::len()).collect(),
                ),
                2 => (
                    "Eye Color",
                    EyeCatalog::name as NameFn,
                    (0..EyeCatalog::len()).collect(),
                ),
                3 => ("Hair", HairCatalog::name as NameFn, hair_choices.clone()),
                4 => (
                    "Hair Color",
                    ColorCatalog::name as NameFn,
                    (0..ColorCatalog::len()).collect(),
                ),
                5 => (
                    "Beard",
                    BeardCatalog::name as NameFn,
                    (0..BeardCatalog::len()).collect(),
                ),
                6 => (
                    "Beard Color",
                    ColorCatalog::name as NameFn,
                    (0..ColorCatalog::len()).collect(),
                ),
                7 => (
                    "Eyebrow Color",
                    ColorCatalog::name as NameFn,
                    (0..ColorCatalog::len()).collect(),
                ),
                8 => (
                    "Class",
                    ClassCatalog::name as NameFn,
                    (0..ClassCatalog::len()).collect(),
                ),
                9 => (
                    "Crime",
                    CrimeCatalog::name as NameFn,
                    (0..CrimeCatalog::len()).collect(),
                ),
                10 => ("Unused", (|_| None) as NameFn, Vec::new()),
                _ => continue,
            };

            ui.horizontal(|ui| {
                ui.label(format!("{}:", label));

                if !choices.is_empty() {
                    // Each slot needs its own push_id so the combo boxes don't share state
                    ui.push_id(slot_idx, |ui| {
                        let selected_text = name_fn(*value as usize)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("{}", *value));

                        egui::ComboBox::from_label("")
                            .selected_text(selected_text)
                            .show_ui(ui, |ui| {
                                for &choice_idx in &choices {
                                    let text = name_fn(choice_idx)
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| format!("{}", choice_idx));
                                    ui.selectable_value(value, choice_idx as i32, text);
                                }
                            });
                    });

                    ui.add_space(8.0);
                    // Show the raw numeric index alongside the name for reference
                    ui.colored_label(egui::Color32::GRAY, format!("{}", *value));
                } else {
                    // Unused slot, bare drag value
                    ui.add(
                        egui::DragValue::new(value)
                            .speed(self.config.drag_value_sensitivity)
                            .range(0..=999),
                    );
                }
            });
        }
    }
}
