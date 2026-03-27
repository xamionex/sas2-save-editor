use std::path::Path;
use std::fs;
use sas2_save::loot_catalog::LootCatalog;
use sas2_save::monster_catalog::MonsterCatalog;

pub fn load_loot_catalog(game_path: &Path) -> Result<LootCatalog, String> {
    let loot_path = game_path.join("Loot").join("data").join("loot.zls");
    if !loot_path.exists() {
        return Err(format!("loot.zls not found in: {}", loot_path.display()));
    }
    let data = fs::read(&loot_path).map_err(|e| e.to_string())?;
    LootCatalog::load_from_bytes(&data).map_err(|e| e.to_string())
}

pub fn load_monster_catalog(game_path: &Path) -> Result<MonsterCatalog, String> {
    let monsters_path = game_path.join("Monsters").join("data").join("monsters.zms");
    if !monsters_path.exists() {
        return Err(format!("monsters.zms not found in: {}", monsters_path.display()));
    }
    let data = fs::read(&monsters_path).map_err(|e| e.to_string())?;
    MonsterCatalog::load_from_bytes(&data).map_err(|e| e.to_string())
}