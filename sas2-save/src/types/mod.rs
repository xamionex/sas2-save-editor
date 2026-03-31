//! Types for working with Salt and Sacrifice save data.

pub mod serializable;
pub mod stats;
pub mod item;
pub mod equipment;
pub mod flags;
pub mod bestiary;
pub mod save_data;
pub mod faction;
pub mod ng_level;

pub use serializable::BinarySerializable;
pub use stats::Stats;
pub use item::Item;
pub use equipment::Equipment;
pub use flags::PlayerFlags;
pub use bestiary::{Bestiary, BestiaryBeast, TOTAL_DROPS};
pub use save_data::SaveData;