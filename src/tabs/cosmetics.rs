use crate::app::SaveEditor;
use crate::player_preview::{CosmeticSelection, PreviewView};
use eframe::egui;
use egui::Ui;
use sas2_parser::SaveData;
use sas2_parser::cosmetics::{
    AncestryCatalog, BeardCatalog, ColorCatalog, EyeCatalog, HairCatalog, SexCatalog,
};


/// Starting loadout, attributes and skill unlocks per class, in cosmetic[8] order.
/// Names are the game's own (LocStrings 551, 552, 554, 553, 555, 557, 558, 559) and the gear list is ClassCatalog's loot order.
/// Note: the parser's ClassCatalog has Duelist and Fighter swapped at indices 2/3, so the picker uses this table.
struct ClassInfo {
    name: &'static str,
    gear: &'static [&'static str],
    /// str, dex, vit, wil, end, arc, conv, res, luck
    stats: [i32; 9],
    unlocks: [i32; 3],
}

const CLASSES: [ClassInfo; 8] = [
    ClassInfo {
        name: "Assassin",
        gear: &["assassin_helm", "assassin_armor", "assassin_gloves", "assassin_boots", "daggers_hunter", "throwing_dagger", "arrow", "dagger_base"],
        stats: [5, 14, 7, 11, 9, 5, 5, 5, 9],
        unlocks: [0, 18, 31],
    },
    ClassInfo {
        name: "Cleric",
        gear: &["cleric_helm", "cleric_armor", "cleric_gloves", "cleric_boots", "mace_simple", "crossbow_simple", "arrow", "dagger_base", "focus_potion"],
        stats: [7, 6, 8, 6, 7, 5, 13, 10, 8],
        unlocks: [17, 308, 48],
    },
    ClassInfo {
        name: "Fighter",
        gear: &["heavy_helm", "heavy_armor", "heavy_gloves", "heavy_boots", "vanguard_battleaxe", "throwing_axe", "arrow", "dagger_base"],
        stats: [12, 6, 12, 6, 12, 5, 5, 5, 7],
        unlocks: [12, 2, 215],
    },
    ClassInfo {
        name: "Duelist",
        gear: &["fencer_helm", "fencer_armor", "fencer_gloves", "fencer_boots", "rapier_steel", "crossbow_wood", "arrow", "dagger_base"],
        stats: [6, 13, 9, 11, 6, 5, 5, 5, 10],
        unlocks: [0, 21, 33],
    },
    ClassInfo {
        name: "Highblade",
        gear: &["swordsman_helm", "swordsman_armor", "swordsman_gloves", "swordsman_boots", "katana_steel", "bow_simple", "arrow", "dagger_base"],
        stats: [6, 12, 12, 10, 8, 5, 5, 5, 7],
        unlocks: [47, 20, 10],
    },
    ClassInfo {
        name: "Paladin",
        gear: &["paladin_helm", "paladin_armor", "paladin_gloves", "paladin_boots", "vanguard_paladin", "throwing_axe", "arrow", "dagger_base"],
        stats: [10, 5, 9, 6, 10, 5, 10, 10, 5],
        unlocks: [10, 215, 308],
    },
    ClassInfo {
        name: "Ranger",
        gear: &["thief_helm", "thief_armor", "thief_gloves", "thief_boots", "spear_wood", "bow_wood", "arrow", "dagger_base"],
        stats: [9, 12, 9, 10, 9, 5, 5, 5, 6],
        unlocks: [24, 23, 28],
    },
    ClassInfo {
        name: "Sage",
        gear: &["sage_helm", "sage_armor", "sage_gloves", "sage_boots", "stave_iron", "channeler_sage", "arrow", "dagger_base", "focus_potion"],
        stats: [6, 8, 7, 7, 8, 13, 5, 10, 6],
        unlocks: [321, 26, 30],
    },
];

/// The starting item each crime grants, in cosmetic[9] order (CrimeCatalog).
struct CrimeInfo {
    name: &'static str,
    loot: &'static str,
    count: i32,
}

const CRIMES: [CrimeInfo; 12] = [
    CrimeInfo { name: "Alchemy", loot: "alchemy_fire", count: 3 },
    CrimeInfo { name: "Arson", loot: "firebomb", count: 5 },
    CrimeInfo { name: "Blasphemy", loot: "censer", count: 1 },
    CrimeInfo { name: "Brigandry", loot: "dagger_brigand", count: 1 },
    CrimeInfo { name: "Drunkenness", loot: "wineskin", count: 5 },
    CrimeInfo { name: "Forgery", loot: "key_forged", count: 1 },
    CrimeInfo { name: "Heresy", loot: "key_fumie", count: 1 },
    CrimeInfo { name: "Lasciviousness", loot: "key_hair", count: 1 },
    CrimeInfo { name: "Smuggling", loot: "key_lantern", count: 1 },
    CrimeInfo { name: "Sumptuousness", loot: "ring_signet", count: 1 },
    CrimeInfo { name: "Usury", loot: "silverbag_1000", count: 1 },
    CrimeInfo { name: "Vagrancy", loot: "key_doll", count: 1 },
];

#[allow(dead_code)]
const UNUSED_SLOT_NOTE: &str = "Unused: reserved slot. The game stores 11 cosmetics but only reads the first 10 (everything the character creator offers), so this is a leftover, probably a cut customization option. Changing it has no effect in game.";

/// Draws a preview image inside a tile box, keeping its aspect ratio.
fn tile_image(
    ui: &mut Ui,
    tex: &Option<egui::TextureHandle>,
    tile_box: egui::Vec2,
) -> egui::Response {
    match tex {
        Some(tex) => {
            let size = tex.size_vec2();
            let scale = (tile_box.x / size.x.max(1.0)).min(tile_box.y / size.y.max(1.0));
            ui.add(egui::Button::image(
                egui::Image::from_texture(tex).fit_to_exact_size(size * scale),
            ))
        }
        None => ui.allocate_response(tile_box, egui::Sense::click()),
    }
}

/// Palette color of a color slot (eye color and the three color catalog slots), used for the swatch under the sprite tiles.
/// Mirrors the tint the preview applies.
fn swatch_color(slot: usize, choice: usize, hazeburnt: bool) -> Option<(u8, u8, u8)> {
    match slot {
        2 => EyeCatalog::get_all().get(choice).map(|c| (c.r, c.g, c.b)),
        4 | 6 | 7 => ColorCatalog::get_all().get(choice).map(|c| {
            if hazeburnt {
                (c.burnt_r, c.burnt_g, c.burnt_b)
            } else {
                (c.r, c.g, c.b)
            }
        }),
        _ => None,
    }
}

/// Class and crime names for the fallback dropdowns (same tables the sprite/text pickers use).
fn class_name(i: usize) -> Option<&'static str> {
    CLASSES.get(i).map(|c| c.name)
}

fn crime_name(i: usize) -> Option<&'static str> {
    CRIMES.get(i).map(|c| c.name)
}

/// Which preview view shows a slot's effect best.
fn slot_view(slot: usize) -> PreviewView {
    match slot {
        // Hair and beard are only visible on the head, eye and eyebrow colors on the face.
        2 | 7 => PreviewView::Face,
        3 | 4 | 5 | 6 => PreviewView::Head,
        _ => PreviewView::Full,
    }
}

impl SaveEditor {
    /// Char def candidates for the player, in the order the game would resolve them:
    /// the "hero" monster def's def field first, then CharDefMgr's fallback entry ("base").
    fn player_char_def_candidates(&self) -> Vec<String> {
        use crate::player_preview::{FALLBACK_CHAR_DEF, PLAYER_MONSTER_DEF};
        let mut candidates: Vec<String> = Vec::new();
        if let Some(cat) = &self.monster_catalog {
            if let Some(def) = cat
                .monsters
                .iter()
                .find(|m| m.name == PLAYER_MONSTER_DEF)
                .or_else(|| cat.monsters.iter().find(|m| m.name == "hero2"))
            {
                if !def.def.is_empty() {
                    candidates.push(def.def.clone());
                }
            }
        }
        candidates.push(FALLBACK_CHAR_DEF.to_string());
        candidates
    }

    pub fn show_cosmetics_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        // The hazeburnt body variant comes from the save's stats (set when the player is burned).
        // Hazeburnt uses the game's burnt color variants (grays), so the tab lets the preview switch bodies without touching the save.
        let save_hazeburnt = save.stats.hazeburnt;
        let hazeburnt = self.cosmetics_preview_hazeburnt.unwrap_or(save_hazeburnt);
        let current = CosmeticSelection::from_cosmetics(&save.cosmetics, hazeburnt);

        // Compose the preview lazily; it needs the game folder for the sheets and the hero char def.
        if self.player_preview.is_none() && self.player_preview_error.is_none() {
            match self.config.game_path.clone() {
                Some(game_path) => {
                    let char_defs = self.player_char_def_candidates();
                    match crate::player_preview::PlayerPreview::load(&game_path, &char_defs) {
                        Ok(preview) => {
                            self.player_preview = Some(preview);
                            self.player_preview_error = None;
                        }
                        Err(e) => self.player_preview_error = Some(e),
                    }
                }
                None => {}
            }
        }

        if self.player_preview.is_none() {
            self.show_cosmetics_fallback(ui, save);
            return;
        }

        // The player preview sits under the options, so it stays visible while they scroll.
        let preview_height = (ui.available_height() * 0.35).clamp(150.0, 320.0);
        egui::Panel::bottom("cosmetics_player_preview")
            .resizable(true)
            .default_size(preview_height)
            .min_size(120.0)
            .show(ui, |ui| {
                self.show_player_preview(ui, &current);
            });

        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // Body picker: Human / Hazeburnt, preview only.
                // First entry of the grid, so it scrolls with everything else instead of being pinned above it.
                self.show_body_picker(ui, &current, save_hazeburnt, hazeburnt);

                for slot_idx in 0..save.cosmetics.len() {
                    let value = save.cosmetics[slot_idx];
                    match slot_idx {
                        0 => self.show_sprite_slot(
                            ui,
                            &current,
                            slot_idx,
                            "Sex",
                            (0..SexCatalog::len()).collect(),
                            |i| SexCatalog::name(i).map(str::to_string),
                            value,
                            &mut save.cosmetics,
                        ),
                        1 => self.show_sprite_slot(
                            ui,
                            &current,
                            slot_idx,
                            "Ancestry",
                            (0..AncestryCatalog::len()).collect(),
                            |i| AncestryCatalog::name(i).map(str::to_string),
                            value,
                            &mut save.cosmetics,
                        ),
                        2 => self.show_sprite_slot(
                            ui,
                            &current,
                            slot_idx,
                            "Eye Color",
                            (0..EyeCatalog::len()).collect(),
                            |i| EyeCatalog::name(i).map(str::to_string),
                            value,
                            &mut save.cosmetics,
                        ),
                        3 => self.show_sprite_slot(
                            ui,
                            &current,
                            slot_idx,
                            "Hair",
                            HairCatalog::get_ordered_indices(),
                            |i| HairCatalog::name(i).map(str::to_string),
                            value,
                            &mut save.cosmetics,
                        ),
                        4 => self.show_sprite_slot(
                            ui,
                            &current,
                            slot_idx,
                            "Hair Color",
                            (0..ColorCatalog::len()).collect(),
                            |i| ColorCatalog::name(i).map(str::to_string),
                            value,
                            &mut save.cosmetics,
                        ),
                        5 => self.show_sprite_slot(
                            ui,
                            &current,
                            slot_idx,
                            "Beard",
                            (0..BeardCatalog::len()).collect(),
                            |i| BeardCatalog::name(i).map(str::to_string),
                            value,
                            &mut save.cosmetics,
                        ),
                        6 => self.show_sprite_slot(
                            ui,
                            &current,
                            slot_idx,
                            "Beard Color",
                            (0..ColorCatalog::len()).collect(),
                            |i| ColorCatalog::name(i).map(str::to_string),
                            value,
                            &mut save.cosmetics,
                        ),
                        7 => self.show_sprite_slot(
                            ui,
                            &current,
                            slot_idx,
                            "Eyebrow Color",
                            (0..ColorCatalog::len()).collect(),
                            |i| ColorCatalog::name(i).map(str::to_string),
                            value,
                            &mut save.cosmetics,
                        ),
                        8 => self.show_text_slot(
                            ui,
                            slot_idx,
                            "Class",
                            (0..CLASSES.len()).collect(),
                            |i| CLASSES.get(i).map(|c| c.name.to_string()),
                            value,
                            &mut save.cosmetics,
                            self.class_note(value),
                        ),
                        9 => self.show_text_slot(
                            ui,
                            slot_idx,
                            "Crime",
                            (0..CRIMES.len()).collect(),
                            |i| CRIMES.get(i).map(|c| c.name.to_string()),
                            value,
                            &mut save.cosmetics,
                            self.crime_note(value),
                        ),
                        // Unused slot 10: nothing in the game reads cosmetic[10] (see
                        // UNUSED_SLOT_NOTE), so it is commented out instead of rendered.
                        // Kept here so the field can be brought back if it ever means something.
                        //
                        // _ => {
                        //     // Unused slot, bare drag value
                        //     ui.horizontal(|ui| {
                        //         ui.label("Unused:");
                        //         ui.add(
                        //             egui::DragValue::new(&mut save.cosmetics[slot_idx])
                        //                 .speed(self.config.drag_value_sensitivity)
                        //                 .range(0..=999),
                        //         )
                        //         .on_hover_text(UNUSED_SLOT_NOTE);
                        //     });
                        // }
                        _ => {}
                    }
                    ui.add_space(6.0);
                }
            });
    }

    /// Human / Hazeburnt body picker.
    /// Hazeburnt is the save's burned state; it uses the game's burnt variants for the skin and every color, which look gray, so this lets the preview switch bodies without editing the save.
    fn show_body_picker(
        &mut self,
        ui: &mut Ui,
        current: &CosmeticSelection,
        save_hazeburnt: bool,
        effective: bool,
    ) {
        let mut tiles: Vec<(bool, String, Option<egui::TextureHandle>)> = Vec::new();
        if let Some(preview) = self.player_preview.as_mut() {
            for (value, label) in [(false, "Human"), (true, "Hazeburnt")] {
                let mut sel = current.clone();
                sel.hazeburnt = value;
                let tex = preview.texture(ui.ctx(), &sel, PreviewView::Full);
                tiles.push((value, label.to_string(), tex));
            }
        }

        let icon_size = self.config.item_icon_size;
        let font_size = self.config.grid_font_size;
        let tile_box = egui::vec2(icon_size, icon_size * 2.4);

        // Same block layout as the Sex and Ancestry slots: a header line, then a note line (always present, so nothing jumps around when the selection changes), then the tiles.
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Body:").strong());
            ui.colored_label(
                egui::Color32::GRAY,
                format!("{} ({})", if effective { "Hazeburnt" } else { "Human" }, if effective { 1 } else { 0 }),
            );
        });
        ui.horizontal(|ui| {
            if effective == save_hazeburnt {
                ui.colored_label(egui::Color32::GRAY, "Preview only. Hazeburnt uses the game's burnt color variants (grays).",);
            } else {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    format!(
                        "Preview only, the save says {}.",
                        if save_hazeburnt { "Hazeburnt" } else { "Human" }
                    ),
                );
                if self.cosmetics_preview_hazeburnt.is_some()
                    && ui
                        .small_button("Use save value")
                        .on_hover_text("Preview the body the save actually uses")
                        .clicked()
                {
                    self.cosmetics_preview_hazeburnt = None;
                }
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.style_mut().interaction.selectable_labels = false;
            for (value, label, tex) in tiles {
                let selected = value == effective;
                ui.vertical(|ui| {
                    let response = tile_image(ui, &tex, tile_box);
                    crate::tabs::multisel::paint_sel_outline(ui, response.rect, selected);
                    ui.set_max_width(response.rect.width().max(icon_size) + 5.0);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&label).size(font_size).color(if selected {
                                egui::Color32::LIGHT_GREEN
                            } else {
                                ui.visuals().text_color()
                            }),
                        )
                        .wrap_mode(egui::TextWrapMode::Extend)
                        .halign(egui::Align::Center)
                        .show_tooltip_when_elided(false),
                    );
                    if response.clicked() {
                        self.cosmetics_preview_hazeburnt = Some(value);
                    }
                    response.on_hover_text(if value {
                        "Burnt body: the game swaps the skin and every color to its burnt (gray) variant."
                    } else {
                        "Human body: the normal skin and color palette."
                    });
                });
            }
        });
    }

    /// One cosmetic slot rendered as a grid of sprite tiles.
    /// Every tile shows the character as it would look with that choice, so the sprites are the real in-game ones.
    fn show_sprite_slot(
        &mut self,
        ui: &mut Ui,
        current: &CosmeticSelection,
        slot_idx: usize,
        label: &str,
        choices: Vec<usize>,
        name_fn: impl Fn(usize) -> Option<String>,
        value: i32,
        cosmetics: &mut [i32],
    ) {
        // Compose every candidate first (this borrows the preview mutably), then draw the grid.
        let view = slot_view(slot_idx);
        let mut tiles: Vec<(usize, String, Option<egui::TextureHandle>)> = Vec::new();
        if let Some(preview) = self.player_preview.as_mut() {
            for &choice in &choices {
                let mut sel = current.clone();
                sel.set_slot(slot_idx, choice);
                // Color tiles need something to color: with a bald head or no beard the sprite would be identical for every color, so those tiles preview a default style.
                if slot_idx == 4 && sel.hair_texture().is_none() {
                    sel.hair = 1; // Short
                }
                if slot_idx == 6 && sel.beard_texture().is_none() {
                    sel.beard = 1;
                }
                let tex = preview.texture(ui.ctx(), &sel, view);
                let name = name_fn(choice).unwrap_or_else(|| format!("{}", choice));
                tiles.push((choice, name, tex));
            }
        }

        let icon_size = self.config.item_icon_size;
        let font_size = self.config.grid_font_size;
        let selected_idx = if value >= 0 { Some(value as usize) } else { None };
        // Full-body previews are tall, so their tiles are taller than wide; head and face previews are roughly square.
        // Images are fitted into the box without squashing them.
        let tile_box = match view {
            PreviewView::Full => egui::vec2(icon_size, icon_size * 2.4),
            _ => egui::vec2(icon_size, icon_size),
        };
        let hazeburnt = current.hazeburnt;

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{}:", label)).strong());
            let current_name = selected_idx
                .and_then(&name_fn)
                .unwrap_or_else(|| format!("{}", value));
            ui.colored_label(egui::Color32::GRAY, format!("{} ({})", current_name, value));
        });

        // The whole tab scrolls in both axes (like the equipment and bestiary tabs), so the rows here are plain wrapped grids. Label selection is off so click-drag scrolling works.
        ui.horizontal_wrapped(|ui| {
            ui.style_mut().interaction.selectable_labels = false;
            for (choice, name, tex) in tiles {
                        let selected = selected_idx == Some(choice);
                        ui.vertical(|ui| {
                            let response = tile_image(ui, &tex, tile_box);
                            crate::tabs::multisel::paint_sel_outline(ui, response.rect, selected);
                            ui.set_max_width(response.rect.width().max(icon_size) + 5.0);
                            if self.config.cosmetic_color_swatches {
                            if let Some((r, g, b)) = swatch_color(slot_idx, choice, hazeburnt) {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(response.rect.width().max(icon_size), 7.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    1.0,
                                    egui::Color32::from_rgb(r, g, b),
                                );
                            }
                            }
                            for word in name.split_whitespace() {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(word).size(font_size).color(
                                            if selected {
                                                egui::Color32::LIGHT_GREEN
                                            } else {
                                                ui.visuals().text_color()
                                            },
                                        ),
                                    )
                                    // Extend instead of truncating: a long word such as "Hazeburnt" must never break mid-word.
                                    .wrap_mode(egui::TextWrapMode::Extend)
                                    .halign(egui::Align::Center)
                                    .show_tooltip_when_elided(false),
                                );
                            }
                            if response.clicked() {
                                if slot_idx < cosmetics.len() {
                                    cosmetics[slot_idx] = choice as i32;
                                }
                            }
                    response.on_hover_text(format!("{} ({})", name, choice));
                });
            }
        });
    }

    /// Slots without an in-game sprite (class and crime) as text tiles, with a note describing what the current pick does in game.
    fn show_text_slot(
        &mut self,
        ui: &mut Ui,
        slot_idx: usize,
        label: &str,
        choices: Vec<usize>,
        name_fn: impl Fn(usize) -> Option<String>,
        value: i32,
        cosmetics: &mut [i32],
        note: String,
    ) {
        let font_size = self.config.grid_font_size;
        let selected_idx = if value >= 0 { Some(value as usize) } else { None };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("{}:", label)).strong());
            let current_name = selected_idx
                .and_then(&name_fn)
                .unwrap_or_else(|| format!("{}", value));
            ui.colored_label(egui::Color32::GRAY, format!("{} ({})", current_name, value));
        });
        // Text-only slots, label selection disabled like the sprite rows.
        ui.horizontal_wrapped(|ui| {
            ui.style_mut().interaction.selectable_labels = false;
            for choice in choices {
                let name = name_fn(choice).unwrap_or_else(|| format!("{}", choice));
                let selected = selected_idx == Some(choice);
                let response = ui.add(
                    egui::Button::new(egui::RichText::new(&name).size(font_size))
                        .selected(selected),
                );
                if response.clicked() && slot_idx < cosmetics.len() {
                    cosmetics[slot_idx] = choice as i32;
                }
                response.on_hover_text(format!("{} ({})", name, choice));
            }
        });
        ui.add_space(2.0);
        ui.label(egui::RichText::new(note).color(egui::Color32::GRAY));
    }

    /// Loot name -> display title, using the loaded catalog when available.
    fn loot_title(&self, loot_name: &str) -> String {
        self.catalog
            .as_ref()
            .and_then(|c| c.loot_defs.iter().find(|d| d.name == loot_name))
            .and_then(|d| d.title.iter().find(|t| !t.is_empty()).cloned())
            .unwrap_or_else(|| loot_name.replace('_', " "))
    }

    /// What the selected class is called in game and what it starts you with.
    fn class_note(&self, value: i32) -> String {
        let Some(info) = usize::try_from(value).ok().and_then(|i| CLASSES.get(i)) else {
            return "In game: unknown class index.".to_string();
        };
        let gear: Vec<String> = info.gear.iter().map(|g| self.loot_title(g)).collect();
        let [str_, dex, vit, wil, end, arc, conv, res, lck] = info.stats;
        format!(
            "In game: {}. Starts you with: {}. Starting stats: STR {} DEX {} VIT {} WIL {} END {} ARC {} CONV {} RES {} LCK {}, plus {} class skill unlocks.",
            info.name,
            gear.join(", "),
            str_, dex, vit, wil, end, arc, conv, res, lck,
            info.unlocks.len()
        )
    }

    /// What the selected crime is called in game and what it does.
    fn crime_note(&self, value: i32) -> String {
        let Some(info) = usize::try_from(value).ok().and_then(|i| CRIMES.get(i)) else {
            return "In game: unknown crime index.".to_string();
        };
        let title = self.loot_title(info.loot);
        let count = if info.count > 1 {
            format!(" (x{})", info.count)
        } else {
            String::new()
        };
        format!(
            "In game: {}. Flavor only: it grants {} {} at the start and NPC dialog can reference it; it has no other effect.",
            info.name, title, count
        )
    }

    /// Body plus face close-up of the current loadout, with the chosen names listed.
    fn show_player_preview(&mut self, ui: &mut Ui, current: &CosmeticSelection) {
        let Some(preview) = self.player_preview.as_mut() else {
            return;
        };
        let body = preview.texture(ui.ctx(), current, PreviewView::Full);
        let face = preview.texture(ui.ctx(), current, PreviewView::Face);
        let name = |f: fn(usize) -> Option<&'static str>, idx: usize| {
            f(idx).map(str::to_string).unwrap_or_default()
        };

        ui.horizontal(|ui| {
            let body_h = (ui.available_height() - 8.0).clamp(96.0, 260.0);
            if let Some(tex) = &body {
                let size = tex.size_vec2();
                let scale = body_h / size.y.max(1.0);
                ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size * scale));
            } else {
                ui.colored_label(egui::Color32::GRAY, "Body sheet not found for this ancestry.");
            }
            let face_h = (body_h * 0.6).clamp(80.0, 180.0);
            if let Some(tex) = &face {
                let size = tex.size_vec2();
                let scale = face_h / size.y.max(1.0);
                ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size * scale));
            }
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Player preview").strong());
                ui.label(format!(
                    "{} {}",
                    name(SexCatalog::name, current.sex),
                    name(AncestryCatalog::name, current.ancestry)
                ));
                ui.label(format!(
                    "Eye color: {}",
                    name(EyeCatalog::name, current.eye)
                ));
                ui.label(format!(
                    "Hair: {} - {}",
                    name(HairCatalog::name, current.hair),
                    name(ColorCatalog::name, current.hair_color)
                ));
                ui.label(format!(
                    "Beard: {} - {}",
                    name(BeardCatalog::name, current.beard),
                    name(ColorCatalog::name, current.beard_color)
                ));
                ui.label(format!(
                    "Eyebrows: {}",
                    name(ColorCatalog::name, current.eyebrow_color)
                ));
                if current.hazeburnt {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new("Hazeburnt (herox body)")
                                .color(egui::Color32::YELLOW),
                        )
                        .wrap_mode(egui::TextWrapMode::Extend),
                    );
                }
            });
        });
    }

    /// Old dropdown UI, used when the game folder (or its hero assets) is unavailable.
    fn show_cosmetics_fallback(&mut self, ui: &mut Ui, save: &mut SaveData) {
        type NameFn = fn(usize) -> Option<&'static str>;

        if let Some(err) = &self.player_preview_error {
            ui.colored_label(
                egui::Color32::YELLOW,
                format!(
                    "Player preview unavailable ({}). Falling back to dropdowns.",
                    err
                ),
            );
        } else {
            ui.colored_label(
                egui::Color32::YELLOW,
                "Set the game folder to see the actual cosmetic sprites and the player preview.",
            );
        }

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
                    class_name as NameFn,
                    (0..CLASSES.len()).collect(),
                ),
                9 => (
                    "Crime",
                    crime_name as NameFn,
                    (0..CRIMES.len()).collect(),
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
