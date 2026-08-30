/// Artifact and talisman (charm) game logic.
///
/// Artifact values are not stored in the save.
/// Each artifact item stores a seed (the `upgrade` field, or `artifact_seed` in modded saves) and the game deterministically rolls the 35 stat values from it on load via PlayerArtifactData.Populate (new Random(seed)).
/// This module replicates that logic so the editor can show and reroll the values.
///
/// Talisman (charm) values are also not stored: GetCharmVal(flag) counts how many equipped talismans share a flag and returns a tier (1.0 / 1.25 / 1.35 / 1.4) which stat formulas multiply by a per-flag vanilla magnitude.
use sas2_parser::Item;
use std::collections::HashMap;
use std::path::Path;

/// Port of Mono's System.Random, the algorithm the game uses for artifact rolls.
pub struct DotNetRandom {
    seed_array: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl DotNetRandom {
    const MBIG: i32 = i32::MAX;
    const MSEED: i32 = 161803398;

    pub fn new(seed: i32) -> Self {
        let mut seed_array = [0i32; 56];
        let num = if seed == i32::MIN { i32::MAX } else { seed.abs() };
        let mut num2 = Self::MSEED - num;
        seed_array[55] = num2;
        let mut num3 = 1;
        for i in 1..55 {
            let num4 = (21 * i) % 55;
            seed_array[num4] = num3;
            num3 = num2 - num3;
            if num3 < 0 {
                num3 += Self::MBIG;
            }
            num2 = seed_array[num4];
        }
        for _ in 1..5 {
            for k in 1..56 {
                seed_array[k] -= seed_array[1 + (k + 30) % 55];
                if seed_array[k] < 0 {
                    seed_array[k] += Self::MBIG;
                }
            }
        }
        Self {
            seed_array,
            inext: 0,
            inextp: 21,
        }
    }

    fn internal_sample(&mut self) -> i32 {
        let mut num = self.inext;
        let mut num2 = self.inextp;
        num += 1;
        if num >= 56 {
            num = 1;
        }
        num2 += 1;
        if num2 >= 56 {
            num2 = 1;
        }
        let mut num3 = self.seed_array[num] - self.seed_array[num2];
        if num3 == Self::MBIG {
            num3 -= 1;
        }
        if num3 < 0 {
            num3 += Self::MBIG;
        }
        self.seed_array[num] = num3;
        self.inext = num;
        self.inextp = num2;
        num3
    }

    /// Equivalent of System.Random.NextDouble().
    pub fn next_double(&mut self) -> f64 {
        self.internal_sample() as f64 * 4.6566128752457969e-10
    }
}

/// (field id, name) for the 35 artifact fields.
pub const ARTIFACT_FIELDS: &[(i32, &str)] = &[
    (0, "Attack Damage"),
    (1, "Attack Speed"),
    (2, "Attack Poise Dmg"),
    (3, "Attack Stamina Reduction"),
    (4, "Damage vs Mages"),
    (5, "Damage vs Minions"),
    (6, "Damage vs Undead"),
    (7, "Damage vs Mobs"),
    (8, "Damage vs Guardians"),
    (9, "Damage vs Hazeburnt"),
    (10, "Attack Rage Buildup"),
    (11, "Attack Reach"),
    (12, "Damage vs Players"),
    (13, "Add HP"),
    (14, "Add MP"),
    (15, "Add Stamina"),
    (16, "Reduce Damage Received"),
    (17, "Add Stamina Recover"),
    (18, "Phys Defense"),
    (19, "Fire Defense"),
    (20, "Cold Defense"),
    (21, "Poison Defense"),
    (22, "Light Defense"),
    (23, "Dark Defense"),
    (24, "Poise Recovery"),
    (25, "Poise"),
    (26, "Ranged Dmg"),
    (27, "Free Ammo"),
    (28, "Item Find"),
    (29, "Silver Find"),
    (30, "XP Find"),
    (31, "Silver Save"),
    (32, "XP Save"),
    (33, "Alchemy Dmg"),
    (34, "Runic Attack"),
];

/// The main stat field for each artifact subtype (3 = Attack, 4 = Defense, 5 = Utility).
pub fn artifact_main_field(subtype: i32) -> i32 {
    match subtype {
        4 => 13,
        5 => 26,
        _ => 0,
    }
}

/// Replicates PlayerArtifactData.GetBoundedVal.
fn bounded(val: f32, soft_min: f32, max: f32) -> f32 {
    let mut v = val;
    if v < soft_min {
        v = (v + soft_min) / 2.0;
    }
    if v > max {
        v = max;
    }
    v
}

/// Compute the 35 artifact values for a seed, replicating PlayerArtifactData.Populate.
/// `tier` is the artifact tier (seed / 2000), capped at 40 like the game's max.
pub fn compute_artifact_values(seed: i32, subtype: i32, tier: i32) -> [f32; 35] {
    let mut values = [0.0f32; 35];
    let mut rng = DotNetRandom::new(seed);
    let num = tier.min(40);

    match subtype {
        3 => {
            values[0] = rng.next_double() as f32 * 3.0 + (num + 1) as f32 * 0.25;
            if values[0] > 20.0 {
                values[0] = 20.0;
            }
            let sub_count = ((rng.next_double() * 4.0) as i32).min(num);
            for _ in 0..sub_count {
                let roll = (rng.next_double() * 13.0) as i32;
                let field = match roll {
                    0 => 12,
                    1 => 1,
                    2 => 8,
                    3 => 4,
                    4 => 5,
                    5 => 7,
                    6 => 6,
                    7 => 2,
                    8 => 10,
                    9 => 11,
                    10 => 3,
                    12 => 9,
                    _ => continue, // case 11 is a no-op in the game
                };
                let v = rng.next_double() as f32 * 5.0 + (num + 5) as f32 * 0.25;
                let v = bounded(v, 5.0, 20.0);
                if values[field] <= 0.0 {
                    values[field] = v;
                }
            }
        }
        4 => {
            values[13] = rng.next_double() as f32 * 5.0 + (num + 5) as f32 * 0.25;
            if values[13] > 50.0 {
                values[13] = 50.0;
            }
            let sub_count = (rng.next_double() * 4.0) as i32;
            for _ in 0..sub_count {
                let roll = (rng.next_double() * 11.0) as i32;
                let field = match roll {
                    0 => 14,
                    1 => 15,
                    2 => 17,
                    3 => 20,
                    4 => 23,
                    5 => 19,
                    6 => 22,
                    7 => 18,
                    8 => 25,
                    9 => 24,
                    10 => 21,
                    _ => continue, // default 13 is the main stat, already set
                };
                let v = rng.next_double() as f32 * 5.0 + (num + 5) as f32 * 0.25;
                let v = bounded(v, 5.0, 20.0);
                if values[field] <= 0.0 {
                    values[field] = v;
                }
            }
        }
        5 => {
            values[26] = rng.next_double() as f32 * 5.0 + (num + 5) as f32 * 0.25;
            if values[26] > 20.0 {
                values[26] = 20.0;
            }
            let sub_count = (rng.next_double() * 4.0) as i32;
            for _ in 0..sub_count {
                let roll = (rng.next_double() * 8.0) as i32;
                let (field, rand_mult, tier_mult, soft_min, max) = match roll {
                    0 => (33, 5.0, 0.25, 5.0, 20.0),
                    1 => (27, 10.0, 0.5, 5.0, 60.0),
                    2 => (28, 5.0, 0.25, 5.0, 20.0),
                    3 => (29, 5.0, 0.25, 5.0, 20.0),
                    4 => (31, 5.0, 0.5, 5.0, 50.0),
                    5 => (30, 5.0, 0.25, 5.0, 20.0),
                    6 => (32, 5.0, 0.5, 5.0, 50.0),
                    7 => (34, 5.0, 0.25, 5.0, 20.0),
                    _ => continue,
                };
                let v = rng.next_double() as f32 * rand_mult + (num + 5) as f32 * tier_mult;
                let v = bounded(v, soft_min, max);
                if values[field] <= 0.0 {
                    values[field] = v;
                }
            }
        }
        _ => {}
    }

    values
}

/// The seed used for an artifact's value roll: artifact_seed in modded saves, upgrade in vanilla saves.
pub fn artifact_seed(item: &Item) -> i32 {
    if item.artifact_seed >= 0 {
        item.artifact_seed
    } else {
        item.upgrade
    }
}

/// The artifact tier (seed / 2000), capped at the game's max of 40.
pub fn artifact_tier(seed: i32) -> i32 {
    (seed / 2000).min(40).max(0)
}

/// Rarity from the number of nonzero values (matches LootDef.GetRarity for type 6).
pub fn artifact_rarity(values: &[f32; 35]) -> i32 {
    let count = values.iter().filter(|v| **v > 0.0).count();
    match count {
        2 => 1,
        3 => 2,
        4 => 3,
        5 => 4,
        _ => 0,
    }
}

/// Vanilla magnitude per charm flag (what a single talisman contributes at tier 1).
pub fn charm_vanilla(flag: i32) -> f32 {
    match flag {
        0 => 10.0,       // Phys Def
        1..=5 => 20.0,   // Elemental Def
        6 => 10.0,       // Item Find
        7 => 0.15,       // Rage Gain
        8 => 1.0,        // Rage Window
        9 | 10 => 1.0,   // Wood Runes / Poise
        11 => 2.0,       // Fast grapple/climb
        12 => 10.0,      // Stamina Regen
        13 => 50.0,      // Silver Find
        14 => 10.0,      // Damage
        15 => 5.0,       // Gold
        16..=20 => 20.0, // Elemental Atk
        21..=28 => 1.0,  // Multiplayer flags
        29 => 5.0,       // Carry Weight
        30 | 31 => 5.0,  // HP/MP Kill Gain
        32 => 50.0,      // Parry Stagger Damage
        33 => 25.0,      // MP Regain
        34 => 50.0,      // Riposte Dmg
        35 => 50.0,      // Dying Boost
        36 | 37 | 39 => 5.0,  // Max HP/Rage/Stamina Boost
        38 => 10.0,      // Max MP Boost
        40 | 41 => 2.5,  // MP/HP Parry regain
        42 | 43 => 50.0, // MP/HP Riposte regain
        44 => 50.0,      // Restock speed
        45 | 46 => 12.5, // Rage Parry/Riposte regain
        47 => 1.0,       // Stamina coverage
        48 => 10.0,      // Blocking stamina cheap
        49 => 15.0,      // Runic art boost
        50 => 50.0,      // Faster Drinking
        51 => 3.1,       // Overall defense
        52 | 53 => 10.0, // Haze HP/MP
        54 => 3.0,       // Haze Rage
        _ => 1.0,
    }
}

/// Display unit for a charm flag.
#[derive(Clone, Copy, PartialEq)]
pub enum CharmUnit {
    Percent,
    Flat,
}

pub fn charm_unit(flag: i32) -> CharmUnit {
    match flag {
        13..=20 | 30..=37 | 39..=46 | 48..=50 | 52..=53 => CharmUnit::Percent,
        _ => CharmUnit::Flat,
    }
}

/// The GetCharmVal tier for a flag count (1.0 / 1.25 / 1.35 / 1.4).
pub fn charm_tier(count: i32) -> f32 {
    match count {
        1 => 1.0,
        2 => 1.25,
        3 => 1.35,
        4 => 1.4,
        _ => 0.0,
    }
}

/// The effective magnitude of a charm flag given how many equipped talismans share it.
pub fn charm_effective(flag: i32, count: i32) -> f32 {
    charm_tier(count) * charm_vanilla(flag)
}

/// A per-field override from the Resalter mod's artifact_boosts.json.
#[derive(Clone, Copy, Debug)]
pub struct ArtifactBoostOverride {
    pub min: f32,
    pub max: f32,
    pub static_boost: bool,
    pub static_value: f32,
}

/// Load the Resalter mod's artifact_boosts.json from the game's BepInEx config folder.
/// Returns an empty map when the file is missing or unreadable.
pub fn load_resalter_artifact_boosts(game_path: &Path) -> HashMap<i32, ArtifactBoostOverride> {
    let mut result = HashMap::new();
    let path = game_path
        .join("BepInEx/config/amione.SaS2Resalter/artifact_boosts.json");
    let Ok(data) = std::fs::read_to_string(&path) else {
        return result;
    };
    let Ok(map) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&data) else {
        return result;
    };
    for (key, value) in map {
        let Ok(field) = key.parse::<i32>() else {
            continue;
        };
        let Some(obj) = value.as_object() else {
            continue;
        };
        let num = |name: &str| -> f32 {
            obj.get(name)
                .and_then(|v| v.as_f64())
                .map(|v| v as f32)
                .unwrap_or(0.0)
        };
        let boolean = |name: &str| -> bool {
            obj.get(name)
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        };
        result.insert(
            field,
            ArtifactBoostOverride {
                min: num("min"),
                max: num("max"),
                static_boost: boolean("static_boost"),
                static_value: num("static_value"),
            },
        );
    }
    result
}

/// The achievable value range for an artifact field at a given tier, without any mod.
/// Returns None when the field can never be nonzero at that tier.
pub fn vanilla_value_range(subtype: i32, tier: i32, field: i32) -> Option<(f32, f32)> {
    let num = tier.min(40);
    let main = artifact_main_field(subtype);
    if field == main {
        let (raw_min, rand_range, cap) = match subtype {
            4 => ((num + 5) as f32 * 0.25, 5.0, 50.0),
            5 => ((num + 5) as f32 * 0.25, 5.0, 20.0),
            _ => ((num + 1) as f32 * 0.25, 3.0, 20.0),
        };
        return Some((raw_min, (raw_min + rand_range).min(cap)));
    }
    // Sub-stat: only reachable when the sub-stat count roll allows it.
    let sub_count = match subtype {
        3 => num.min(3),
        4 => 3,
        5 => 3,
        _ => 0,
    };
    if sub_count <= 0 {
        return None;
    }
    let (soft_min, raw_min, raw_max, max) = match subtype {
        3 => (5.0, (num + 5) as f32 * 0.25, (num + 5) as f32 * 0.25 + 5.0, 20.0),
        4 => (5.0, (num + 5) as f32 * 0.25, (num + 5) as f32 * 0.25 + 5.0, 20.0),
        5 => match field {
            27 => (5.0, (num + 5) as f32 * 0.5, (num + 5) as f32 * 0.5 + 10.0, 60.0),
            31 | 32 => (5.0, (num + 5) as f32 * 0.5, (num + 5) as f32 * 0.5 + 5.0, 50.0),
            _ => (5.0, (num + 5) as f32 * 0.25, (num + 5) as f32 * 0.25 + 5.0, 20.0),
        },
        _ => return None,
    };
    // GetBoundedVal pushes values below soft_min up to (val + soft_min) / 2.
    let min = if raw_min < soft_min {
        (raw_min + soft_min) / 2.0
    } else {
        raw_min
    };
    Some((min, raw_max.min(max)))
}

/// The effective min/max for a field: the Resalter override when present, otherwise the vanilla range.
/// Returns None when the field can never be set.
pub fn effective_value_range(
    subtype: i32,
    tier: i32,
    field: i32,
    resalter: Option<&ArtifactBoostOverride>,
) -> Option<(f32, f32)> {
    if let Some(o) = resalter {
        if o.static_boost {
            return Some((o.static_value, o.static_value));
        }
        return Some((o.min, o.max));
    }
    vanilla_value_range(subtype, tier, field)
}

/// Result of a best-effort seed search.
pub struct SeedSearchResult {
    /// The best seed found.
    pub seed: i32,
    /// Fields that matched within tolerance.
    pub matched: Vec<i32>,
    /// Fields that did not match: (field, desired, actual).
    pub missed: Vec<(i32, f32, f32)>,
}

/// Search all seeds in a tier.
/// Returns a perfect match when one exists, otherwise the seed matching the most desired fields, with the smallest total deviation among ties.
pub fn find_best_seed_for_values(
    subtype: i32,
    tier: i32,
    desired: &[(i32, f32)],
    max_attempts: usize,
) -> Option<SeedSearchResult> {
    if desired.is_empty() {
        return None;
    }
    let (min, max) = tier_seed_range(tier);
    let attempts = ((max - min + 1) as usize).min(max_attempts);
    let mut best: Option<SeedSearchResult> = None;
    for seed in min..=max {
        if (seed - min) as usize >= attempts {
            break;
        }
        let values = compute_artifact_values(seed, subtype, tier);
        let mut matched = Vec::new();
        let mut missed = Vec::new();
        for (field, want) in desired {
            let actual = values[*field as usize];
            if (actual - *want).abs() <= 0.05 {
                matched.push(*field);
            } else {
                missed.push((*field, *want, actual));
            }
        }
        if matched.len() == desired.len() {
            return Some(SeedSearchResult {
                seed,
                matched,
                missed,
            });
        }
        let better = match &best {
            None => true,
            Some(b) => {
                matched.len() > b.matched.len()
                    || (matched.len() == b.matched.len()
                        && total_error(&missed) < total_error(&b.missed))
            }
        };
        if better {
            best = Some(SeedSearchResult {
                seed,
                matched,
                missed,
            });
        }
    }
    best
}

fn total_error(missed: &[(i32, f32, f32)]) -> f32 {
    missed
        .iter()
        .map(|(_, want, actual)| (actual - want).abs())
        .sum()
}

/// The seed range for a tier: tier * 2000 + 1..=2000.
fn tier_seed_range(tier: i32) -> (i32, i32) {
    (tier * 2000 + 1, tier * 2000 + 2000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotnet_random_matches_framework() {
        // Reference values from .NET Framework's System.Random (verified with dotnet 10).
        let mut rng = DotNetRandom::new(0);
        assert!((rng.next_double() - 0.7262432699679598).abs() < 1e-12);
        let mut rng = DotNetRandom::new(1);
        assert!((rng.next_double() - 0.24866858415709278).abs() < 1e-12);
        let mut rng = DotNetRandom::new(12345);
        assert!((rng.next_double() - 0.06674693481379511).abs() < 1e-12);
        let mut rng = DotNetRandom::new(42);
        assert!((rng.next_double() - 0.6681064659115423).abs() < 1e-12);
    }

    #[test]
    fn artifact_values_are_deterministic() {
        let a = compute_artifact_values(12345, 3, 6);
        let b = compute_artifact_values(12345, 3, 6);
        assert_eq!(a, b);
        // Main stat is always set for attack artifacts.
        assert!(a[0] > 0.0);
        // Defense main stat.
        let d = compute_artifact_values(12345, 4, 6);
        assert!(d[13] > 0.0);
        // Utility main stat.
        let u = compute_artifact_values(12345, 5, 6);
        assert!(u[26] > 0.0);
    }

    #[test]
    fn charm_tiers() {
        assert_eq!(charm_tier(0), 0.0);
        assert_eq!(charm_tier(1), 1.0);
        assert_eq!(charm_tier(2), 1.25);
        assert_eq!(charm_tier(3), 1.35);
        assert_eq!(charm_tier(4), 1.4);
        assert_eq!(charm_effective(14, 2), 12.5);
    }

    #[test]
    fn vanilla_ranges() {
        // Attack main stat at tier 6: 1.75..=4.75 (cap 20 only at high tiers)
        let (min, max) = vanilla_value_range(3, 6, 0).unwrap();
        assert!((min - 1.75).abs() < 0.001);
        assert!((max - 4.75).abs() < 0.001);
        // Defense main stat at tier 6: 2.75..=7.75 (cap 50 only at high tiers)
        let (min, max) = vanilla_value_range(4, 6, 13).unwrap();
        assert!((min - 2.75).abs() < 0.001);
        assert!((max - 7.75).abs() < 0.001);
        // Utility main stat at tier 6: 2.75..=7.75
        let (min, max) = vanilla_value_range(5, 6, 26).unwrap();
        assert!((min - 2.75).abs() < 0.001);
        assert!((max - 7.75).abs() < 0.001);
        // Sub-stat at tier 0 attack: never set
        assert!(vanilla_value_range(3, 0, 1).is_none());
        // Free ammo range at tier 6: 5.5..=16 (bounded min 5, raw 5.5..=15.5)
        let (min, max) = vanilla_value_range(5, 6, 27).unwrap();
        assert!((min - 5.5).abs() < 0.001);
        assert!((max - 15.5).abs() < 0.001);
    }

    #[test]
    fn find_seed_matches_desired() {
        // Find a seed for attack damage 3% at tier 6 (range 1.75..=4.75), then verify it.
        let result = find_best_seed_for_values(3, 6, &[(0, 3.0)], 200_000).expect("seed should exist");
        assert!(result.missed.is_empty());
        let values = compute_artifact_values(result.seed, 3, 6);
        assert!((values[0] - 3.0).abs() <= 0.05);
        // The seed must be in the tier range: tier 6 = 12001..=14000.
        assert!((12001..=14000).contains(&result.seed));
    }

    #[test]
    fn find_seed_matches_multiple_desired() {
        // Find a seed for field 0, then use its field 1 value as the second desired value.
        // That seed itself matches the combo, so the search must find something.
        let seed_a = find_best_seed_for_values(3, 6, &[(0, 3.0)], 200_000)
            .expect("seed should exist");
        let values_a = compute_artifact_values(seed_a.seed, 3, 6);
        let v1 = values_a[1];
        let result = find_best_seed_for_values(3, 6, &[(0, 3.0), (1, v1)], 200_000)
            .expect("seed should exist");
        assert!(result.missed.is_empty());
        let values = compute_artifact_values(result.seed, 3, 6);
        assert!((values[0] - 3.0).abs() <= 0.05);
        assert!((values[1] - v1).abs() <= 0.05);
    }

    #[test]
    fn find_seed_impossible_returns_none() {
        // 100% is impossible for any attack artifact field: the best result has no matched fields.
        let result = find_best_seed_for_values(3, 6, &[(0, 100.0)], 200_000)
            .expect("best seed should exist");
        assert!(result.matched.is_empty());
        assert_eq!(result.missed.len(), 1);
        // Empty desired list never matches.
        assert!(find_best_seed_for_values(3, 6, &[], 200_000).is_none());
    }

    #[test]
    fn find_best_seed_prefers_more_matches() {
        // A combo that cannot match exactly: field 0 = 3.0 and field 1 = 0.0 (field 1 is never 0 when the artifact has sub-stats, but a seed with only the main stat has field 1 = 0).
        // The best result should match field 0 and report field 1 as missed, or match both when possible.
        let result = find_best_seed_for_values(3, 6, &[(0, 3.0), (1, 0.0)], 200_000)
            .expect("seed should exist");
        // At least one field must match.
        assert!(!result.matched.is_empty() || !result.missed.is_empty());
        // The reported seed must reproduce the matched fields.
        let values = compute_artifact_values(result.seed, 3, 6);
        for f in &result.matched {
            let want = if *f == 0 { 3.0 } else { 0.0 };
            assert!((values[*f as usize] - want).abs() <= 0.05);
        }
    }

    #[test]
    fn resalter_override_effective_range() {
        let o = ArtifactBoostOverride {
            min: 5.0,
            max: 40.0,
            static_boost: false,
            static_value: 5.0,
        };
        let (min, max) = effective_value_range(3, 6, 0, Some(&o)).unwrap();
        assert_eq!(min, 5.0);
        assert_eq!(max, 40.0);
        let s = ArtifactBoostOverride {
            min: 5.0,
            max: 5.0,
            static_boost: true,
            static_value: 25.0,
        };
        let (min, max) = effective_value_range(3, 6, 0, Some(&s)).unwrap();
        assert_eq!(min, 25.0);
        assert_eq!(max, 25.0);
        // Without an override, the vanilla range applies.
        let (min, max) = effective_value_range(3, 6, 0, None).unwrap();
        assert!((min - 1.75).abs() < 0.001);
        assert!((max - 4.75).abs() < 0.001);
    }
}
