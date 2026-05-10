use std::collections::HashMap;
use crate::app::SaveEditor;
use eframe::egui;
use egui::Ui;
use sas2_parser::{BestiaryBeast, SaveData};

impl SaveEditor {
    pub fn show_bestiary_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        if self.monster_catalog.is_none() {
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (idx, beast) in save.bestiary.beasts.iter_mut().enumerate() {
                        ui.collapsing(format!("Beast {}", idx), |ui| {
                            self.add_bestiary_details(ui, beast);
                        });
                    }
                });
            if let Some(err) = &self.monster_catalog_error {
                ui.colored_label(egui::Color32::RED, format!("Monster catalog error: {}", err));
            }
            return;
        }

        let full_width = ui.available_width();
        let min_size = 250.0;
        let panel_width = if self.config.bestiary_panel_width > 0.0 {
            self.config.bestiary_panel_width.max(min_size)
        } else {
            full_width * 0.4
        };

        // Right panel: selected beast details
        let right_panel = egui::Panel::right("bestiary_details")
            .resizable(true)
            .default_size(panel_width)
            .min_size(min_size)
            .max_size(full_width * 0.8)
            .size_range(min_size..=full_width * 0.8)
            .show_inside(ui, |ui| {
                ui.set_min_width(ui.available_width());

                if let Some(idx) = self.selected_bestiary_beast {
                    if idx < save.bestiary.beasts.len() {
                        let beast = &mut save.bestiary.beasts[idx];

                        if let Some(catalog) = &self.monster_catalog {
                            if let Some(def) = catalog.monsters.get(idx) {
                                if let Some(tex) = self.monster_texture_cache.get_or_assemble(
                                    ui.ctx(),
                                    &def.def,
                                    &def.texture,
                                ) {
                                    ui.add(
                                        egui::Image::from_texture(&tex)
                                            .fit_to_exact_size(egui::vec2(96.0, 96.0)),
                                    );
                                } else {
                                    ui.add_sized([96.0, 96.0], egui::Label::new(""));
                                }
                            }
                        }

                        if let Some(catalog) = &self.monster_catalog {
                            if let Some(def) = catalog.monsters.get(idx) {
                                ui.heading(&def.titles[0]);
                            } else {
                                ui.heading(format!("Beast {}", idx));
                            }
                        } else {
                            ui.heading(format!("Beast {}", idx));
                        }

                        ui.separator();
                        self.add_bestiary_details(ui, beast);
                    }
                } else {
                    ui.label("Select a beast to edit.");
                }
            });

        let actual_width = right_panel.response.rect.width();
        if (actual_width - self.config.bestiary_panel_width).abs() > 0.1 {
            self.config.bestiary_panel_width = actual_width;
            self.config_save_timer = 0.25;
        }

        // Central panel: monster grid
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.set_min_width(200.0);
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.bestiary_search_filter);
            });
            ui.add_space(4.0);

            let filter = self.bestiary_search_filter.to_lowercase();
            let catalog = self.monster_catalog.as_ref().unwrap();

            // filtered entries: (index, name, optional owned texture handle)
            let filtered: Vec<(usize, &str, Option<egui::TextureHandle>)> = save
                .bestiary
                .beasts
                .iter()
                .enumerate()
                .filter(|(idx, _beast)| {
                    let name = catalog.monsters.get(*idx)
                        .map(|m| m.titles[0].as_str())
                        .unwrap_or("");
                    name.to_lowercase().contains(&filter) || idx.to_string().contains(&filter)
                })
                .map(|(idx, _beast)| {
                    let tex = catalog.monsters.get(idx)
                        .and_then(|def| self.monster_texture_cache.get_or_assemble(
                            ui.ctx(),
                            &def.def,
                            &def.texture,
                        ));
                    let name = catalog.monsters.get(idx)
                        .map(|m| m.titles[0].as_str())
                        .unwrap_or("");
                    (idx, name, tex)
                })
                .collect();

            // Group by type/subtype
            let mut grouped: HashMap<String, Vec<(usize, &str, Option<egui::TextureHandle>)>> = HashMap::new();
            for (idx, name, tex) in filtered {
                let cat = catalog.monsters.get(idx)
                    .map(|m| format!("Type {} - SubType {}", m.type_, m.sub_type))
                    .unwrap_or_else(|| "Unknown".to_string());
                grouped.entry(cat).or_default().push((idx, name, tex));
            }

            let mut categories: Vec<_> = grouped.keys().cloned().collect();
            categories.sort();

            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for cat in categories {
                        let entries = grouped.get(&cat).unwrap();
                        ui.style_mut().interaction.selectable_labels = false;
                        ui.label(egui::RichText::new(&cat).strong());

                        egui::Grid::new(&cat).spacing([8.0, 8.0]).show(ui, |ui| {
                            for (orig_idx, name, tex) in entries {
                                ui.vertical(|ui| {
                                    let response = if let Some(tex) = tex {
                                        ui.add(egui::Button::image(
                                            egui::Image::from_texture(&tex.clone())
                                                .fit_to_exact_size(egui::vec2(self.config.item_icon_size, self.config.item_icon_size)),
                                        ))
                                    } else {
                                        ui.allocate_response(
                                            egui::vec2(self.config.item_icon_size, self.config.item_icon_size),
                                            egui::Sense::click(),
                                        )
                                    };
                                    if response.clicked() {
                                        self.selected_bestiary_beast = Some(*orig_idx);
                                    }
                                    ui.set_max_width(response.rect.width());
                                    for word in name.split_whitespace() {
                                        ui.add(
                                            egui::Label::new(egui::RichText::new(word).size(self.config.item_font_size))
                                                .wrap_mode(egui::TextWrapMode::Truncate)
                                                .halign(egui::Align::Center)
                                                .show_tooltip_when_elided(false),
                                        );
                                    }
                                });
                            }
                        });
                        ui.add_space(8.0);
                    }
                });
        });
    }

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
}