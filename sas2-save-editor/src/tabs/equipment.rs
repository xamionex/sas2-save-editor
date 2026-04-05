use crate::app::SaveEditorApp;
use crate::atlas::ItemAtlas;
use crate::tabs::EquipmentSubTab;
use eframe::egui;
use egui::{Response, ScrollArea, Ui};
use sas2_save::loot_catalog::LootDef;
use sas2_save::{loot_names, Item, SaveData};

/// Draw one icon button from the atlas.  If either the atlas or the def is
/// missing (or the def has no icon), an invisible placeholder of the same size
/// is rendered so the grid columns stay aligned.
///
/// Free function rather than a method so callers can hold borrows into `self`
/// (e.g. into the catalog) at the same time — a `&self` method would conflict.
pub fn draw_image_button(
    ui: &mut Ui,
    atlas: Option<&ItemAtlas>,
    def: Option<&LootDef>,
    icon_size: f32,
) -> Response {
    let uv = atlas.zip(def).and_then(|(a, d)| a.icon_uv(d));

    if let (Some(uv), Some(atlas)) = (uv, atlas) {
        ui.add(egui::Button::image(
            egui::Image::from_texture(&atlas.texture)
                .fit_to_exact_size(egui::vec2(icon_size, icon_size))
                .uv(uv),
        ))
    } else {
        ui.add_space(icon_size);
        ui.allocate_response(egui::vec2(icon_size, icon_size), egui::Sense::click())
    }
}

/// Render a word-wrapped item name at `font_size` points.
/// Each whitespace-separated word gets its own truncating label so long names
/// don't overflow their icon column.
pub fn add_item_label(ui: &mut Ui, title: &str, font_size: f32) {
    for word in title.split_whitespace() {
        ui.add(
            egui::Label::new(egui::RichText::new(word).size(font_size))
                .wrap_mode(egui::TextWrapMode::Truncate)
                .halign(egui::Align::Center)
                .show_tooltip_when_elided(false),
        );
    }
}

impl SaveEditorApp {
    /// Full item detail panel: name, title, description, type/subtype, cost,
    /// editable count/upgrade, and a collapsible raw-fields section.
    /// Used both in the inventory view and the catalog add-item preview.
    pub fn draw_item_details(&self, ui: &mut Ui, def: &LootDef, item: &mut Item) {
        ui.heading("Item Details");
        ui.separator();

        ui.label(format!("Name: {}", def.name));

        if let Some(title) = def.title.first() {
            if !title.is_empty() {
                ui.label(format!("Title: {}", title));
            }
        }
        if let Some(desc) = def.description.first() {
            if !desc.is_empty() {
                ui.label(format!("Description: {}", desc));
            }
        }

        let type_name = loot_names::get_type_name(def.type_);
        let subtype_name = loot_names::get_subtype_name(def.type_, def.sub_type);
        ui.label(format!("Type: {} - {}", type_name, subtype_name));
        ui.label(format!("Cost: {:.0}", def.cost));

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Count:");
            ui.add(
                egui::DragValue::new(&mut item.count)
                    .speed(self.config.drag_value_sensitivity)
                    .range(0..=999),
            );
            ui.label("Upgrade:");
            ui.add(
                egui::DragValue::new(&mut item.upgrade)
                    .speed(self.config.drag_value_sensitivity)
                    .range(0..=10),
            );
        });

        ui.add_space(8.0);
        ui.separator();

        ui.collapsing(format!("Fields ({})", def.fields.len()), |ui| {
            ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                for field in &def.fields {
                    let name = loot_names::get_field_name(def.type_, field.id);
                    let value = match &field.value {
                        sas2_save::loot_catalog::LootFieldValue::Float(v) => format!("{:.2}", v),
                        sas2_save::loot_catalog::LootFieldValue::Int(v) => v.to_string(),
                        sas2_save::loot_catalog::LootFieldValue::Bool(v) => v.to_string(),
                        sas2_save::loot_catalog::LootFieldValue::String(v) => v.clone(),
                    };
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(name).weak().size(12.0));
                        ui.label(value);
                    });
                }
            });
        });
    }

    pub fn show_equipment_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.equipment_subtab,
                EquipmentSubTab::Inventory,
                "Inventory",
            );
            ui.selectable_value(
                &mut self.equipment_subtab,
                EquipmentSubTab::Stockpile,
                "Stockpile",
            );
            ui.selectable_value(
                &mut self.equipment_subtab,
                EquipmentSubTab::AddItems,
                "Add Items",
            );
        });
        ui.add_space(8.0);

        match self.equipment_subtab {
            EquipmentSubTab::Inventory | EquipmentSubTab::Stockpile => {
                self.show_inventory_or_stockpile(ui, save);
            }
            EquipmentSubTab::AddItems => {
                self.show_add_items_tab(ui, save);
            }
        }
    }

    fn show_inventory_or_stockpile(&mut self, ui: &mut Ui, save: &mut SaveData) {
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.item_search_filter);
        });

        let stockpile_mode = self.equipment_subtab == EquipmentSubTab::Stockpile;

        // Collect matching indices first so we can freely borrow self below.
        let filtered_indices: Vec<usize> = {
            let filter = self.item_search_filter.to_lowercase();
            save.equipment
                .inventory_items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    if item.stock_piled != stockpile_mode {
                        return None;
                    }
                    let matches = if filter.is_empty() {
                        true
                    } else {
                        let id_match = item.loot_idx.to_string().contains(&filter);
                        let name_match = self
                            .catalog
                            .as_ref()
                            .and_then(|c| {
                                c.loot_defs.get(item.loot_idx as usize).map(|d| {
                                    d.name.to_lowercase().contains(&filter)
                                        || d.title
                                            .first()
                                            .map(|t| t.to_lowercase().contains(&filter))
                                            .unwrap_or(false)
                                })
                            })
                            .unwrap_or(false);
                        id_match || name_match
                    };
                    if matches { Some(idx) } else { None }
                })
                .collect()
        };

        // Extract Copy config values up front so the closures below don't need
        // to borrow all of `self` while we also hold field-level borrows.
        let icon_size = self.config.item_icon_size;
        let font_size = self.config.item_font_size;
        let mut selected_local = self.selected_equipment_item;
        let full_width = ui.available_width();
        let min_size = 250.0;
        let panel_width = if self.config.equipment_panel_width > 0.0 {
            self.config.equipment_panel_width.max(min_size)
        } else {
            full_width * 0.5
        };

        let right_panel = egui::Panel::right("item_details")
            .resizable(true)
            .default_size(panel_width)
            .min_size(min_size)
            .max_size(full_width * 0.8)
            .size_range(min_size..=full_width * 0.8)
            .show_inside(ui, |ui| {
                ui.set_min_width(ui.available_width());

                if let Some(orig_idx) = selected_local {
                    let items = &mut save.equipment.inventory_items;
                    if orig_idx < items.len() {
                        let loot_idx = items[orig_idx].loot_idx;

                        // Clone the def so the catalog borrow ends before draw_item_details,
                        // which itself needs &self for drag sensitivity config.
                        let def = self
                            .catalog
                            .as_ref()
                            .and_then(|c| c.loot_defs.get(loot_idx as usize))
                            .cloned();

                        if let Some(def) = def {
                            self.draw_item_details(ui, &def, &mut items[orig_idx]);

                            if ui.button("Remove Item").clicked() {
                                items.remove(orig_idx);
                                selected_local = None;
                            }
                        } else if self.catalog.is_some() {
                            ui.label("Item definition not found.");
                        } else {
                            ui.label("Catalog not loaded.");
                        }
                    }
                } else {
                    ui.label("No item selected.");
                }
            });

        let actual_width = right_panel.response.rect.width();
        if (actual_width - self.config.equipment_panel_width).abs() > 0.1 {
            self.config.equipment_panel_width = actual_width;
            self.config_save_timer = 0.25;
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let mut clicked_item = None;

            ScrollArea::both()
                .max_height(ui.available_height())
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let mut grouped: std::collections::HashMap<String, Vec<usize>> =
                        std::collections::HashMap::new();

                    for &orig_idx in &filtered_indices {
                        let loot_idx = save.equipment.inventory_items[orig_idx].loot_idx;
                        let cat = self
                            .catalog
                            .as_ref()
                            .and_then(|c| c.loot_defs.get(loot_idx as usize))
                            .map(|d| {
                                format!(
                                    "{} - {}",
                                    loot_names::get_type_name(d.type_),
                                    loot_names::get_subtype_name(d.type_, d.sub_type)
                                )
                            })
                            .unwrap_or_else(|| "Other".to_string());

                        grouped.entry(cat).or_default().push(orig_idx);
                    }

                    let mut categories: Vec<_> = grouped.keys().cloned().collect();
                    categories.sort();

                    for cat in categories {
                        let orig_indices = grouped.get(&cat).unwrap();

                        ui.style_mut().interaction.selectable_labels = false;
                        ui.label(egui::RichText::new(&cat).strong());

                        egui::Grid::new(&cat).spacing([8.0, 8.0]).show(ui, |ui| {
                            for &orig_idx in orig_indices {
                                let loot_idx = save.equipment.inventory_items[orig_idx].loot_idx;

                                // Scope the catalog and atlas borrows tightly so they
                                // don't overlap with each other in the borrow checker.
                                let (def_cloned, name) = {
                                    let def = self
                                        .catalog
                                        .as_ref()
                                        .and_then(|c| c.loot_defs.get(loot_idx as usize));
                                    let name = def
                                        .and_then(|d| d.title.first())
                                        .cloned()
                                        .unwrap_or_else(|| format!("Item {}", loot_idx));
                                    (def.cloned(), name)
                                };

                                let atlas = self.item_atlas.as_ref();

                                ui.vertical(|ui| {
                                    let response = draw_image_button(
                                        ui,
                                        atlas,
                                        def_cloned.as_ref(),
                                        icon_size,
                                    );
                                    let btn_w = response.rect.width();

                                    if response.clicked() {
                                        clicked_item = Some(orig_idx);
                                    }

                                    ui.set_max_width(btn_w);
                                    add_item_label(ui, &name, font_size);
                                });
                            }
                        });

                        ui.add_space(8.0);
                    }
                });

            if let Some(idx) = clicked_item {
                selected_local = Some(idx);
            }
        });

        self.selected_equipment_item = selected_local;
    }

    fn show_add_items_tab(&mut self, ui: &mut Ui, save: &mut SaveData) {
        let Some(catalog) = &self.catalog else {
            ui.label("Catalog not loaded.");
            return;
        };

        // Unified search bar (synced with inventory)
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.item_search_filter);
        });

        let filter = self.item_search_filter.to_lowercase();

        // Group items by category
        let mut grouped: std::collections::HashMap<String, Vec<(usize, LootDef)>> =
            std::collections::HashMap::new();

        for (idx, def) in catalog.loot_defs.iter().enumerate() {
            if !filter.is_empty() {
                let matches = def.name.to_lowercase().contains(&filter)
                    || def
                        .title
                        .first()
                        .map(|t| t.to_lowercase().contains(&filter))
                        .unwrap_or(false)
                    || idx.to_string().contains(&filter);
                if !matches {
                    continue;
                }
            }
            let cat = format!(
                "{} - {}",
                loot_names::get_type_name(def.type_),
                loot_names::get_subtype_name(def.type_, def.sub_type)
            );
            grouped.entry(cat).or_default().push((idx, def.clone()));
        }

        let icon_size = self.config.item_icon_size;
        let font_size = self.config.item_font_size;
        let full_width = ui.available_width();
        let min_size = 250.0;
        let panel_width = if self.config.equipment_panel_width > 0.0 {
            self.config.equipment_panel_width.max(min_size)
        } else {
            full_width * 0.4
        };

        // Right panel: item details and add buttons
        let right_panel = egui::Panel::right("add_item_details")
            .resizable(true)
            .default_size(panel_width)
            .min_size(min_size)
            .max_size(full_width * 0.8)
            .size_range(min_size..=full_width * 0.8)
            .show_inside(ui, |ui| {
                ui.set_min_width(ui.available_width());

                if let Some(idx) = self.selected_catalog_item {
                    // Find the def from our grouped map (already cloned)
                    let def = grouped
                        .values()
                        .flatten()
                        .find(|(i, _)| *i == idx)
                        .map(|(_, d)| d.clone());

                    if let Some(def) = def {
                        let mut dummy = Item {
                            loot_idx: idx as i32,
                            count: self.add_item_count,
                            upgrade: self.add_item_upgrade,
                            stock_piled: false,
                            artifact_seed: -1,
                            item_version: 0,
                            rarity: 1,
                        };

                        self.draw_item_details(ui, &def, &mut dummy);

                        // Sync back the edited count/upgrade
                        self.add_item_count = dummy.count;
                        self.add_item_upgrade = dummy.upgrade;

                        ui.horizontal(|ui| {
                            if ui.button("Add to Inventory").clicked() {
                                save.equipment.inventory_items.push(Item {
                                    loot_idx: idx as i32,
                                    count: self.add_item_count,
                                    upgrade: self.add_item_upgrade,
                                    stock_piled: false,
                                    artifact_seed: -1,
                                    item_version: 0,
                                    rarity: 1,
                                });
                            }
                            if ui.button("Add to Stockpile").clicked() {
                                save.equipment.inventory_items.push(Item {
                                    loot_idx: idx as i32,
                                    count: self.add_item_count,
                                    upgrade: self.add_item_upgrade,
                                    stock_piled: true,
                                    artifact_seed: -1,
                                    item_version: 0,
                                    rarity: 1,
                                });
                            }
                        });
                    } else {
                        ui.label("Selected item definition not found.");
                    }
                } else {
                    ui.label("Select an item from the left panel.");
                }
            });

        // Save panel width when resized
        let actual_width = right_panel.response.rect.width();
        if (actual_width - self.config.equipment_panel_width).abs() > 0.1 {
            self.config.equipment_panel_width = actual_width;
            self.config_save_timer = 0.5;
        }

        // Central panel: scrollable grid of items
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let mut clicked_item = None;

            ScrollArea::both()
                .max_height(ui.available_height())
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let mut categories: Vec<_> = grouped.keys().cloned().collect();
                    categories.sort();

                    for cat in categories {
                        let items = grouped.get(&cat).unwrap();

                        ui.style_mut().interaction.selectable_labels = false;
                        ui.label(egui::RichText::new(&cat).strong());

                        egui::Grid::new(&cat).spacing([8.0, 8.0]).show(ui, |ui| {
                            for (idx, def) in items {
                                let atlas = self.item_atlas.as_ref();
                                ui.vertical(|ui| {
                                    let response =
                                        draw_image_button(ui, atlas, Some(def), icon_size);
                                    let btn_w = response.rect.width();

                                    if response.clicked() {
                                        clicked_item = Some(*idx);
                                    }

                                    ui.set_max_width(btn_w);
                                    add_item_label(
                                        ui,
                                        def.title.first().map(|s| s.as_str()).unwrap_or(""),
                                        font_size,
                                    );
                                });
                            }
                        });
                        ui.add_space(8.0);
                    }
                });

            if let Some(idx) = clicked_item {
                self.selected_catalog_item = Some(idx);
            }
        });
    }
}
