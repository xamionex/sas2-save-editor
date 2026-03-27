use crate::config::AppConfig;
use crate::catalog::{load_loot_catalog, load_monster_catalog};
use eframe::{egui, Frame};
use rfd::FileDialog;
use sas2_save::loot_catalog::LootCatalog;
use sas2_save::monster_catalog::MonsterCatalog;
use sas2_save::{SaveData, Item};
use std::fs;
use std::path::{Path, PathBuf};
use eframe::egui::ScrollArea;
use sas2_save::cosmetics::{AncestryCatalog, BeardCatalog, ClassCatalog, ColorCatalog, CrimeCatalog, EyeCatalog, HairCatalog, SexCatalog};

#[derive(PartialEq)]
pub enum Tab {
    Stats,
    Equipment,
    Flags,
    Bestiary,
    Cosmetics,
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
    pub item_atlas: Option<egui::TextureHandle>,
    pub atlas_width: u32,
    pub atlas_height: u32,

    // For inline item catalog
    pub item_search_filter: String,
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

    pub fn get_item_category(&self, loot_idx: i32) -> Option<String> {
        if let Some(catalog) = &self.catalog {
            if let Some(def) = catalog.loot_defs.get(loot_idx as usize) {
                return Some(match def.type_ {
                    1 => "Weapons".to_string(),
                    2 => "Ranged".to_string(),
                    0 => match def.sub_type {
                        0 => "Helms",
                        1 => "Chests",
                        2 => "Gloves",
                        3 => "Boots",
                        _ => "Armor",
                    }
                        .to_string(),
                    3 => "Consumables".to_string(),
                    4 => "Materials".to_string(),
                    5 => "Keys".to_string(),
                    6 => "Charms".to_string(),
                    7 => "Magic".to_string(),
                    8 => "Gestures".to_string(),
                    _ => "Other".to_string(),
                });
            }
        }
        None
    }

    pub fn add_icon(&self, ui: &mut egui::Ui, icon_uv: Option<egui::Rect>) {
        if let (Some(atlas), Some(uv)) = (&self.item_atlas, icon_uv) {
            ui.add(
                egui::Image::from_texture(atlas)
                    .fit_to_exact_size(egui::vec2(48.0, 48.0))
                    .uv(uv),
            );
        } else {
            ui.add_space(48.0);
        }
    }

    /// Draws the icon and truncated name of an item, then calls the closure to add controls.
    pub fn draw_item_cell<F>(&self, ui: &mut egui::Ui, name: &str, icon_uv: Option<egui::Rect>, controls: F)
    where
        F: FnOnce(&mut egui::Ui),
    {
        ui.vertical(|ui| {
            self.add_icon(ui, icon_uv);
            let short_name = if name.len() > 15 { &name[..15] } else { name };
            ui.label(short_name);
            controls(ui);
        });
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

        // Existing inventory section
        ui.collapsing("Inventory", |ui| {
            // Group inventory items by category, storing index and mutable reference
            let mut grouped: std::collections::HashMap<String, Vec<(usize, &mut Item)>> =
                std::collections::HashMap::new();
            for (idx, item) in save.equipment.inventory_items.iter_mut().enumerate() {
                let cat = self
                    .get_item_category(item.loot_idx)
                    .unwrap_or_else(|| "Other".to_string());
                grouped.entry(cat).or_default().push((idx, item));
            }

            let mut to_remove = Vec::new();

            ScrollArea::both()
                .max_height(ui.available_height() - 32.0)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let mut categories: Vec<_> = grouped.keys().cloned().collect();
                    categories.sort();
                    for cat in categories {
                        let items = grouped.get_mut(&cat).unwrap();
                        ui.label(egui::RichText::new(&cat).strong());
                        ui.add_space(4.0);
                        egui::Grid::new(&cat)
                            .num_columns(6)
                            .spacing([8.0, 8.0])
                            .show(ui, |ui| {
                                for (idx_ref, item_ref) in items.iter_mut() {
                                    let idx = *idx_ref;
                                    let item = &mut **item_ref;

                                    let (item_name, icon_uv) = if let Some(catalog) = &self.catalog {
                                        let def = catalog.loot_defs.get(item.loot_idx as usize);
                                        let name = def
                                            .map(|d| d.name.clone())
                                            .unwrap_or_else(|| format!("Unknown (ID: {})", item.loot_idx));
                                        let uv = def.and_then(|d| {
                                            let img = d.img;
                                            if img >= 0 && self.item_atlas.is_some() {
                                                let x = (img as u32 % 32) * 128;
                                                let y = (img as u32 / 32) * 128;
                                                Some(egui::Rect::from_min_max(
                                                    egui::pos2(
                                                        x as f32 / self.atlas_width as f32,
                                                        y as f32 / self.atlas_height as f32,
                                                    ),
                                                    egui::pos2(
                                                        (x + 128) as f32 / self.atlas_width as f32,
                                                        (y + 128) as f32 / self.atlas_height as f32,
                                                    ),
                                                ))
                                            } else {
                                                None
                                            }
                                        });
                                        (name, uv)
                                    } else {
                                        (format!("Item ID: {}", item.loot_idx), None)
                                    };

                                    self.draw_item_cell(ui, &item_name, icon_uv, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.add(egui::DragValue::new(&mut item.count).speed(1).range(0..=999));
                                            if ui.button("X").clicked() {
                                                to_remove.push(idx);
                                            }
                                        });
                                    });
                                }
                            });
                        ui.add_space(8.0);
                    }
                });

            drop(grouped);
            // Remove items in reverse order to keep indices valid
            for idx in to_remove.into_iter().rev() {
                save.equipment.inventory_items.remove(idx);
            }
        });

        // Add item from catalog
        ui.collapsing("Add Item", |ui| {
            if let Some(catalog) = &self.catalog {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.item_search_filter);
                });
                ui.add_space(4.0);

                struct CatalogItemInfo {
                    loot_idx: i32,
                    name: String,
                    img: i32,
                }
                let mut grouped: std::collections::HashMap<String, Vec<CatalogItemInfo>> =
                    std::collections::HashMap::new();

                for (idx, def) in catalog.loot_defs.iter().enumerate() {
                    let name_lower = def.name.to_lowercase();
                    let filter_lower = self.item_search_filter.to_lowercase();
                    if !self.item_search_filter.is_empty() && !name_lower.contains(&filter_lower) {
                        continue;
                    }
                    let cat = self
                        .get_item_category(idx as i32)
                        .unwrap_or_else(|| "Other".to_string());
                    grouped.entry(cat).or_default().push(CatalogItemInfo {
                        loot_idx: idx as i32,
                        name: def.name.clone(),
                        img: def.img,
                    });
                }

                ScrollArea::both()
                    .max_height(ui.available_height() - 32.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let mut categories: Vec<_> = grouped.keys().cloned().collect();
                        categories.sort();
                        for cat in categories {
                            let items = grouped.get(&cat).unwrap();
                            ui.label(egui::RichText::new(&cat).strong());
                            ui.add_space(4.0);
                            egui::Grid::new(&format!("catalog_{}", cat))
                                .num_columns(6)
                                .spacing([8.0, 8.0])
                                .show(ui, |ui| {
                                    for info in items {
                                        let icon_uv = if info.img >= 0 && self.item_atlas.is_some() {
                                            let x = (info.img as u32 % 32) * 128;
                                            let y = (info.img as u32 / 32) * 128;
                                            Some(egui::Rect::from_min_max(
                                                egui::pos2(
                                                    x as f32 / self.atlas_width as f32,
                                                    y as f32 / self.atlas_height as f32,
                                                ),
                                                egui::pos2(
                                                    (x + 128) as f32 / self.atlas_width as f32,
                                                    (y + 128) as f32 / self.atlas_height as f32,
                                                ),
                                            ))
                                        } else {
                                            None
                                        };

                                        self.draw_item_cell(ui, &info.name, icon_uv, |ui| {
                                            if ui.button("+").clicked() {
                                                save.equipment.inventory_items.push(Item {
                                                    loot_idx: info.loot_idx,
                                                    count: 1,
                                                    upgrade: 0,
                                                    stock_piled: false,
                                                });
                                            }
                                        });
                                    }
                                });
                            ui.add_space(8.0);
                        }
                    });
            } else {
                ui.label("Catalog not loaded. Please set the game folder.");
            }
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