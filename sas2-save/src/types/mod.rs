//! Types for working with Salt and Sacrifice save data.

mod serializable;
mod stats;
mod item;
mod equipment;
mod flags;
mod bestiary;
mod save_data;
pub mod faction;

pub use serializable::BinarySerializable;
pub use stats::Stats;
pub use item::Item;
pub use equipment::Equipment;
pub use flags::PlayerFlags;
pub use bestiary::{Bestiary, BestiaryBeast, TOTAL_DROPS};
pub use save_data::SaveData;