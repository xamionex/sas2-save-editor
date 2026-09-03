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
        let num = if seed == i32::MIN {
            i32::MAX
        } else {
            seed.abs()
        };
        let mut num2 = Self::MSEED.wrapping_sub(num);
        seed_array[55] = num2;
        let mut num3 = 1;
        for i in 1..55 {
            let num4 = (21 * i) % 55;
            seed_array[num4] = num3;
            num3 = num2.wrapping_sub(num3);
            if num3 < 0 {
                num3 = num3.wrapping_add(Self::MBIG);
            }
            num2 = seed_array[num4];
        }
        for _ in 1..5 {
            for k in 1..56 {
                seed_array[k] = seed_array[k].wrapping_sub(seed_array[1 + (k + 30) % 55]);
                if seed_array[k] < 0 {
                    seed_array[k] = seed_array[k].wrapping_add(Self::MBIG);
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
        let mut num3 = self.seed_array[num].wrapping_sub(self.seed_array[num2]);
        if num3 == Self::MBIG {
            num3 = num3.wrapping_sub(1);
        }
        if num3 < 0 {
            num3 = num3.wrapping_add(Self::MBIG);
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
        0 => 10.0,           // Phys Def
        1..=5 => 20.0,       // Elemental Def
        6 => 10.0,           // Item Find
        7 => 0.15,           // Rage Gain
        8 => 1.0,            // Rage Window
        9 | 10 => 1.0,       // Wood Runes / Poise
        11 => 2.0,           // Fast grapple/climb
        12 => 10.0,          // Stamina Regen
        13 => 50.0,          // Silver Find
        14 => 10.0,          // Damage
        15 => 5.0,           // Gold
        16..=20 => 20.0,     // Elemental Atk
        21..=28 => 1.0,      // Multiplayer flags
        29 => 5.0,           // Carry Weight
        30 | 31 => 5.0,      // HP/MP Kill Gain
        32 => 50.0,          // Parry Stagger Damage
        33 => 25.0,          // MP Regain
        34 => 50.0,          // Riposte Dmg
        35 => 50.0,          // Dying Boost
        36 | 37 | 39 => 5.0, // Max HP/Rage/Stamina Boost
        38 => 10.0,          // Max MP Boost
        40 | 41 => 2.5,      // MP/HP Parry regain
        42 | 43 => 50.0,     // MP/HP Riposte regain
        44 => 50.0,          // Restock speed
        45 | 46 => 12.5,     // Rage Parry/Riposte regain
        47 => 1.0,           // Stamina coverage
        48 => 10.0,          // Blocking stamina cheap
        49 => 15.0,          // Runic art boost
        50 => 50.0,          // Faster Drinking
        51 => 3.1,           // Overall defense
        52 | 53 => 10.0,     // Haze HP/MP
        54 => 3.0,           // Haze Rage
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
    let path = game_path.join("BepInEx/config/amione.SaS2Resalter/artifact_boosts.json");
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
        let boolean =
            |name: &str| -> bool { obj.get(name).and_then(|v| v.as_bool()).unwrap_or(false) };
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
/// Returns None when the field can never be nonzero at that tier
/// (wrong subtype or too few sub-stat rolls).
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
    // Sub-stat: only fields the subtype can actually roll are reachable.
    // Attack rolls 1..=12, Defense rolls 14..=25, Utility rolls 27..=34.
    // Fields outside the set are never set, regardless of tier.
    let rollable = match subtype {
        3 => 1..=12,
        4 => 14..=25,
        5 => 27..=34,
        _ => return None,
    };
    if !rollable.contains(&field) {
        return None;
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
        3 => (
            5.0,
            (num + 5) as f32 * 0.25,
            (num + 5) as f32 * 0.25 + 5.0,
            20.0,
        ),
        4 => (
            5.0,
            (num + 5) as f32 * 0.25,
            (num + 5) as f32 * 0.25 + 5.0,
            20.0,
        ),
        5 => match field {
            27 => (
                5.0,
                (num + 5) as f32 * 0.5,
                (num + 5) as f32 * 0.5 + 10.0,
                60.0,
            ),
            31 | 32 => (
                5.0,
                (num + 5) as f32 * 0.5,
                (num + 5) as f32 * 0.5 + 5.0,
                50.0,
            ),
            _ => (
                5.0,
                (num + 5) as f32 * 0.25,
                (num + 5) as f32 * 0.25 + 5.0,
                20.0,
            ),
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

/// The achievable range of a field across every tier in `min_tier..=max_tier` (inclusive), for the "is this desired value possible" checks.
/// A Resalter override is tier-independent and wins over the vanilla ranges.
/// Returns None when the field can never be nonzero in the tier range.
pub fn effective_range_union(
    subtype: i32,
    min_tier: i32,
    max_tier: i32,
    field: i32,
    resalter: Option<&ArtifactBoostOverride>,
) -> Option<(f32, f32)> {
    if let Some(o) = resalter {
        if o.static_boost {
            return Some((o.static_value, o.static_value));
        }
        return Some((o.min, o.max));
    }
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for tier in min_tier..=max_tier {
        let Some((lo, hi)) = vanilla_value_range(subtype, tier, field) else {
            continue;
        };
        min = min.min(lo);
        max = max.max(hi);
    }
    if max < min { None } else { Some((min, max)) }
}

/// A candidate artifact seed whose values contain all the desired fields.
#[derive(Clone)]
pub struct ArtifactMatch {
    /// The seed of the matching artifact.
    pub seed: i32,
    /// The tier the seed belongs to (seed / 2000).
    pub tier: i32,
    /// (field, actual value) for each desired field, in desired order.
    pub values: Vec<(i32, f32)>,
    /// Total absolute deviation from the desired values (0 = exact match).
    pub error: f32,
}

/// The tier scope for an artifact search: the selected artifact's own tier, an explicit min/max tier range, or every tier.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum SearchTierScope {
    #[default]
    StaticTier,
    MinMax,
    AllTiers,
}

/// The sort key of the merged result list: closeness, tier or an artifact field.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum ResultSortKey {
    #[default]
    Closeness,
    Tier,
    Field(i32),
}

/// How the merged result list is grouped: not grouped, by tier, or by an artifact field.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum ResultGroupBy {
    #[default]
    None,
    Tier,
    Field(i32),
}

/// The tier range searched by a scope: (min, max) inclusive.
pub fn search_tier_range(
    scope: SearchTierScope,
    current_tier: i32,
    min_tier: i32,
    max_tier: i32,
) -> (i32, i32) {
    match scope {
        SearchTierScope::StaticTier => (current_tier, current_tier),
        SearchTierScope::MinMax => {
            let min = min_tier.clamp(0, 40);
            let max = max_tier.clamp(min, 40);
            (min, max)
        }
        SearchTierScope::AllTiers => (0, 40),
    }
}

/// Collect the (field, desired) pairs from `desired` that are nonzero.
fn nonzero_desired(desired: &HashMap<i32, f32>) -> Vec<(i32, f32)> {
    let mut result: Vec<(i32, f32)> = desired
        .iter()
        .filter(|(_, d)| **d > 0.0)
        .map(|(f, d)| (*f, *d))
        .collect();
    result.sort_by_key(|(f, _)| *f);
    result
}

/// The result of a search: the exact matches and the partial matches.
pub struct SearchResults {
    pub exact: Vec<ArtifactMatch>,
    pub partial: Vec<ArtifactMatch>,
}

/// Collect the actual values of `fields` that are nonzero on an artifact.
fn collect_values(values: &[f32; 35], fields: &[i32]) -> Vec<(i32, f32)> {
    let mut vals = Vec::with_capacity(fields.len());
    for f in fields {
        let v = values[*f as usize];
        if v > 0.0 {
            vals.push((*f, v));
        }
    }
    vals
}

/// Find the exact and partial matches for the combined must/can filters.
///
/// Exact matches: every must field within 0.05 of its desired value, and every can field present (nonzero).
/// With no must filters set, nothing is exact.
/// Partial matches: every can field present, excluding seeds already listed as exact matches.
/// With no can filters set, nothing is partial.
/// Every match carries the actual values of all filtered fields that are present on the artifact (the union of the must and can fields).
pub fn find_matches(
    subtype: i32,
    min_tier: i32,
    max_tier: i32,
    must: &HashMap<i32, f32>,
    can: &HashMap<i32, f32>,
) -> SearchResults {
    let must = nonzero_desired(must);
    let can = nonzero_desired(can);
    if must.is_empty() && can.is_empty() {
        return SearchResults {
            exact: Vec::new(),
            partial: Vec::new(),
        };
    }
    // Union of the filter fields, sorted: the display columns of the result list.
    let mut fields: Vec<i32> = must
        .iter()
        .map(|(f, _)| *f)
        .chain(can.iter().map(|(f, _)| *f))
        .collect();
    fields.sort_unstable();
    fields.dedup();
    let mut exact = Vec::new();
    let mut partial = Vec::new();
    for tier in min_tier..=max_tier {
        let (min, max) = tier_seed_range(tier);
        for seed in min..=max {
            let values = compute_artifact_values(seed, subtype, tier);
            // Every can field must be present for either list.
            let mut can_ok = true;
            let mut can_error = 0.0;
            for (field, want) in &can {
                let actual = values[*field as usize];
                if actual <= 0.0 {
                    can_ok = false;
                    break;
                }
                can_error += (actual - want).abs();
            }
            if !can_ok {
                continue;
            }
            // Must fields, when set, must all be within 0.05 for an exact match.
            let mut must_ok = true;
            let mut must_error = 0.0;
            for (field, want) in &must {
                let actual = values[*field as usize];
                if actual <= 0.0 || (actual - want).abs() > 0.05 {
                    must_ok = false;
                    break;
                }
                must_error += (actual - want).abs();
            }
            if !must.is_empty() && must_ok {
                exact.push(ArtifactMatch {
                    seed,
                    tier,
                    values: collect_values(&values, &fields),
                    error: must_error,
                });
            } else if !can.is_empty() {
                partial.push(ArtifactMatch {
                    seed,
                    tier,
                    values: collect_values(&values, &fields),
                    error: can_error,
                });
            }
        }
    }
    exact.sort_by(|a, b| a.error.total_cmp(&b.error));
    partial.sort_by(|a, b| a.error.total_cmp(&b.error));
    SearchResults { exact, partial }
}

/// Remap a seed into another tier, preserving the within-tier offset.
/// Seed ranges are tier * 2000 + 1..=tier * 2000 + 2000.
pub fn seed_for_tier(seed: i32, tier: i32) -> i32 {
    let offset = seed % 2000;
    let offset = if offset == 0 { 2000 } else { offset };
    tier * 2000 + offset
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
    fn dotnet_random_handles_extreme_seeds() {
        // Large seeds overflow the internal int arithmetic, the port must wrap like .NET's unchecked math instead of panicking.
        // Reference values verified with dotnet 10.
        let mut rng = DotNetRandom::new(i32::MAX);
        assert!((rng.next_double() - 0.72624326996795985).abs() < 1e-12);
        let mut rng = DotNetRandom::new(i32::MAX - 1);
        assert!((rng.next_double() - 0.20381795577882694).abs() < 1e-12);
        let mut rng = DotNetRandom::new(2_000_000_000);
        assert!((rng.next_double() - 0.10450909990095957).abs() < 1e-12);
        let mut rng = DotNetRandom::new(i32::MIN);
        assert!((rng.next_double() - 0.72624326996795985).abs() < 1e-12);
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
    fn vanilla_ranges_respect_subtype() {
        // Attack artifacts can never roll defense/utility fields.
        assert!(vanilla_value_range(3, 40, 13).is_none());
        assert!(vanilla_value_range(3, 40, 27).is_none());
        // Defense artifacts can never roll attack/utility fields.
        assert!(vanilla_value_range(4, 40, 0).is_none());
        assert!(vanilla_value_range(4, 40, 1).is_none());
        assert!(vanilla_value_range(4, 40, 4).is_none());
        assert!(vanilla_value_range(4, 40, 5).is_none());
        assert!(vanilla_value_range(4, 40, 27).is_none());
        // Utility artifacts can never roll attack/defense fields.
        assert!(vanilla_value_range(5, 40, 0).is_none());
        assert!(vanilla_value_range(5, 40, 13).is_none());
        // But their own rollable sub-stats are reachable at tier >= 1.
        assert!(vanilla_value_range(3, 40, 1).is_some());
        assert!(vanilla_value_range(3, 40, 12).is_some());
        assert!(vanilla_value_range(4, 40, 14).is_some());
        assert!(vanilla_value_range(4, 40, 25).is_some());
        assert!(vanilla_value_range(5, 40, 27).is_some());
        assert!(vanilla_value_range(5, 40, 34).is_some());
    }

    #[test]
    fn find_matches_must_only_and_can_only() {
        // Must-only: exact matches require the must fields within 0.05.
        let must = HashMap::from([(0, 3.0)]);
        let res = find_matches(3, 6, 6, &must, &HashMap::new());
        assert!(!res.exact.is_empty());
        assert!(res.partial.is_empty(), "no can filters -> nothing partial");
        for m in &res.exact {
            assert!((12001..=14000).contains(&m.seed));
            let values = compute_artifact_values(m.seed, 3, 6);
            assert!((values[0] - 3.0).abs() <= 0.05);
        }
        // Sorted by error ascending: the first is the closest to 3.0.
        assert!(res.exact.first().unwrap().error <= res.exact.last().unwrap().error);

        // Can-only: every can-present seed is a partial match, nothing exact.
        let can = HashMap::from([(0, 3.0)]);
        let res = find_matches(3, 6, 6, &HashMap::new(), &can);
        assert!(res.exact.is_empty(), "no must filters -> nothing exact");
        assert!(!res.partial.is_empty());
        for m in &res.partial {
            assert!((12001..=14000).contains(&m.seed));
            let values = compute_artifact_values(m.seed, 3, 6);
            assert!(values[0] > 0.0);
        }
        // Sorted by error ascending.
        assert!(res.partial.first().unwrap().error <= res.partial.last().unwrap().error);
    }

    #[test]
    fn find_matches_combined() {
        // With both filters, a seed matching both is exact; seeds with the can field but not the exact must value are partial, never repeated.
        let must = HashMap::from([(0, 3.0)]);
        let res = find_matches(3, 6, 6, &must, &HashMap::new());
        let seed_a = res
            .exact
            .iter()
            .find(|m| compute_artifact_values(m.seed, 3, 6)[1] > 0.0)
            .expect("a match with a sub-stat should exist")
            .seed;
        let v1 = compute_artifact_values(seed_a, 3, 6)[1];
        let must2 = HashMap::from([(1, v1)]);
        let can = HashMap::from([(0, 3.0)]);
        let res2 = find_matches(3, 6, 6, &must2, &can);
        assert!(
            res2.exact.iter().any(|m| m.seed == seed_a),
            "a seed matching both filters must be an exact match"
        );
        // Every exact match has the can field present and the must field close.
        for m in &res2.exact {
            let values = compute_artifact_values(m.seed, 3, 6);
            assert!(values[0] > 0.0);
            assert!((values[1] - v1).abs() <= 0.05);
        }
        // Seeds with the can field but not the exact must field go to partial.
        for m in &res2.partial {
            assert!(
                !res2.exact.iter().any(|e| e.seed == m.seed),
                "partial must not repeat exact seeds"
            );
            let values = compute_artifact_values(m.seed, 3, 6);
            assert!(values[0] > 0.0);
            assert!((values[1] - v1).abs() > 0.05);
        }
    }

    #[test]
    fn find_matches_empty_and_impossible() {
        // Neither filter set: both lists empty.
        let res = find_matches(3, 6, 6, &HashMap::new(), &HashMap::new());
        assert!(res.exact.is_empty());
        assert!(res.partial.is_empty());
        // An impossible must field (never rollable by the subtype) blocks the
        // exact list; can-only seeds still appear as partial matches.
        let can = HashMap::from([(0, 3.0)]);
        let impossible = HashMap::from([(13, 7.0)]);
        let res = find_matches(3, 6, 6, &impossible, &can);
        assert!(res.exact.is_empty());
        assert!(!res.partial.is_empty());
        for m in &res.partial {
            let values = compute_artifact_values(m.seed, 3, 6);
            assert!(values[0] > 0.0);
        }
        // An impossible must field alone yields nothing at all.
        let res = find_matches(3, 6, 6, &impossible, &HashMap::new());
        assert!(res.exact.is_empty());
        assert!(res.partial.is_empty());
    }

    #[test]
    fn find_matches_all_tiers_works() {
        // Across all tiers, can-only partial matches come from many tiers.
        let can = HashMap::from([(0, 3.0)]);
        let res = find_matches(3, 0, 40, &HashMap::new(), &can);
        assert!(res.exact.is_empty());
        assert!(!res.partial.is_empty());
        // Each match carries its correct tier and seed range.
        for m in &res.partial {
            // Seed ranges are tier * 2000 + 1..=tier * 2000 + 2000.
            assert_eq!(m.tier, (m.seed - 1) / 2000);
            assert!(m.tier <= 40);
        }
        // Sorted by error ascending.
        assert!(res.partial.first().unwrap().error <= res.partial.last().unwrap().error);
    }

    #[test]
    fn search_tier_range_scopes() {
        assert_eq!(
            search_tier_range(SearchTierScope::StaticTier, 6, 2, 10),
            (6, 6)
        );
        assert_eq!(
            search_tier_range(SearchTierScope::AllTiers, 6, 2, 10),
            (0, 40)
        );
        assert_eq!(
            search_tier_range(SearchTierScope::MinMax, 6, 2, 10),
            (2, 10)
        );
        // Min/max are clamped into 0..=40 and min <= max.
        assert_eq!(
            search_tier_range(SearchTierScope::MinMax, 6, -5, 99),
            (0, 40)
        );
        assert_eq!(
            search_tier_range(SearchTierScope::MinMax, 6, 12, 7),
            (12, 12)
        );
    }

    #[test]
    fn effective_range_union_spans_tiers() {
        // Attack damage at tier 6 alone: 1.75..=4.75.
        let (min, max) = effective_range_union(3, 6, 6, 0, None).unwrap();
        assert!((min - 1.75).abs() < 0.001);
        assert!((max - 4.75).abs() < 0.001);
        // Across tiers 0..=40 the union is wider than any single tier.
        // Attack main stat maxes at tier 40: 10.25 + 3.0 = 13.25 (cap 20 unreachable).
        let (min, max) = effective_range_union(3, 0, 40, 0, None).unwrap();
        assert!((min - 0.25).abs() < 0.001);
        assert!((max - 13.25).abs() < 0.001);
        // Sub-stats are unreachable at tier 0 attack.
        assert!(effective_range_union(3, 0, 0, 1, None).is_none());
        assert!(effective_range_union(3, 0, 40, 1, None).is_some());
        // Fields the subtype can never roll are never reachable.
        assert!(effective_range_union(3, 0, 40, 13, None).is_none());
        // A Resalter override wins regardless of the tier range.
        let o = ArtifactBoostOverride {
            min: 5.0,
            max: 40.0,
            static_boost: false,
            static_value: 5.0,
        };
        let (min, max) = effective_range_union(3, 0, 40, 0, Some(&o)).unwrap();
        assert_eq!(min, 5.0);
        assert_eq!(max, 40.0);
    }

    #[test]
    fn seed_for_tier_preserves_offset() {
        assert_eq!(seed_for_tier(1234, 0), 1234);
        assert_eq!(seed_for_tier(1234, 5), 5 * 2000 + 1234);
        // Seed exactly at a tier boundary (2000) maps to offset 2000.
        assert_eq!(seed_for_tier(2000, 1), 1 * 2000 + 2000);
        assert_eq!(seed_for_tier(6000, 2), 2 * 2000 + 2000);
    }

    #[test]
    fn resalter_override_range_union() {
        let o = ArtifactBoostOverride {
            min: 5.0,
            max: 40.0,
            static_boost: false,
            static_value: 5.0,
        };
        let (min, max) = effective_range_union(3, 0, 40, 0, Some(&o)).unwrap();
        assert_eq!(min, 5.0);
        assert_eq!(max, 40.0);
        let s = ArtifactBoostOverride {
            min: 5.0,
            max: 5.0,
            static_boost: true,
            static_value: 25.0,
        };
        let (min, max) = effective_range_union(3, 0, 40, 0, Some(&s)).unwrap();
        assert_eq!(min, 25.0);
        assert_eq!(max, 25.0);
        // Without an override, the single-tier union matches the vanilla range.
        let (min, max) = effective_range_union(3, 6, 6, 0, None).unwrap();
        assert!((min - 1.75).abs() < 0.001);
        assert!((max - 4.75).abs() < 0.001);
    }
}
