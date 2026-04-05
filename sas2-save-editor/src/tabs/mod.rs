pub mod bestiary;
pub mod convert;
pub mod cosmetics;
pub mod equipment;
pub mod faction;
pub mod flags;
pub mod skilltree;
pub mod stats;

#[derive(PartialEq)]
pub enum Tab {
    Stats,
    Equipment,
    Flags,
    Bestiary,
    Cosmetics,
    SkillTree,
    Faction,
    ConvertSave,
}

#[derive(PartialEq)]
pub enum EquipmentSubTab {
    Inventory,
    Stockpile,
    AddItems,
}
