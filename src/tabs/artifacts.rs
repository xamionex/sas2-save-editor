use crate::app::SaveEditor;
use crate::artifact::{
    ARTIFACT_FIELDS, artifact_main_field, artifact_rarity, artifact_seed, artifact_tier,
    charm_effective, charm_unit, charm_vanilla, compute_artifact_values, effective_value_range,
    find_matching_seeds, find_matching_seeds_all_tiers, load_resalter_artifact_boosts,
    seed_for_tier, CharmUnit,
};
use eframe::egui;
use egui::Ui;
use sas2_parser::{SaveData, loot_names};

/// Talisman slots used by GetCharmVal: 7/8 = ring slots, 9 = amulet, 20 = dagger.
const CHARM_SLOTS: [i32; 4] = [7, 8, 9, 20];

/// Height of the reserved warning area while desired values are being edited.
const WARN_AREA_HEIGHT: f32 = 64.0;

/// The seed range for a tier: tier * 2000 + 1..=2000.
fn tier_seed_range(tier: i32) -> (i32, i32) {
    (tier * 2000 + 1, tier * 2000 + 2000)
}

/// Reroll the seed of an artifact, keeping its tier.
fn reroll_seed(item: &mut sas2_parser::Item) {
    let seed = artifact_seed(item);
    let tier = artifact_tier(seed);
    let (min, max) = tier_seed_range(tier);
    let new_seed = min + (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as i32)
        .unwrap_or(0)
        .rem_euclid(max - min + 1));
    item.artifact_seed = new_seed;
    item.upgrade = new_seed;
}

/// Owned snapshot of an artifact item for the list.
struct ArtifactEntry {
    inv_idx: usize,
    seed: i32,
    subtype: i32,
    name: String,
}

impl SaveEditor {
    pub fn show_artifacts_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        ui.add(
            egui::Label::new(
                "Artifact values are not stored in the save: the game rolls them from a seed \
                 on load. Edit the seed or reroll it to change the values. Talisman boosts \
                 come from the equipped talismans' flags, shown below for reference.",
            )
            .wrap(),
        );

        // Collect artifact items (type 6, subtype 3/4/5) as owned data.
        let catalog = self.catalog.as_ref();
        let mut artifacts: Vec<ArtifactEntry> = Vec::new();
        for (idx, item) in save.equipment.inventory_items.iter().enumerate() {
            let Some(def) = catalog.and_then(|c| c.loot_defs.get(item.loot_idx as usize)) else {
                continue;
            };
            if def.type_ == 6 && (3..=5).contains(&def.sub_type) {
                let name = def
                    .title
                    .first()
                    .filter(|t| !t.is_empty())
                    .cloned()
                    .unwrap_or_else(|| format!("Item {}", item.loot_idx));
                artifacts.push(ArtifactEntry {
                    inv_idx: idx,
                    seed: artifact_seed(item),
                    subtype: def.sub_type,
                    name,
                });
            }
        }

        if artifacts.is_empty() {
            ui.label("No artifacts (talisman subtype Attack/Defense/Utility) in the inventory.");
            return;
        }

        let mut selected: Option<usize> = self.selected_artifact;
        let mut reroll_requested: Option<usize> = None;
        let mut search_matches_requested: Option<usize> = None;

        // Resalter override info for the impossibility check, loaded once.
        let resalter_boosts = self
            .config
            .game_path
            .as_deref()
            .map(load_resalter_artifact_boosts)
            .unwrap_or_default();

        // Left panel: artifact list
        egui::Panel::left("artifact_list")
            .resizable(true)
            .default_size(280.0)
            .min_size(180.0)
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                ui.label(egui::RichText::new("Artifacts").strong());
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for entry in &artifacts {
                            let tier = artifact_tier(entry.seed);
                            let values = compute_artifact_values(entry.seed, entry.subtype, tier);
                            let rarity = artifact_rarity(&values);
                            let label = format!(
                                "{} (T{}{})",
                                entry.name,
                                tier,
                                if rarity > 0 {
                                    format!(", R{}", rarity)
                                } else {
                                    String::new()
                                }
                            );
                            if ui
                                .selectable_label(selected == Some(entry.inv_idx), label)
                                .clicked()
                            {
                                if selected != Some(entry.inv_idx) {
                                    self.artifact_search_result = None;
                                    self.artifact_matches.clear();
                                    self.artifact_match_picker_open = false;
                                }
                                selected = Some(entry.inv_idx);
                            }
                        }
                    });
            });

        // Central panel: selected artifact details
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let Some(sel_idx) = selected else {
                ui.label("Select an artifact.");
                return;
            };
            let Some(entry) = artifacts.iter().find(|e| e.inv_idx == sel_idx) else {
                ui.label("Selected artifact not found.");
                return;
            };
            let seed = entry.seed;
            let subtype = entry.subtype;
            let tier = artifact_tier(seed);
            let values = compute_artifact_values(seed, subtype, tier);
            let main_field = artifact_main_field(subtype);
            let subtype_name = loot_names::get_subtype_name(6, subtype);

            ui.heading(&entry.name);
            ui.label(format!("Type: Charm - {}", subtype_name));
            ui.label(format!("Tier: {} (seed {})", tier, seed));
            ui.label(format!(
                "Rarity: {}",
                match artifact_rarity(&values) {
                    0 => "Common".to_string(),
                    1 => "Rare".to_string(),
                    2 => "Very Rare".to_string(),
                    3 => "Legendary".to_string(),
                    _ => "Epic".to_string(),
                }
            ));
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Tier:");
                let mut tier_edit = tier;
                if ui
                    .add(
                        egui::DragValue::new(&mut tier_edit)
                            .speed(0.1)
                            .range(0..=100),
                    )
                    .changed()
                {
                    if let Some(item) = save.equipment.inventory_items.get_mut(sel_idx) {
                        let new_seed = seed_for_tier(artifact_seed(item), tier_edit);
                        item.artifact_seed = new_seed;
                        item.upgrade = new_seed;
                    }
                }
                ui.label("Seed:");
                let mut seed_edit = seed;
                if ui
                    .add(
                        egui::DragValue::new(&mut seed_edit)
                            .speed(self.config.drag_value_sensitivity)
                            .range(0..=i32::MAX),
                    )
                    .changed()
                {
                    if let Some(item) = save.equipment.inventory_items.get_mut(sel_idx) {
                        item.artifact_seed = seed_edit;
                        item.upgrade = seed_edit;
                    }
                }
                if ui
                    .button("Reroll")
                    .on_hover_text("Roll a new seed in the same tier")
                    .clicked()
                {
                    reroll_requested = Some(sel_idx);
                }
                // Find matching artifacts: lists all seeds in the tier (or all tiers with the checkbox) that contain the desired stats.
                let has_desired = self
                    .artifact_desired_values
                    .values()
                    .any(|d| *d > 0.0);
                let resp = ui.add_enabled(
                    has_desired,
                    egui::Button::new("Find matching artifacts"),
                );
                if resp
                    .on_hover_text(
                        "List all artifacts that contain the desired stats, for you to pick from",
                    )
                    .clicked()
                {
                    search_matches_requested = Some(sel_idx);
                }
                ui.checkbox(
                    &mut self.artifact_try_all_tiers,
                    "Try all tiers",
                );
            });
            ui.separator();

            // QoL buttons: apply to currently possible boosts only.
            let mut qol_reset = false;
            let mut qol_current = false;
            let mut qol_max = false;
            ui.horizontal(|ui| {
                if ui
                    .button("Reset desired to 0")
                    .on_hover_text("Clear all desired values")
                    .clicked()
                {
                    qol_reset = true;
                }
                if ui
                    .button("Set desired to current")
                    .on_hover_text("Set desired to the current value of each possible boost")
                    .clicked()
                {
                    qol_current = true;
                }
                if ui
                    .button("Set desired to max")
                    .on_hover_text("Set desired to the max of each possible boost")
                    .clicked()
                {
                    qol_max = true;
                }
            });
            if qol_reset {
                self.artifact_desired_values.clear();
                self.artifact_search_result = None;
            }
            if qol_current {
                for (field_id, _) in ARTIFACT_FIELDS {
                    let v = values[*field_id as usize];
                    if v > 0.0
                        && effective_value_range(
                            subtype,
                            tier,
                            *field_id,
                            resalter_boosts.get(field_id),
                        )
                        .is_some()
                    {
                        self.artifact_desired_values.insert(*field_id, v);
                    }
                }
                self.artifact_search_result = None;
            }
            if qol_max {
                for (field_id, _) in ARTIFACT_FIELDS {
                    // Only set fields the artifact actually has (nonzero value).
                    let v = values[*field_id as usize];
                    if v > 0.0 {
                        if let Some((_, max)) = effective_value_range(
                            subtype,
                            tier,
                            *field_id,
                            resalter_boosts.get(field_id),
                        ) {
                            self.artifact_desired_values.insert(*field_id, max);
                        }
                    }
                }
                self.artifact_search_result = None;
            }
            ui.separator();

            // Impossibility warnings and search result.
            // The area is always reserved with a fixed height so the grid below never shifts, even on the very first drag from 0 (when the warning appears mid-drag and would otherwise move the widget under the cursor and drop the input focus).
            let mut warnings: Vec<String> = Vec::new();
            for (field_id, field_name) in ARTIFACT_FIELDS {
                let Some(desired) = self.artifact_desired_values.get(field_id).copied() else {
                    continue;
                };
                if desired <= 0.0 {
                    continue;
                }
                let possible = effective_value_range(
                    subtype,
                    tier,
                    *field_id,
                    resalter_boosts.get(field_id),
                )
                .map(|(min, max)| {
                    // The search matches within 0.05.
                    // The min is achievable, but the max is exclusive (NextDouble() < 1.0), so the upper bound needs a tiny epsilon to catch values like 3.30 when the max is 3.25 (the closest achievable value is ~3.25).
                    desired >= min - 0.05 && desired <= max + 0.05 - 0.0001
                })
                .unwrap_or(false);
                if !possible {
                    let range_text = effective_value_range(
                        subtype,
                        tier,
                        *field_id,
                        resalter_boosts.get(field_id),
                    )
                    .map(|(min, max)| format!("{:.1}-{:.1}%", min, max))
                    .unwrap_or_else(|| "never set".to_string());
                    warnings.push(format!(
                        "Impossible: {} = {:.1}% is outside the possible range ({})",
                        field_name, desired, range_text
                    ));
                }
            }
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), WARN_AREA_HEIGHT),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    for warning in &warnings {
                        ui.colored_label(egui::Color32::RED, warning);
                    }
                    // Search result message, kept with the warnings so it is visible without scrolling the value grid.
                    if let Some((found, message)) = &self.artifact_search_result {
                        if *found {
                            ui.colored_label(egui::Color32::GREEN, message);
                        } else {
                            ui.colored_label(egui::Color32::RED, message);
                        }
                    }
                },
            );
            ui.separator();

            // Value grid
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::Grid::new("artifact_values")
                        .num_columns(5)
                        .spacing([16.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Field").strong());
                            ui.label(egui::RichText::new("Current").strong());
                            ui.label(egui::RichText::new("Desired").strong());
                            ui.label(egui::RichText::new("Min/Max").strong());
                            ui.label("");
                            ui.end_row();

                            for (field_id, field_name) in ARTIFACT_FIELDS {
                                let v = values[*field_id as usize];
                                let label = if *field_id == main_field {
                                    format!("{} (main)", field_name)
                                } else {
                                    field_name.to_string()
                                };
                                if v > 0.0 {
                                    ui.label(egui::RichText::new(&label).strong());
                                    ui.label(format!("+{:.1}%", v));
                                } else {
                                    ui.label(egui::RichText::new(&label).weak());
                                    ui.label(egui::RichText::new("-").weak());
                                }

                                // Desired value input
                                let desired = self
                                    .artifact_desired_values
                                    .entry(*field_id)
                                    .or_insert(0.0);
                                let desired_resp = ui.add(
                                    egui::DragValue::new(desired)
                                        .speed(self.config.drag_value_sensitivity)
                                        .range(0.0..=100.0)
                                        .suffix("%"),
                                );
                                if desired_resp.changed() {
                                    self.artifact_search_result = None;
                                    self.artifact_matches.clear();
                                    self.artifact_match_picker_open = false;
                                }

                                // Per-row QoL buttons, right of the desired input.
                                let range = effective_value_range(
                                    subtype,
                                    tier,
                                    *field_id,
                                    resalter_boosts.get(field_id),
                                );
                                ui.horizontal(|ui| {
                                    if let Some((rmin, rmax)) = range {
                                        if ui
                                            .small_button("min")
                                            .on_hover_text("Set desired to the minimum")
                                            .clicked()
                                        {
                                            *desired = rmin;
                                            self.artifact_search_result = None;
                                            self.artifact_matches.clear();
                                            self.artifact_match_picker_open = false;
                                        }
                                        if ui
                                            .small_button("max")
                                            .on_hover_text("Set desired to the maximum")
                                            .clicked()
                                        {
                                            *desired = rmax;
                                            self.artifact_search_result = None;
                                            self.artifact_matches.clear();
                                            self.artifact_match_picker_open = false;
                                        }
                                    }
                                    if ui
                                        .small_button("0")
                                        .on_hover_text("Set desired to 0 (clear)")
                                        .clicked()
                                    {
                                        *desired = 0.0;
                                        self.artifact_search_result = None;
                                        self.artifact_matches.clear();
                                        self.artifact_match_picker_open = false;
                                    }
                                });

                                // Min/Max column: Resalter override when present, else vanilla.
                                let has_resalter = resalter_boosts.contains_key(field_id);
                                match effective_value_range(
                                    subtype,
                                    tier,
                                    *field_id,
                                    resalter_boosts.get(field_id),
                                ) {
                                    Some((min, max)) => {
                                        let tag = if has_resalter {
                                            "Resalter"
                                        } else {
                                            "vanilla"
                                        };
                                        if (max - min).abs() < 0.001 {
                                            ui.label(format!("{:.1}% ({})", min, tag));
                                        } else {
                                            ui.label(format!("{:.1}-{:.1}% ({})", min, max, tag));
                                        }
                                    }
                                    None => {
                                        ui.label(egui::RichText::new("-").weak());
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
        });

        if let Some(idx) = reroll_requested {
            if let Some(item) = save.equipment.inventory_items.get_mut(idx) {
                reroll_seed(item);
            }
        }
        if let Some(idx) = search_matches_requested {
            let Some(entry) = artifacts.iter().find(|e| e.inv_idx == idx) else {
                return;
            };
            let tier = artifact_tier(entry.seed);
            let subtype = entry.subtype;
            // Collect all desired values (fields with a nonzero desired value).
            let desired: Vec<(i32, f32)> = self
                .artifact_desired_values
                .iter()
                .filter(|(_, d)| **d > 0.0)
                .map(|(f, d)| (*f, *d))
                .collect();
            if desired.is_empty() {
                self.artifact_search_result = Some((
                    false,
                    "No desired values set. Set a desired value on a field first.".to_string(),
                ));
                self.artifact_matches.clear();
            } else {
                // Fields the subtype can never roll are impossible together.
                let impossible: Vec<String> = desired
                    .iter()
                    .filter(|(f, _)| {
                        effective_value_range(subtype, 40, *f, resalter_boosts.get(f)).is_none()
                    })
                    .map(|(f, _)| {
                        ARTIFACT_FIELDS
                            .iter()
                            .find(|(id, _)| id == f)
                            .map(|(_, n)| n.to_string())
                            .unwrap_or_else(|| format!("field {}", f))
                    })
                    .collect();
                if !impossible.is_empty() {
                    self.artifact_search_result = Some((
                        false,
                        format!(
                            "This artifact type (subtype {}) can never roll: {}",
                            subtype,
                            impossible.join(", ")
                        ),
                    ));
                    self.artifact_matches.clear();
                } else if self.artifact_try_all_tiers {
                    let matches = find_matching_seeds_all_tiers(subtype, 40, &desired);
                    self.artifact_matches = matches;
                    if self.artifact_matches.is_empty() {
                        self.artifact_search_result = Some((
                            false,
                            "No artifact in any tier contains all the desired stats.".to_string(),
                        ));
                    } else {
                        self.artifact_search_result = Some((
                            true,
                            format!(
                                "Found {} artifact(s) across all tiers containing the desired stats.",
                                self.artifact_matches.len()
                            ),
                        ));
                        self.artifact_match_target = Some(idx);
                        self.artifact_match_search.clear();
                        self.artifact_match_focus = true;
                        self.artifact_match_picker_open = true;
                    }
                } else {
                    let matches = find_matching_seeds(subtype, tier, &desired);
                    self.artifact_matches = matches;
                    if self.artifact_matches.is_empty() {
                        self.artifact_search_result = Some((
                            false,
                            format!(
                                "No artifact in tier {} contains all the desired stats.",
                                tier
                            ),
                        ));
                    } else {
                        self.artifact_search_result = Some((
                            true,
                            format!(
                                "Found {} artifact(s) in tier {} containing all desired stats.",
                                self.artifact_matches.len(),
                                tier
                            ),
                        ));
                        self.artifact_match_target = Some(idx);
                        self.artifact_match_search.clear();
                        self.artifact_match_focus = true;
                        self.artifact_match_picker_open = true;
                    }
                }
            }
        }
        // Apply a chosen seed from the match picker (pending state set by the
        // Apply button click, processed here so the picker window can close).
        if let Some(new_seed) = self.artifact_pending_apply.take() {
            if let Some(target) = self.artifact_match_target {
                if let Some(item) = save.equipment.inventory_items.get_mut(target) {
                    item.artifact_seed = new_seed;
                    item.upgrade = new_seed;
                }
                self.artifact_match_picker_open = false;
                self.artifact_match_target = None;
                self.artifact_search_result = Some((
                    true,
                    format!("Applied seed {} to the artifact.", new_seed),
                ));
            }
        }
        self.selected_artifact = selected;

        // Match picker window: list all matching artifacts with their values.
        if self.artifact_match_picker_open {
            let mut open = self.artifact_match_picker_open;
            egui::Window::new("Matching artifacts")
                .collapsible(false)
                .resizable(true)
                .default_width(560.0)
                .default_height(480.0)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    ui.label("Search seed:");
                    let resp = ui.text_edit_singleline(&mut self.artifact_match_search);
                    if self.artifact_match_focus {
                        resp.request_focus();
                        self.artifact_match_focus = false;
                    }
                    ui.separator();
                    let needle = self.artifact_match_search.to_lowercase();
                    let desired: Vec<(i32, f32)> = self
                        .artifact_desired_values
                        .iter()
                        .filter(|(_, d)| **d > 0.0)
                        .map(|(f, d)| (*f, *d))
                        .collect();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            egui::Grid::new("artifact_matches")
                                .num_columns(desired.len() + 4)
                                .spacing([12.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Seed").strong());
                                    ui.label(egui::RichText::new("Tier").strong());
                                    for (f, _) in &desired {
                                        let name = ARTIFACT_FIELDS
                                            .iter()
                                            .find(|(id, _)| id == f)
                                            .map(|(_, n)| n.to_string())
                                            .unwrap_or_else(|| format!("field {}", f));
                                        ui.label(egui::RichText::new(&name).strong());
                                    }
                                    ui.label(egui::RichText::new("Closeness").strong());
                                    ui.label("");
                                    ui.end_row();

                                    for m in &self.artifact_matches {
                                        if !needle.is_empty()
                                            && !m.seed.to_string().contains(&needle)
                                        {
                                            continue;
                                        }
                                        ui.label(m.seed.to_string());
                                        ui.label(m.tier.to_string());
                                        for (f, actual) in &m.values {
                                            let want = desired
                                                .iter()
                                                .find(|(df, _)| df == f)
                                                .map(|(_, d)| *d)
                                                .unwrap_or(0.0);
                                            let diff = (actual - want).abs();
                                            let text = format!("{:.1}%", actual);
                                            if diff <= 0.05 {
                                                ui.colored_label(
                                                    egui::Color32::LIGHT_GREEN,
                                                    text,
                                                );
                                            } else {
                                                ui.label(text);
                                            }
                                        }
                                        ui.label(format!("{:.2}", m.error));
                                        if ui.button("Apply").clicked() {
                                            if self.artifact_match_target.is_some() {
                                                self.artifact_pending_apply = Some(m.seed);
                                            }
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                });
            self.artifact_match_picker_open = open;
        }

        ui.add_space(16.0);
        ui.separator();
        self.show_talismans_ui(ui, save);
    }

    /// Show equipped talismans and the effective charm values they produce.
    fn show_talismans_ui(&self, ui: &mut Ui, save: &SaveData) {
        ui.heading("Equipped Talismans");
        ui.add(
            egui::Label::new(
                "Charm boosts are computed from the flags of equipped talismans: each flag counts how many equipped talismans share it, giving a tier of 1.0 / 1.25 / 1.35 / 1.4. \
                The effective value is that tier times the flag's vanilla magnitude. \
                Nothing here is stored in the save.",
            )
            .wrap(),
        );
        ui.add_space(8.0);

        let catalog = self.catalog.as_ref();
        let items = &save.equipment.inventory_items;
        let equipped = &save.equipment.equipped_items;

        // Collect equipped talismans (slots 7/8/9/20) with their flags.
        let mut talisman_flags: Vec<(String, Vec<i32>)> = Vec::new();
        for slot in CHARM_SLOTS {
            let Some(&inv_idx) = equipped.get(slot as usize) else {
                continue;
            };
            if inv_idx < 0 || inv_idx as usize >= items.len() {
                continue;
            }
            let item = &items[inv_idx as usize];
            let Some(def) = catalog.and_then(|c| c.loot_defs.get(item.loot_idx as usize)) else {
                continue;
            };
            if def.type_ != 6 || def.sub_type > 2 {
                continue;
            }
            let name = def
                .title
                .first()
                .filter(|t| !t.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("Item {}", item.loot_idx));
            talisman_flags.push((name, def.flags.clone()));
        }

        if talisman_flags.is_empty() {
            ui.label("No talismans equipped (ring/amulet/dagger slots).");
            return;
        }

        // Count flags across equipped talismans.
        let mut flag_counts: std::collections::BTreeMap<i32, i32> = std::collections::BTreeMap::new();
        for (_, flags) in &talisman_flags {
            for flag in flags {
                *flag_counts.entry(*flag).or_insert(0) += 1;
            }
        }

        // Equipped talismans list
        ui.label(egui::RichText::new("Equipped:").strong());
        for (name, flags) in &talisman_flags {
            let flag_names: Vec<&str> = flags
                .iter()
                .map(|f| loot_names::get_flag_name(6, *f))
                .collect();
            ui.label(format!("{}: {}", name, flag_names.join(", ")));
        }

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Effective charm values:").strong());

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                egui::Grid::new("charm_values")
                    .num_columns(3)
                    .spacing([16.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Boost").strong());
                        ui.label(egui::RichText::new("Count").strong());
                        ui.label(egui::RichText::new("Effective").strong());
                        ui.end_row();

                        for (flag, count) in &flag_counts {
                            let name = loot_names::get_flag_name(6, *flag);
                            let unit = charm_unit(*flag);
                            let suffix = match unit {
                                CharmUnit::Percent => "%",
                                CharmUnit::Flat => "",
                            };
                            let effective = charm_effective(*flag, *count);
                            let vanilla = charm_vanilla(*flag);
                            ui.label(format!("[{}] {}", flag, name));
                            ui.label(count.to_string());
                            ui.label(format!(
                                "{:.1}{} (vanilla {:.1}{})",
                                effective, suffix, vanilla, suffix
                            ));
                            ui.end_row();
                        }
                    });
            });
    }
}
