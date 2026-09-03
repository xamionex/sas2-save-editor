pub mod artifacts;
pub mod bestiary;
pub mod convert;
pub mod cosmetics;
pub mod equipment;
pub mod faction;
pub mod flags;
pub mod multisel;
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
    Artifacts,
    ConvertSave,
}

#[derive(Clone, PartialEq)]
pub enum EquipmentSubTab {
    Inventory,
    Stockpile,
    AddItems,
}
