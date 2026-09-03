use crate::app::SaveEditor;
use eframe::egui;
use egui::Ui;
use sas2_parser::{BestiaryBeast, SaveData};
use std::collections::HashMap;

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
                ui.colored_label(
                    egui::Color32::RED,
                    format!("Monster catalog error: {}", err),
                );
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

                // Multi-selection: edit the common fields of every selected beast.
                let multi: Vec<usize> = self.selected_bestiary_beasts.iter().copied().collect();
                if multi.len() > 1 {
                    ui.heading("Edit Selected Beasts");
                    ui.label(format!("{} beasts selected", multi.len()));
                    ui.add_space(4.0);
                    // Kills / Deaths apply to every selected beast.
                    let mut kills_changed = false;
                    let mut deaths_changed = false;
                    {
                        let beasts = &mut save.bestiary.beasts;
                        let first_kills = beasts.get(multi[0]).map(|b| b.kills).unwrap_or(0);
                        let first_deaths = beasts.get(multi[0]).map(|b| b.deaths).unwrap_or(0);
                        let mut kills = first_kills;
                        let mut deaths = first_deaths;
                        ui.horizontal(|ui| {
                            ui.label("Kills:");
                            kills_changed = ui
                                .add(
                                    egui::DragValue::new(&mut kills)
                                        .speed(self.config.drag_value_sensitivity)
                                        .range(0..=9999),
                                )
                                .changed();
                            ui.label("Deaths:");
                            deaths_changed = ui
                                .add(
                                    egui::DragValue::new(&mut deaths)
                                        .speed(self.config.drag_value_sensitivity)
                                        .range(0..=9999),
                                )
                                .changed();
                        });
                        if kills_changed {
                            for &idx in &multi {
                                if let Some(b) = beasts.get_mut(idx) {
                                    b.kills = kills;
                                }
                            }
                        }
                        if deaths_changed {
                            for &idx in &multi {
                                if let Some(b) = beasts.get_mut(idx) {
                                    b.deaths = deaths;
                                }
                            }
                        }
                    }
                    ui.separator();
                }

                if multi.len() <= 1 {
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
                ui.separator();
                crate::tabs::multisel::mouse_help_button(ui, &[]);
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
                    let name = catalog
                        .monsters
                        .get(*idx)
                        .map(|m| m.titles[0].as_str())
                        .unwrap_or("");
                    name.to_lowercase().contains(&filter) || idx.to_string().contains(&filter)
                })
                .map(|(idx, _beast)| {
                    let tex = catalog.monsters.get(idx).and_then(|def| {
                        self.monster_texture_cache
                            .get_or_assemble(ui.ctx(), &def.def, &def.texture)
                    });
                    let name = catalog
                        .monsters
                        .get(idx)
                        .map(|m| m.titles[0].as_str())
                        .unwrap_or("");
                    (idx, name, tex)
                })
                .collect();

            // Group by type/subtype
            let mut grouped: HashMap<String, Vec<(usize, &str, Option<egui::TextureHandle>)>> =
                HashMap::new();
            for (idx, name, tex) in filtered {
                let cat = catalog
                    .monsters
                    .get(idx)
                    .map(|m| format!("Type {} - SubType {}", m.type_, m.sub_type))
                    .unwrap_or_else(|| "Unknown".to_string());
                grouped.entry(cat).or_default().push((idx, name, tex));
            }

            let mut categories: Vec<_> = grouped.keys().cloned().collect();
            categories.sort();

            // Selection gesture state (click / ctrl+click / shift+click / shift+drag box).
            let mut gsel = std::mem::take(&mut self.bestiary_grid_sel);
            gsel.begin(ui);

            // Full display order (all filtered beasts, not just visible ones) so
            // shift+click ranges work across scrolled-out entries.
            gsel.display_order.clear();
            for cat in &categories {
                for (idx, _, _) in &grouped[cat] {
                    gsel.display_order.push(*idx);
                }
            }

            egui::ScrollArea::both()
                .scroll_source(crate::tabs::multisel::grid_scroll_source(ui))
                .auto_shrink([false; 2])
                .show_viewport(ui, |ui, viewport| {
                    // Only entries whose x-range intersects the visible viewport are laid out each frame.
                    // Culled entries still advance the grid cursor via allocate_space with the exact cell size, so positions, row heights and the scrollbar stay exact while the widget count stays proportional to the viewport.
                    let icon_size = self.config.item_icon_size;
                    let label_h = ui.fonts_mut(|f| {
                        f.row_height(&egui::FontId::proportional(self.config.grid_font_size))
                    });
                    let spacing_y = ui.spacing().item_spacing.y;
                    let pad_x = 2.0 * ui.spacing().button_padding.x;
                    let pad_y = 2.0 * ui.spacing().button_padding.y;
                    let overscan = 3.0 * (icon_size + pad_x);
                    let vp_min = viewport.min.x - overscan;
                    let vp_max = viewport.max.x + overscan;

                    for cat in categories {
                        let entries = grouped.get(&cat).unwrap();
                        ui.style_mut().interaction.selectable_labels = false;
                        ui.label(egui::RichText::new(&cat).strong());

                        egui::Grid::new(&cat).spacing([8.0, 8.0]).show(ui, |ui| {
                            let mut x = 0.0f32;
                            for (orig_idx, name, tex) in entries {
                                let word_count = name.split_whitespace().count();
                                // Image buttons are icon_size + button frame margins; placeholders are icon_size.
                                let item_w = if tex.is_some() {
                                    icon_size + pad_x
                                } else {
                                    icon_size
                                };
                                let item_h = if tex.is_some() {
                                    icon_size + pad_y
                                } else {
                                    icon_size
                                } + word_count as f32 * (label_h + spacing_y);
                                let start = x;
                                let end = x + item_w;
                                x = end + 8.0;
                                if end < vp_min || start > vp_max {
                                    ui.allocate_space(egui::vec2(item_w, item_h));
                                    continue;
                                }

                                ui.vertical(|ui| {
                                    let response = if let Some(tex) = tex {
                                        ui.add(egui::Button::image(
                                            egui::Image::from_texture(&tex.clone())
                                                .fit_to_exact_size(egui::vec2(
                                                    icon_size, icon_size,
                                                )),
                                        ))
                                    } else {
                                        ui.allocate_response(
                                            egui::vec2(icon_size, icon_size),
                                            egui::Sense::click(),
                                        )
                                    };
                                    if response.clicked() {
                                        // The gesture helper handles selection; this branch
                                        // is only for the plain single-select path.
                                    }
                                    gsel.cell(response.rect, *orig_idx);
                                    let is_sel = self.selected_bestiary_beast == Some(*orig_idx)
                                        || self.selected_bestiary_beasts.contains(orig_idx)
                                        || gsel.is_box_hit(orig_idx);
                                    crate::tabs::multisel::paint_sel_outline(
                                        ui,
                                        response.rect,
                                        is_sel,
                                    );
                                    ui.set_max_width(response.rect.width());
                                    for word in name.split_whitespace() {
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(word)
                                                    .size(self.config.grid_font_size)
                                                    .color(if is_sel {
                                                        egui::Color32::LIGHT_GREEN
                                                    } else {
                                                        ui.visuals().text_color()
                                                    }),
                                            )
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

                    gsel.update_target();
                    gsel.paint(ui);
                });

            gsel.end(
                ui,
                &mut self.selected_bestiary_beasts,
                &mut self.selected_bestiary_beast,
            );
            self.bestiary_grid_sel = gsel;
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
