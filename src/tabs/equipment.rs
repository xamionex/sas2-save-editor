use crate::app::SaveEditor;
use crate::atlas::ItemAtlas;
use crate::tabs::EquipmentSubTab;
use eframe::egui;
use egui::{Response, ScrollArea, Ui};
use sas2_parser::loot_catalog::LootDef;
use sas2_parser::{Item, SaveData, loot_names};

/// Draw one icon button from the atlas.
/// If either the atlas or the def is missing (or the def has no icon), an invisible placeholder of the same size is rendered so the grid columns stay aligned.
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
/// When `selected`, the name is green.
/// Each whitespace-separated word gets its own truncating label so long names don't overflow their icon column.
pub fn add_item_label(ui: &mut Ui, title: &str, font_size: f32, selected: bool) {
    let color = if selected {
        egui::Color32::LIGHT_GREEN
    } else {
        ui.visuals().text_color()
    };
    for word in title.split_whitespace() {
        ui.add(
            egui::Label::new(egui::RichText::new(word).size(font_size).color(color))
                .wrap_mode(egui::TextWrapMode::Truncate)
                .halign(egui::Align::Center)
                .show_tooltip_when_elided(false),
        );
    }
}

impl SaveEditor {
    /// Full item detail panel: name, title, description, type/subtype, cost, editable count/upgrade, and a collapsible raw-fields section.
    /// Used both in the inventory view and the catalog add-item preview.
    pub fn draw_item_details(&self, ui: &mut Ui, def: &LootDef, item: &mut Item) {
        ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
        ui.style_mut().text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::proportional(self.config.sidebar_font_size),
        );
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

        // Artifacts store their seed in the upgrade field (or artifact_seed in modded saves), so the upgrade drag must not clamp it to 0..=10.
        let is_artifact = def.type_ == 6 && (3..=5).contains(&def.sub_type);
        ui.horizontal(|ui| {
            ui.label("Count:");
            ui.add(
                egui::DragValue::new(&mut item.count)
                    .speed(self.config.drag_value_sensitivity)
                    .range(0..=999),
            );
            if is_artifact {
                ui.label("Seed:");
                let mut seed = crate::artifact::artifact_seed(item);
                if ui
                    .add(
                        egui::DragValue::new(&mut seed)
                            .speed(self.config.drag_value_sensitivity)
                            .range(0..=82000),
                    )
                    .changed()
                {
                    item.artifact_seed = seed;
                    item.upgrade = seed;
                }
                ui.label(format!("Tier: {}", crate::artifact::artifact_tier(seed)));
            } else {
                ui.label("Upgrade:");
                ui.add(
                    egui::DragValue::new(&mut item.upgrade)
                        .speed(self.config.drag_value_sensitivity)
                        .range(0..=10),
                );
            }
        });

        ui.add_space(8.0);
        ui.separator();

        ui.collapsing(format!("Fields ({})", def.fields.len()), |ui| {
            ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                for field in &def.fields {
                    let name = loot_names::get_field_name(def.type_, field.id);
                    let value = match &field.value {
                        sas2_parser::loot_catalog::LootFieldValue::Float(v) => format!("{:.2}", v),
                        sas2_parser::loot_catalog::LootFieldValue::Int(v) => v.to_string(),
                        sas2_parser::loot_catalog::LootFieldValue::Bool(v) => v.to_string(),
                        sas2_parser::loot_catalog::LootFieldValue::String(v) => v.clone(),
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
        let prev_subtab = self.equipment_subtab.clone();
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

            // Multi-select toolbar, next to the subtab selectors.
            let multi_count = self.selected_equipment_items.len();
            if multi_count > 1 || !self.equipment_remove_all_open {
                ui.separator();
                if multi_count > 1
                    && ui
                        .button(format!("Remove selected ({})", multi_count))
                        .on_hover_text("Remove the right-click selected items from the inventory")
                        .clicked()
                {
                    let mut to_remove: Vec<usize> =
                        self.selected_equipment_items.iter().copied().collect();
                    to_remove.sort_unstable();
                    to_remove.reverse();
                    for idx in to_remove {
                        if idx < save.equipment.inventory_items.len() {
                            save.equipment.inventory_items.remove(idx);
                        }
                    }
                    self.selected_equipment_items.clear();
                    self.selected_equipment_item = None;
                }
                if ui
                    .button("Remove all by type...")
                    .on_hover_text(
                        "Open a picker to remove every item of the chosen type-subtype categories",
                    )
                    .clicked()
                {
                    self.equipment_remove_all_open = true;
                    self.equipment_remove_all_types.clear();
                }
                ui.separator();
                crate::tabs::multisel::mouse_help_button(ui, &[]);
            }
        });
        ui.add_space(8.0);

        // Switching between Inventory and Stockpile changes the item set, so the range-select anchor and any in-flight gesture must reset.
        if prev_subtab != self.equipment_subtab {
            self.equipment_grid_sel.reset_gesture();
        }

        match self.equipment_subtab {
            EquipmentSubTab::Inventory | EquipmentSubTab::Stockpile => {
                self.show_inventory_or_stockpile(ui, save);
            }
            EquipmentSubTab::AddItems => {
                self.show_add_items_tab(ui, save);
            }
        }

        // "Remove all by type" picker window, reachable from any subtab via the header button.
        if self.equipment_remove_all_open {
            let mut open = self.equipment_remove_all_open;
            let mut do_remove = false;
            egui::Window::new("Remove all by type")
                .collapsible(false)
                .resizable(true)
                .default_width(360.0)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    // Collect the categories present in the inventory (both stockpiled and not).
                    let mut cats: Vec<String> = Vec::new();
                    for item in &save.equipment.inventory_items {
                        let cat = self
                            .catalog
                            .as_ref()
                            .and_then(|c| c.loot_defs.get(item.loot_idx as usize))
                            .map(|d| {
                                format!(
                                    "{} - {}",
                                    loot_names::get_type_name(d.type_),
                                    loot_names::get_subtype_name(d.type_, d.sub_type)
                                )
                            })
                            .unwrap_or_else(|| "Other".to_string());
                        if !cats.contains(&cat) {
                            cats.push(cat);
                        }
                    }
                    cats.sort();
                    for cat in &cats {
                        let mut checked = self.equipment_remove_all_types.contains(cat);
                        if ui.checkbox(&mut checked, cat).changed() {
                            if checked {
                                self.equipment_remove_all_types.insert(cat.clone());
                            } else {
                                self.equipment_remove_all_types.remove(cat);
                            }
                        }
                    }
                    ui.separator();
                    if ui
                        .add_enabled(
                            !self.equipment_remove_all_types.is_empty(),
                            egui::Button::new("Remove"),
                        )
                        .clicked()
                    {
                        do_remove = true;
                    }
                });
            self.equipment_remove_all_open = open;
            if do_remove {
                let mut to_remove: Vec<usize> = Vec::new();
                for (idx, item) in save.equipment.inventory_items.iter().enumerate() {
                    let cat = self
                        .catalog
                        .as_ref()
                        .and_then(|c| c.loot_defs.get(item.loot_idx as usize))
                        .map(|d| {
                            format!(
                                "{} - {}",
                                loot_names::get_type_name(d.type_),
                                loot_names::get_subtype_name(d.type_, d.sub_type)
                            )
                        })
                        .unwrap_or_else(|| "Other".to_string());
                    if self.equipment_remove_all_types.contains(&cat) {
                        to_remove.push(idx);
                    }
                }
                to_remove.sort_unstable();
                to_remove.reverse();
                for idx in to_remove {
                    save.equipment.inventory_items.remove(idx);
                }
                self.selected_equipment_items.clear();
                self.selected_equipment_item = None;
                self.equipment_remove_all_open = false;
            }
        }
    }

    fn show_inventory_or_stockpile(&mut self, ui: &mut Ui, save: &mut SaveData) {
        // Synced search bar
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

        // Extract Copy config values up front so the closures below don't need to borrow all of `self` while we also hold field-level borrows.
        let icon_size = self.config.item_icon_size;
        let font_size = self.config.grid_font_size;
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

                // Multi-selection: edit the common fields of every selected item.
                let multi: Vec<usize> = self.selected_equipment_items.iter().copied().collect();
                if multi.len() > 1 {
                    ui.heading("Edit Selected Items");
                    ui.label(format!("{} items selected", multi.len()));
                    ui.add_space(4.0);

                    // Count / Upgrade apply to every selected item.
                    // Artifacts store their seed in the upgrade field, so the drag must not clamp it to 0..=10.
                    let any_artifact = multi.iter().any(|&idx| {
                        save.equipment
                            .inventory_items
                            .get(idx)
                            .and_then(|i| {
                                self.catalog
                                    .as_ref()
                                    .and_then(|c| c.loot_defs.get(i.loot_idx as usize))
                            })
                            .is_some_and(|d| d.type_ == 6 && (3..=5).contains(&d.sub_type))
                    });
                    let mut count_changed = false;
                    let mut upgrade_changed = false;
                    {
                        let items = &mut save.equipment.inventory_items;
                        let first_count = items.get(multi[0]).map(|i| i.count).unwrap_or(0);
                        let first_upgrade = items.get(multi[0]).map(|i| i.upgrade).unwrap_or(0);
                        let mut count = first_count;
                        let mut upgrade = first_upgrade;
                        ui.horizontal(|ui| {
                            ui.label("Count:");
                            count_changed = ui
                                .add(
                                    egui::DragValue::new(&mut count)
                                        .speed(self.config.drag_value_sensitivity)
                                        .range(0..=999),
                                )
                                .changed();
                            if any_artifact {
                                ui.label("Seed:");
                                upgrade_changed = ui
                                    .add(
                                        egui::DragValue::new(&mut upgrade)
                                            .speed(self.config.drag_value_sensitivity)
                                            .range(0..=82000),
                                    )
                                    .changed();
                            } else {
                                ui.label("Upgrade:");
                                upgrade_changed = ui
                                    .add(
                                        egui::DragValue::new(&mut upgrade)
                                            .speed(self.config.drag_value_sensitivity)
                                            .range(0..=10),
                                    )
                                    .changed();
                            }
                        });
                        if count_changed {
                            for &idx in &multi {
                                if let Some(item) = items.get_mut(idx) {
                                    item.count = count;
                                }
                            }
                        }
                        if upgrade_changed {
                            for &idx in &multi {
                                if let Some(item) = items.get_mut(idx) {
                                    item.upgrade = upgrade;
                                    if any_artifact {
                                        item.artifact_seed = upgrade;
                                    }
                                }
                            }
                        }
                    }
                    ui.add_space(4.0);
                    if ui.button("Remove all selected").clicked() {
                        let mut to_remove = multi.clone();
                        to_remove.sort_unstable();
                        to_remove.reverse();
                        for idx in to_remove {
                            if idx < save.equipment.inventory_items.len() {
                                save.equipment.inventory_items.remove(idx);
                            }
                        }
                        self.selected_equipment_items.clear();
                        selected_local = None;
                    }
                    ui.separator();
                }

                if multi.len() <= 1 {
                    if let Some(orig_idx) = selected_local {
                        let items = &mut save.equipment.inventory_items;
                        if orig_idx < items.len() {
                            let loot_idx = items[orig_idx].loot_idx;

                            // Clone the def so the catalog borrow ends before draw_item_details, itself needs &self for drag sensitivity config.
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
                }
            });

        let actual_width = right_panel.response.rect.width();
        if (actual_width - self.config.equipment_panel_width).abs() > 0.1 {
            self.config.equipment_panel_width = actual_width;
            self.config_save_timer = 0.25;
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Selection gesture state (click / ctrl+click / shift+click / shift+drag box).
            // Taken out of self so the scroll closure can mutate it while borrowing self immutably for the catalog/atlas.
            let mut gsel = std::mem::take(&mut self.equipment_grid_sel);
            gsel.begin(ui);

            // Shift disables drag-panning so the selection box can be drawn; without
            // shift, holding left click pans the view.
            ScrollArea::both()
                .scroll_source(crate::tabs::multisel::grid_scroll_source(ui))
                .max_height(ui.available_height())
                .auto_shrink([false; 2])
                .show_viewport(ui, |ui, viewport| {
                    let mut grouped: std::collections::HashMap<String, Vec<usize>> =
                        std::collections::HashMap::new();

                    for &orig_idx in &filtered_indices {
                        // The right panel may have removed an item this frame, so the index can be stale.
                        if orig_idx >= save.equipment.inventory_items.len() {
                            continue;
                        }
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

                    // Full display order (all filtered items, not just visible ones) so shift+right-click ranges work across scrolled-out items.
                    gsel.display_order.clear();
                    for cat in &categories {
                        for &orig_idx in &grouped[cat] {
                            gsel.display_order.push(orig_idx);
                        }
                    }

                    // Only items whose x-range intersects the visible viewport are laid out each frame.
                    // Culled items still advance the grid cursor via allocate_space with the exact cell size, so positions, row heights and the scrollbar stay exact while the widget count stays proportional to the viewport.
                    let label_h =
                        ui.fonts_mut(|f| f.row_height(&egui::FontId::proportional(font_size)));
                    let spacing_y = ui.spacing().item_spacing.y;
                    let pad_x = 2.0 * ui.spacing().button_padding.x;
                    let pad_y = 2.0 * ui.spacing().button_padding.y;
                    let overscan = 3.0 * (icon_size + pad_x);
                    let vp_min = viewport.min.x - overscan;
                    let vp_max = viewport.max.x + overscan;

                    for cat in categories {
                        let orig_indices = grouped.get(&cat).unwrap();

                        ui.style_mut().interaction.selectable_labels = false;
                        ui.label(
                            egui::RichText::new(&cat)
                                .strong()
                                .size(self.config.category_font_size),
                        );

                        egui::Grid::new(&cat).spacing([8.0, 8.0]).show(ui, |ui| {
                            let mut x = 0.0f32;
                            for &orig_idx in orig_indices {
                                let loot_idx = save.equipment.inventory_items[orig_idx].loot_idx;

                                // Scope the catalog and atlas borrows tightly so they don't overlap with each other in the borrow checker.
                                let (def_cloned, name, has_icon) = {
                                    let def = self
                                        .catalog
                                        .as_ref()
                                        .and_then(|c| c.loot_defs.get(loot_idx as usize));
                                    let name = def
                                        .and_then(|d| d.title.first())
                                        .cloned()
                                        .unwrap_or_else(|| format!("Item {}", loot_idx));
                                    let has_icon = self
                                        .item_atlas
                                        .as_ref()
                                        .zip(def)
                                        .and_then(|(a, d)| a.icon_uv(d))
                                        .is_some();
                                    (def.cloned(), name, has_icon)
                                };

                                let atlas = self.item_atlas.as_ref();

                                // Image buttons are icon_size + button frame margins wide; placeholders are icon_size.
                                let word_count = name.split_whitespace().count();
                                let item_w = if has_icon {
                                    icon_size + pad_x
                                } else {
                                    icon_size
                                };
                                let item_h = if has_icon {
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
                                    let response = draw_image_button(
                                        ui,
                                        atlas,
                                        def_cloned.as_ref(),
                                        icon_size,
                                    );
                                    let btn_w = response.rect.width();
                                    // The whole cell (icon + label) is the box target.
                                    gsel.cell(response.rect, orig_idx);

                                    let is_sel = selected_local == Some(orig_idx)
                                        || self.selected_equipment_items.contains(&orig_idx)
                                        || gsel.is_box_hit(&orig_idx);
                                    crate::tabs::multisel::paint_sel_outline(
                                        ui,
                                        response.rect,
                                        is_sel,
                                    );
                                    // Artifact seed badge (bottom-left of the icon), only for artifacts (type 6, subtypes 3-5).
                                    let is_artifact = def_cloned.as_ref().is_some_and(|d| {
                                        d.type_ == 6 && (3..=5).contains(&d.sub_type)
                                    });
                                    if is_artifact {
                                        let seed = save
                                            .equipment
                                            .inventory_items
                                            .get(orig_idx)
                                            .map(crate::artifact::artifact_seed)
                                            .unwrap_or(0);
                                        crate::tabs::multisel::paint_seed_badge(
                                            ui,
                                            response.rect,
                                            self.config.artifact_seed_style,
                                            seed,
                                        );
                                    }
                                    // Upgrade level badge (bottom-right of the icon).
                                    // Artifacts have no 0-10 upgrade: their upgrade field holds the seed, so show the tier (seed / 2000) instead.
                                    let badge_level = if is_artifact {
                                        let seed = save
                                            .equipment
                                            .inventory_items
                                            .get(orig_idx)
                                            .map(crate::artifact::artifact_seed)
                                            .unwrap_or(0);
                                        crate::artifact::artifact_tier(seed)
                                    } else {
                                        save.equipment
                                            .inventory_items
                                            .get(orig_idx)
                                            .map(|i| i.upgrade)
                                            .unwrap_or(0)
                                    };
                                    crate::tabs::multisel::paint_upgrade_badge(
                                        ui,
                                        response.rect,
                                        self.config.upgrade_style,
                                        badge_level,
                                    );
                                    ui.set_max_width(btn_w);
                                    add_item_label(ui, &name, font_size, is_sel);
                                });
                            }
                        });

                        ui.add_space(8.0);
                    }

                    gsel.update_target();
                    gsel.paint(ui);
                });

            gsel.end(ui, &mut self.selected_equipment_items, &mut selected_local);
            self.equipment_grid_sel = gsel;
        });

        self.selected_equipment_item = selected_local;
    }

    fn show_add_items_tab(&mut self, ui: &mut Ui, save: &mut SaveData) {
        let Some(catalog) = &self.catalog else {
            ui.label("Catalog not loaded.");
            return;
        };

        // Synced search bar
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.item_search_filter);
            crate::tabs::multisel::mouse_help_button(ui, &[]);
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
        let font_size = self.config.grid_font_size;
        let full_width = ui.available_width();
        let min_size = 250.0;
        let panel_width = if self.config.add_items_panel_width > 0.0 {
            self.config.add_items_panel_width.max(min_size)
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

                // Multi-selection: add all selected items.
                let multi = self.selected_catalog_items.clone();
                if multi.len() > 1 {
                    ui.heading("Add Selected Items");
                    ui.label(format!("{} items selected", multi.len()));
                    ui.add_space(4.0);
                    // Common count/upgrade applied to every selected item.
                    ui.horizontal(|ui| {
                        ui.label("Count:");
                        ui.add(
                            egui::DragValue::new(&mut self.add_item_count)
                                .speed(self.config.drag_value_sensitivity)
                                .range(0..=999),
                        );
                        ui.label("Upgrade:");
                        ui.add(
                            egui::DragValue::new(&mut self.add_item_upgrade)
                                .speed(self.config.drag_value_sensitivity)
                                .range(0..=10),
                        );
                    });
                    ui.add_space(4.0);
                    let count = self.add_item_count;
                    let upgrade = self.add_item_upgrade;
                    ui.horizontal(|ui| {
                        if ui.button("Add all to Inventory").clicked() {
                            for &idx in &multi {
                                save.equipment.inventory_items.push(Item {
                                    loot_idx: idx as i32,
                                    count,
                                    upgrade,
                                    stock_piled: false,
                                    artifact_seed: -1,
                                    item_version: 0,
                                    rarity: 1,
                                });
                            }
                        }
                        if ui.button("Add all to Stockpile").clicked() {
                            for &idx in &multi {
                                save.equipment.inventory_items.push(Item {
                                    loot_idx: idx as i32,
                                    count,
                                    upgrade,
                                    stock_piled: true,
                                    artifact_seed: -1,
                                    item_version: 0,
                                    rarity: 1,
                                });
                            }
                        }
                    });
                    ui.separator();
                }

                if multi.len() <= 1 {
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
                    } else if multi.is_empty() {
                        ui.label("Select an item from the left panel.");
                    }
                }
            });

        // Save panel width when resized
        let actual_width = right_panel.response.rect.width();
        if (actual_width - self.config.add_items_panel_width).abs() > 0.1 {
            self.config.add_items_panel_width = actual_width;
            self.config_save_timer = 0.5;
        }

        // Central panel: scrollable grid of items
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Selection gesture state (click / ctrl+click / shift+click / shift+drag box).
            let mut gsel = std::mem::take(&mut self.add_items_grid_sel);
            gsel.begin(ui);

            // Full display order (all filtered items, not just visible ones).
            gsel.display_order.clear();
            let mut categories: Vec<_> = grouped.keys().cloned().collect();
            categories.sort();
            for cat in &categories {
                for (idx, _) in &grouped[cat] {
                    gsel.display_order.push(*idx);
                }
            }

            ScrollArea::both()
                .scroll_source(crate::tabs::multisel::grid_scroll_source(ui))
                .max_height(ui.available_height())
                .auto_shrink([false; 2])
                .show_viewport(ui, |ui, viewport| {
                    // Only items whose x-range intersects the visible viewport are laid out each frame.
                    // Culled items still advance the grid cursor via allocate_space with the exact cell size, so positions, row heights and the scrollbar stay exact while the widget count stays proportional to the viewport.
                    let label_h =
                        ui.fonts_mut(|f| f.row_height(&egui::FontId::proportional(font_size)));
                    let spacing_y = ui.spacing().item_spacing.y;
                    let pad_x = 2.0 * ui.spacing().button_padding.x;
                    let pad_y = 2.0 * ui.spacing().button_padding.y;
                    let overscan = 3.0 * (icon_size + pad_x);
                    let vp_min = viewport.min.x - overscan;
                    let vp_max = viewport.max.x + overscan;

                    for cat in categories {
                        let items = grouped.get(&cat).unwrap();

                        ui.style_mut().interaction.selectable_labels = false;
                        ui.label(
                            egui::RichText::new(&cat)
                                .strong()
                                .size(self.config.category_font_size),
                        );

                        egui::Grid::new(&cat).spacing([8.0, 8.0]).show(ui, |ui| {
                            let mut x = 0.0f32;
                            for (idx, def) in items {
                                let atlas = self.item_atlas.as_ref();
                                let has_icon = atlas.and_then(|a| a.icon_uv(def)).is_some();
                                let name = def.title.first().map(|s| s.as_str()).unwrap_or("");
                                let word_count = name.split_whitespace().count();
                                let item_w = if has_icon {
                                    icon_size + pad_x
                                } else {
                                    icon_size
                                };
                                let item_h = if has_icon {
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
                                    let response =
                                        draw_image_button(ui, atlas, Some(def), icon_size);
                                    let btn_w = response.rect.width();

                                    gsel.cell(response.rect, *idx);

                                    let is_sel = self.selected_catalog_item == Some(*idx)
                                        || self.selected_catalog_items.contains(idx)
                                        || gsel.is_box_hit(idx);
                                    crate::tabs::multisel::paint_sel_outline(
                                        ui,
                                        response.rect,
                                        is_sel,
                                    );
                                    ui.set_max_width(btn_w);
                                    add_item_label(ui, name, font_size, is_sel);
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
                &mut self.selected_catalog_items,
                &mut self.selected_catalog_item,
            );
            self.add_items_grid_sel = gsel;
        });
    }
}
