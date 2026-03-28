use crate::config::AppConfig;
use crate::catalog::{load_loot_catalog, load_monster_catalog};
use eframe::{egui, Frame};
use rfd::FileDialog;
use sas2_save::loot_catalog::{LootCatalog, LootDef};
use sas2_save::monster_catalog::MonsterCatalog;
use sas2_save::{SaveData, Item, loot_names};
use std::fs;
use std::path::{Path, PathBuf};
use eframe::egui::{Grid, ScrollArea, TextureHandle};
use sas2_save::cosmetics::{AncestryCatalog, BeardCatalog, ClassCatalog, ColorCatalog, CrimeCatalog, EyeCatalog, HairCatalog, SexCatalog};

#[derive(PartialEq)]
pub enum Tab {
    Stats,
    Equipment,
    Flags,
    Bestiary,
    Cosmetics,
}

#[derive(PartialEq)]
pub enum EquipmentSubTab {
    Inventory,
    Stockpile,
    AddItems,
}

pub struct SaveEditorApp {
    pub save_data: Option<SaveData>,
    pub file_path: Option<PathBuf>,
    pub error_message: Option<String>,
    pub active_tab: Tab,

    pub config: AppConfig,
    pub catalog: Option<LootCatalog>,
    pub catalog_error: Option<String>,
    pub monster_catalog: Option<MonsterCatalog>,
    pub monster_catalog_error: Option<String>,
    pub item_atlas: Option<TextureHandle>,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub item_search_filter: String,
    pub equipment_subtab: EquipmentSubTab,
    pub selected_equipment_item: Option<usize>,
    pub selected_catalog_item: Option<usize>,
    pub add_item_count: i32,
    pub add_item_upgrade: i32,
}

impl Default for SaveEditorApp {
    fn default() -> Self {
        let mut app = Self {
            save_data: None,
            file_path: None,
            error_message: None,
            active_tab: Tab::Stats,
            config: AppConfig::load(),
            catalog: None,
            catalog_error: None,
            monster_catalog: None,
            monster_catalog_error: None,
            item_atlas: None,
            atlas_width: 0,
            atlas_height: 0,
            item_search_filter: String::new(),
            equipment_subtab: EquipmentSubTab::Inventory,
            selected_equipment_item: None,
            selected_catalog_item: None,
            add_item_count: 1,
            add_item_upgrade: 0,
        };
        // Try to load catalogs if game path is set
        if let Some(game_path) = &app.config.game_path {
            match load_loot_catalog(game_path) {
                Ok(cat) => app.catalog = Some(cat),
                Err(e) => app.catalog_error = Some(e),
            }
            match load_monster_catalog(game_path) {
                Ok(cat) => app.monster_catalog = Some(cat),
                Err(e) => app.monster_catalog_error = Some(e),
            }
        }
        app
    }
}

impl SaveEditorApp {
    pub fn open_file(&mut self) {
        if let Some(path) = FileDialog::new().pick_file() {
            match fs::read(&path) {
                Ok(data) => match SaveData::from_bytes(&data) {
                    Ok(save) => {
                        self.save_data = Some(save);
                        self.file_path = Some(path);
                        self.error_message = None;
                    }
                    Err(e) => self.error_message = Some(e.to_string()),
                },
                Err(e) => self.error_message = Some(e.to_string()),
            }
        }
    }

    pub fn save_file(&mut self) {
        if let (Some(save), Some(path)) = (self.save_data.as_ref(), &self.file_path) {
            match save.to_bytes() {
                Ok(data) => {
                    if let Err(e) = fs::write(path, data) {
                        self.error_message = Some(e.to_string());
                    } else {
                        self.error_message = None;
                    }
                }
                Err(e) => self.error_message = Some(e.to_string()),
            }
        } else {
            self.error_message = Some("No file loaded".into());
        }
    }

    pub fn set_game_path(&mut self, path: PathBuf) {
        self.config.game_path = Some(path.clone());
        self.config.save();
        // Reload loot catalog
        match load_loot_catalog(&path) {
            Ok(cat) => {
                self.catalog = Some(cat);
                self.catalog_error = None;
            }
            Err(e) => {
                self.catalog = None;
                self.catalog_error = Some(e);
            }
        }
        // Reload monster catalog
        match load_monster_catalog(&path) {
            Ok(cat) => {
                self.monster_catalog = Some(cat);
                self.monster_catalog_error = None;
            }
            Err(e) => {
                self.monster_catalog = None;
                self.monster_catalog_error = Some(e);
            }
        }
        // Atlas will be loaded lazily in update
        self.item_atlas = None;
    }

    pub fn choose_game_folder(&mut self) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            self.set_game_path(folder);
        }
    }

    pub fn load_atlas(&mut self, game_path: &Path, ctx: &egui::Context) {
        let items_xnb = game_path.join("Content").join("gfx").join("items.xnb");
        if !items_xnb.exists() {
            eprintln!("items.xnb not found at {:?}", items_xnb);
            return;
        }
        match sas2_save::xnb_loader::load_texture_from_xnb(items_xnb.to_str().unwrap()) {
            Ok(img) => {
                let width = img.width();
                let height = img.height();
                let pixels = img.into_vec();
                let size = [width as usize, height as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                let texture = ctx.load_texture("items_atlas", color_image, Default::default());
                self.item_atlas = Some(texture);
                self.atlas_width = width;
                self.atlas_height = height;
            }
            Err(e) => eprintln!("Failed to load items.xnb: {}", e),
        }
    }

    pub fn show_stats_ui(&mut self, ui: &mut egui::Ui, save: &mut SaveData) {
        ui.heading("Stats");
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Player Name:");
            ui.text_edit_singleline(&mut save.name);
        });
        ui.horizontal(|ui| {
            ui.label("Level:");
            ui.add(egui::DragValue::new(&mut save.stats.level).speed(1).range(1..=100));
        });
        ui.horizontal(|ui| {
            ui.label("XP:");
            ui.add(egui::DragValue::new(&mut save.stats.xp).speed(100).range(0..=999999));
        });
        ui.horizontal(|ui| {
            ui.label("Silver:");
            ui.add(egui::DragValue::new(&mut save.stats.silver).speed(100).range(0..=999999));
        });
        ui.horizontal(|ui| {
            ui.label("Time Played (seconds):");
            ui.add(egui::DragValue::new(&mut save.stats.time_played).speed(1.0).range(0.0..=1e9));
        });
    }

    pub fn show_equipment_ui(&mut self, ui: &mut egui::Ui, save: &mut SaveData) {
        ui.heading("Equipment");
        ui.separator();

        // Ensure atlas is loaded if we have a game folder
        if self.item_atlas.is_none() {
            if let Some(game_path) = self.config.game_path.clone() {
                self.load_atlas(&game_path, ui.ctx());
            }
        }

        // Sub-tab bar
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.equipment_subtab, EquipmentSubTab::Inventory, "Inventory");
            ui.selectable_value(&mut self.equipment_subtab, EquipmentSubTab::Stockpile, "Stockpile");
            ui.selectable_value(&mut self.equipment_subtab, EquipmentSubTab::AddItems, "Add Items");
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

    fn get_icon_uv(&self, def: &LootDef, atlas: Option<&TextureHandle>, atlas_width: f32, atlas_height: f32) -> Option<egui::Rect> {
        let icon_uv = if def.img >= 0 && atlas.is_some() {
            let x = (def.img as u32 % 32) * 128;
            let y = (def.img as u32 / 32) * 128;

            Some(egui::Rect::from_min_max(
                egui::pos2(
                    x as f32 / atlas_width,
                    y as f32 / atlas_height,
                ),
                egui::pos2(
                    (x + 128) as f32 / atlas_width,
                    (y + 128) as f32 / atlas_height,
                ),
            ))
        } else {
            None
        };
        icon_uv
    }

    fn show_inventory_or_stockpile(&mut self, ui: &mut egui::Ui, save: &mut SaveData) {
        let stockpile_mode = self.equipment_subtab == EquipmentSubTab::Stockpile;
        let items = &mut save.equipment.inventory_items;

        let filtered_indices: Vec<usize> = items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                if item.stock_piled == stockpile_mode {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();

        let catalog = self.catalog.as_ref();
        let atlas = self.item_atlas.as_ref();
        let atlas_width = self.atlas_width;
        let atlas_height = self.atlas_height;

        let mut selected_equipment_item_local = self.selected_equipment_item;

        let full_width = ui.available_width();
        let height = ui.available_height();

        ui.horizontal(|ui| {
            // LEFT
            ui.allocate_ui_with_layout(
                egui::vec2(full_width * 0.6, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    let mut clicked_item = None;

                    ScrollArea::both()
                        .max_height(ui.available_height())
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            let mut grouped: std::collections::HashMap<String, Vec<usize>> =
                                std::collections::HashMap::new();

                            for &orig_idx in &filtered_indices {
                                let item = &items[orig_idx];

                                let cat = if let Some(catalog) = catalog {
                                    if let Some(def) =
                                        catalog.loot_defs.get(item.loot_idx as usize)
                                    {
                                        let type_name = loot_names::get_type_name(def.type_);
                                        let subtype_name =
                                            loot_names::get_subtype_name(def.type_, def.sub_type);
                                        format!("{} - {}", type_name, subtype_name)
                                    } else {
                                        "Other".to_string()
                                    }
                                } else {
                                    "Other".to_string()
                                };

                                grouped.entry(cat).or_default().push(orig_idx);
                            }

                            let mut to_remove: Vec<usize> = Vec::new();

                            let mut categories: Vec<_> = grouped.keys().cloned().collect();
                            categories.sort();

                            for cat in categories {
                                let orig_indices = grouped.get(&cat).unwrap();

                                ui.label(egui::RichText::new(&cat).strong());
                                ui.add_space(4.0);

                                Grid::new(&cat)
                                    .spacing([8.0, 8.0])
                                    .show(ui, |ui| {
                                        for &orig_idx in orig_indices {
                                            let item = &mut items[orig_idx];

                                            let (item_name, icon_uv) = if let Some(catalog) = catalog {
                                                if let Some(def) =
                                                    catalog.loot_defs.get(item.loot_idx as usize)
                                                {
                                                    let name = def.title[0].clone();

                                                    let uv = self.get_icon_uv(def, atlas, atlas_width as f32, atlas_height as f32);

                                                    (name, uv)
                                                } else {
                                                    (format!("Unknown ({})", item.loot_idx), None)
                                                }
                                            } else {
                                                (format!("Item {}", item.loot_idx), None)
                                            };

                                            ui.vertical(|ui| {
                                                // Image as button
                                                let response = if let Some(uv) = icon_uv {
                                                    ui.add(
                                                        egui::Button::image(
                                                            egui::Image::from_texture(atlas.unwrap())
                                                                .fit_to_exact_size(egui::vec2(48.0, 48.0))
                                                                .uv(uv),
                                                        )
                                                    )
                                                } else {
                                                    ui.add_space(48.0);
                                                    ui.allocate_response(egui::vec2(48.0, 48.0), egui::Sense::click())
                                                };

                                                if response.clicked() {
                                                    clicked_item = Some(orig_idx);
                                                }

                                                // Name under the button (non-clickable)
                                                ui.scope(|ui| {
                                                    ui.style_mut().interaction.selectable_labels = false;

                                                    ui.add(
                                                        egui::Label::new(&item_name)
                                                            .sense(egui::Sense::empty())
                                                    );
                                                });

                                                //ui.add(
                                                //    egui::DragValue::new(&mut item.count)
                                                //        .speed(1)
                                                //        .range(0..=999),
                                                //);

                                                //if ui.button("X").clicked() {
                                                //    to_remove.push(orig_idx);
                                                //}
                                            });
                                        }
                                    });

                                ui.add_space(8.0);
                            }

                            if !to_remove.is_empty() {
                                to_remove.sort_unstable_by(|a, b| b.cmp(a));
                                for idx in to_remove {
                                    items.remove(idx);

                                    if let Some(sel) = selected_equipment_item_local {
                                        if sel == idx {
                                            selected_equipment_item_local = None;
                                        } else if sel > idx {
                                            selected_equipment_item_local = Some(sel - 1);
                                        }
                                    }
                                }
                            }
                        });

                    if let Some(idx) = clicked_item {
                        selected_equipment_item_local = Some(idx);
                    }
                },
            );

            // RIGHT
            ui.allocate_ui_with_layout(
                egui::vec2(full_width * 0.4, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    if let Some(orig_idx) = selected_equipment_item_local {
                        if orig_idx < items.len() {
                            let item = &mut items[orig_idx];

                            if let Some(catalog) = catalog {
                                if let Some(def) =
                                    catalog.loot_defs.get(item.loot_idx as usize)
                                {
                                    Self::draw_item_details(ui, def, item);

                                    if ui.button("Remove Item").clicked() {
                                        items.remove(orig_idx);
                                        selected_equipment_item_local = None;
                                    }
                                } else {
                                    ui.label("Item definition not found.");
                                }
                            } else {
                                ui.label("Catalog not loaded.");
                            }
                        }
                    } else {
                        ui.label("No item selected.");
                    }
                },
            );
        });

        self.selected_equipment_item = selected_equipment_item_local;
    }

    fn show_add_items_tab(&mut self, ui: &mut egui::Ui, save: &mut SaveData) {
        let catalog = self.catalog.as_ref();
        let atlas = self.item_atlas.as_ref();
        let atlas_width = self.atlas_width;
        let atlas_height = self.atlas_height;

        let item_search_filter = &mut self.item_search_filter;

        let mut selected_catalog_item_local = self.selected_catalog_item;
        let mut add_item_count_local = self.add_item_count;
        let mut add_item_upgrade_local = self.add_item_upgrade;

        if let Some(catalog) = catalog {
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(item_search_filter);
            });

            let full_width = ui.available_width();
            let height = ui.available_height();

            // Pre-group
            let mut grouped: std::collections::HashMap<
                String,
                Vec<(usize, &LootDef)>,
            > = std::collections::HashMap::new();

            for (idx, def) in catalog.loot_defs.iter().enumerate() {
                if !item_search_filter.is_empty()
                    && !def
                    .name
                    .to_lowercase()
                    .contains(&item_search_filter.to_lowercase())
                {
                    continue;
                }

                let type_name = loot_names::get_type_name(def.type_);
                let subtype_name = loot_names::get_subtype_name(def.type_, def.sub_type);
                let cat = format!("{} - {}", type_name, subtype_name);

                grouped.entry(cat).or_default().push((idx, def));
            }

            ui.horizontal(|ui| {
                // LEFT
                ui.allocate_ui_with_layout(
                    egui::vec2(full_width * 0.6, height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        let mut clicked_item = None;

                        ScrollArea::both().show(ui, |ui| {
                            let mut categories: Vec<_> = grouped.keys().cloned().collect();
                            categories.sort();

                            for cat in categories {
                                let items = grouped.get(&cat).unwrap();

                                ui.label(egui::RichText::new(&cat).strong());

                                Grid::new(&cat)
                                    .spacing([8.0, 8.0])
                                    .show(ui, |ui| {
                                        for (idx, def) in items {
                                            let icon_uv = self.get_icon_uv(def, atlas, atlas_width as f32, atlas_height as f32);

                                            ui.vertical(|ui| {
                                                // Image as button
                                                let response = if let Some(uv) = icon_uv {
                                                    ui.add(
                                                        egui::Button::image(
                                                            egui::Image::from_texture(atlas.unwrap())
                                                                .fit_to_exact_size(egui::vec2(48.0, 48.0))
                                                                .uv(uv),
                                                        )
                                                    )
                                                } else {
                                                    ui.add_space(48.0);
                                                    ui.allocate_response(egui::vec2(48.0, 48.0), egui::Sense::click())
                                                };

                                                if response.clicked() {
                                                    clicked_item = Some(*idx);
                                                }

                                                // Name under the button
                                                ui.scope(|ui| {
                                                    ui.style_mut().interaction.selectable_labels = false;

                                                    ui.add(
                                                        egui::Label::new(&def.title[0])
                                                            .sense(egui::Sense::empty())
                                                    );
                                                });
                                            });
                                        }
                                    });
                            }
                        });

                        if let Some(idx) = clicked_item {
                            selected_catalog_item_local = Some(idx);
                        }
                    },
                );

                // RIGHT
                ui.allocate_ui_with_layout(
                    egui::vec2(full_width * 0.4, height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        if let Some(idx) = selected_catalog_item_local {
                            if let Some(def) = catalog.loot_defs.get(idx) {
                                let mut dummy_item = Item {
                                    loot_idx: idx as i32,
                                    count: add_item_count_local,
                                    upgrade: add_item_upgrade_local,
                                    stock_piled: false,
                                };

                                Self::draw_item_details(ui, def, &mut dummy_item);

                                add_item_count_local = dummy_item.count;
                                add_item_upgrade_local = dummy_item.upgrade;

                                ui.horizontal(|ui| {
                                    if ui.button("Add to Inventory").clicked() {
                                        save.equipment.inventory_items.push(Item {
                                            loot_idx: idx as i32,
                                            count: add_item_count_local,
                                            upgrade: add_item_upgrade_local,
                                            stock_piled: false,
                                        });
                                    }

                                    if ui.button("Add to Stockpile").clicked() {
                                        save.equipment.inventory_items.push(Item {
                                            loot_idx: idx as i32,
                                            count: add_item_count_local,
                                            upgrade: add_item_upgrade_local,
                                            stock_piled: true,
                                        });
                                    }
                                });
                            }
                        } else {
                            ui.label("Select an item.");
                        }
                    },
                );
            });
        } else {
            ui.label("Catalog not loaded.");
        }

        self.selected_catalog_item = selected_catalog_item_local;
        self.add_item_count = add_item_count_local;
        self.add_item_upgrade = add_item_upgrade_local;
    }

    /// Draw detailed information about an item (name, title, description, fields, etc.)
    fn draw_item_details(
        ui: &mut egui::Ui,
        def: &LootDef,
        item: &mut Item,
    ) {
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
            ui.add(egui::DragValue::new(&mut item.count).range(0..=999));

            ui.label("Upgrade:");
            ui.add(egui::DragValue::new(&mut item.upgrade).range(0..=10));
        });

        ui.add_space(8.0);

        ui.separator();

        ui.collapsing(format!("Fields ({})", def.fields.len()), |ui| {
            ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    for field in &def.fields {
                        let field_name = loot_names::get_field_name(def.type_, field.id);

                        let field_value = match &field.value {
                            sas2_save::loot_catalog::LootFieldValue::Float(v) => format!("{:.2}", v),
                            sas2_save::loot_catalog::LootFieldValue::Int(v) => v.to_string(),
                            sas2_save::loot_catalog::LootFieldValue::Bool(v) => v.to_string(),
                            sas2_save::loot_catalog::LootFieldValue::String(v) => v.clone(),
                        };

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(field_name)
                                    .weak()
                                    .size(12.0),
                            );
                            ui.label(field_value);
                        });
                    }
                });
        });
    }

    pub fn show_flags_ui(&mut self, ui: &mut egui::Ui, save: &mut SaveData) {
        ui.heading("Player Flags");
        ui.separator();

        // Editable flags list
        ui.label("Flags:");
        ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
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
                }
            });

        // Add new flag
        ui.horizontal(|ui| {
            if ui.button("Add Flag").clicked() {
                save.flags.flags.push(String::new());
            }
        });

        ui.separator();

        // Editable bounty data
        ui.horizontal(|ui| {
            ui.label("Bounty Seed:");
            ui.add(egui::DragValue::new(&mut save.flags.bounty_seed).speed(1).range(0..=999999));
        });
        ui.horizontal(|ui| {
            ui.label("Bounties Complete (bitmask):");
            ui.add(egui::DragValue::new(&mut save.flags.bounties_complete).speed(1).range(0..=999999));
        });

        // Recalculate NG level from flags after any edit
        save.flags.ng_level = save.flags.flags.iter()
            .filter_map(|f| f.strip_prefix("$&ng_").and_then(|s| s.parse::<i32>().ok()))
            .max()
            .unwrap_or(0);

        ui.label(format!("NG Level (derived from flags): {}", save.flags.ng_level));
    }

    pub fn show_bestiary_ui(&mut self, ui: &mut egui::Ui, save: &mut SaveData) {
        ui.heading("Bestiary");
        ui.separator();

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
                            ui.horizontal(|ui| {
                                ui.label("Kills:");
                                ui.add(egui::DragValue::new(&mut beast.kills).speed(1).range(0..=9999));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Deaths:");
                                ui.add(egui::DragValue::new(&mut beast.deaths).speed(1).range(0..=9999));
                            });
                            ui.label("Drops:");
                            for (drop_idx, drop) in beast.drops.iter_mut().enumerate() {
                                ui.checkbox(drop, format!("Drop {}", drop_idx));
                            }
                        });
                    }
                } else {
                    for (idx, beast) in save.bestiary.beasts.iter_mut().enumerate() {
                        ui.collapsing(format!("Beast {}", idx), |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Kills:");
                                ui.add(egui::DragValue::new(&mut beast.kills).speed(1).range(0..=9999));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Deaths:");
                                ui.add(egui::DragValue::new(&mut beast.deaths).speed(1).range(0..=9999));
                            });
                            ui.label("Drops:");
                            for (drop_idx, drop) in beast.drops.iter_mut().enumerate() {
                                ui.checkbox(drop, format!("Drop {}", drop_idx));
                            }
                        });
                    }
                    if let Some(err) = &self.monster_catalog_error {
                        ui.colored_label(egui::Color32::RED, format!("Monster catalog error: {}", err));
                    }
                }
            });
    }

    pub fn show_cosmetics_ui(&mut self, ui: &mut egui::Ui, save: &mut SaveData) {
        ui.heading("Cosmetics");
        ui.separator();

        type NameFn = fn(usize) -> Option<&'static str>;

        let hair_choices: Vec<(usize, &'static str)> = {
            let indices = HairCatalog::get_ordered_indices();
            indices.into_iter().map(|idx| (idx, HairCatalog::name(idx).unwrap())).collect()
        };

        for slot_idx in 0..save.cosmetics.len() {
            let value = &mut save.cosmetics[slot_idx];
            let (label, name_fn, choices) = match slot_idx {
                0 => ("Sex", SexCatalog::name as NameFn, (0..SexCatalog::len()).collect()),
                1 => ("Ancestry", AncestryCatalog::name as NameFn, (0..AncestryCatalog::len()).collect()),
                2 => ("Eye Color", EyeCatalog::name as NameFn, (0..EyeCatalog::len()).collect()),
                3 => ("Hair", HairCatalog::name as NameFn, hair_choices.iter().map(|&(orig, _)| orig).collect()),
                4 => ("Hair Color", ColorCatalog::name as NameFn, (0..ColorCatalog::len()).collect()),
                5 => ("Beard", BeardCatalog::name as NameFn, (0..BeardCatalog::len()).collect()),
                6 => ("Beard Color", ColorCatalog::name as NameFn, (0..ColorCatalog::len()).collect()),
                7 => ("Eyebrow Color", ColorCatalog::name as NameFn, (0..ColorCatalog::len()).collect()),
                8 => ("Class", ClassCatalog::name as NameFn, (0..ClassCatalog::len()).collect()),
                9 => ("Crime", CrimeCatalog::name as NameFn, (0..CrimeCatalog::len()).collect()),
                10 => ("Unused", (|_| None) as NameFn, Vec::new()),
                _ => continue,
            };

            ui.horizontal(|ui| {
                ui.label(format!("{}:", label));

                if !choices.is_empty() {
                    // Unique ID per slot to avoid clashing
                    ui.push_id(slot_idx, |ui| {
                        let selected_text = name_fn(*value as usize)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("{}", *value));

                        egui::ComboBox::from_label("")
                            .selected_text(selected_text)
                            .show_ui(ui, |ui| {
                                for &choice_idx in &choices {
                                    let choice_text = name_fn(choice_idx)
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| format!("{}", choice_idx));
                                    ui.selectable_value(value, choice_idx as i32, choice_text);
                                }
                            });
                    });
                    // Small label showing the numeric value (instead of a large icon)
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::GRAY, format!("{}", *value));
                } else {
                    // Fallback for unused slot
                    ui.add(egui::DragValue::new(value).speed(1).range(0..=999));
                }
            });
        }
    }
}

impl eframe::App for SaveEditorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Menu bar
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        self.open_file();
                        ui.close();
                    }
                    if ui.button("Save").clicked() {
                        self.save_file();
                        ui.close();
                    }
                });
                ui.menu_button("Settings", |ui| {
                    if ui.button("Set Game Folder").clicked() {
                        self.choose_game_folder();
                        ui.close();
                    }
                });
            });

            // Show game folder status
            if let Some(game_path) = &self.config.game_path {
                ui.label(format!("Game folder: {}", game_path.display()));
                ui.label("(Expected subfolder: Loot/data/loot.zls and Monsters/data/monsters.zms)");
            } else {
                ui.colored_label(egui::Color32::YELLOW, "Game folder not set (needed for item names, icons, and bestiary names)");
                if ui.button("Set Game Folder").clicked() {
                    self.choose_game_folder();
                }
            }
            if let Some(err) = &self.catalog_error {
                ui.colored_label(egui::Color32::RED, format!("Loot catalog error: {}", err));
            }
            if let Some(err) = &self.monster_catalog_error {
                ui.colored_label(egui::Color32::RED, format!("Monster catalog error: {}", err));
            }

            ui.separator();

            // Fix borrow checker: take save_data out of self temporarily
            let mut save_taken = self.save_data.take();

            if let Some(save) = &mut save_taken {
                // Tab bar
                egui::Panel::top("tabs").show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.active_tab, Tab::Stats, "Stats");
                        ui.selectable_value(&mut self.active_tab, Tab::Equipment, "Equipment");
                        ui.selectable_value(&mut self.active_tab, Tab::Flags, "Flags");
                        ui.selectable_value(&mut self.active_tab, Tab::Bestiary, "Bestiary");
                        ui.selectable_value(&mut self.active_tab, Tab::Cosmetics, "Cosmetics");
                    });
                });

                // Tab content
                match self.active_tab {
                    Tab::Stats => self.show_stats_ui(ui, save),
                    Tab::Equipment => self.show_equipment_ui(ui, save),
                    Tab::Flags => self.show_flags_ui(ui, save),
                    Tab::Bestiary => self.show_bestiary_ui(ui, save),
                    Tab::Cosmetics => self.show_cosmetics_ui(ui, save),
                }
            } else {
                ui.label("No save file loaded. Click File -> Open to load a save.");
            }

            self.save_data = save_taken;

            if let Some(err) = &self.error_message {
                ui.colored_label(egui::Color32::RED, err);
            }
        });
    }
}
