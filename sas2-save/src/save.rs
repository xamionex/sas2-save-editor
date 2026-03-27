use crate::utils::{read_string, write_string, xor_data, SaveError};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use md5;
use std::io::{Cursor, Read, Write};

// -----------------------------------------------------------------------------
// Data structures (temporarily defined here; can be split into separate files)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub level: i32,
    pub stats: [i32; 9],
    pub xp: i64,
    pub silver: i64,
    pub dropped_xp: i64,
    pub dropped_xp_area: i32,
    pub dropped_xp_vec: (f32, f32),
    pub time_played: f64,
    pub hazeburnt: bool,
    pub item_class: [i32; 40],
    pub tree_unlocks: [i32; 500],
    pub class_unlocks: [i32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub loot_idx: i32,
    pub count: i32,
    pub upgrade: i32,
    pub stock_piled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Equipment {
    pub inventory_items: Vec<Item>,
    pub equipped_items: [i32; 31], // TOTAL_EQUIPMENT_SLOTS
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerFlags {
    pub flags: Vec<String>,
    pub bounty_seed: i32,
    pub bounties_complete: i32,
    pub ng_level: i32,
}

pub const TOTAL_DROPS: usize = 5;

#[derive(Debug, Clone, PartialEq)]
pub struct BestiaryBeast {
    pub kills: i32,
    pub deaths: i32,
    pub drops: [bool; TOTAL_DROPS],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bestiary {
    pub beasts: Vec<BestiaryBeast>,
}

// -----------------------------------------------------------------------------
// BinarySerializable trait
// -----------------------------------------------------------------------------

pub trait BinarySerializable: Sized {
    fn read<R: Read>(reader: &mut R) -> Result<Self, SaveError>;
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), SaveError>;
}

// -----------------------------------------------------------------------------
// Implementations
// -----------------------------------------------------------------------------

impl BinarySerializable for Stats {
    fn read<R: Read>(reader: &mut R) -> Result<Self, SaveError> {
        let level = reader.read_i32::<LittleEndian>()?;
        let mut stats = [0; 9];
        for s in &mut stats {
            *s = reader.read_i32::<LittleEndian>()?;
        }
        let xp = reader.read_i64::<LittleEndian>()?;
        let silver = reader.read_i64::<LittleEndian>()?;
        let dropped_xp = reader.read_i64::<LittleEndian>()?;
        let dropped_xp_area = reader.read_i32::<LittleEndian>()?;
        let dx = reader.read_f32::<LittleEndian>()?;
        let dy = reader.read_f32::<LittleEndian>()?;
        let time_played = reader.read_f64::<LittleEndian>()?;
        let hazeburnt = reader.read_u8()? != 0;

        let mut item_class = [0; 40];
        for ic in &mut item_class {
            *ic = reader.read_i32::<LittleEndian>()?;
        }
        let mut tree_unlocks = [0; 500];
        for tu in &mut tree_unlocks {
            *tu = reader.read_i32::<LittleEndian>()?;
        }
        let mut class_unlocks = [0; 3];
        for cu in &mut class_unlocks {
            *cu = reader.read_i32::<LittleEndian>()?;
        }

        Ok(Stats {
            level,
            stats,
            xp,
            silver,
            dropped_xp,
            dropped_xp_area,
            dropped_xp_vec: (dx, dy),
            time_played,
            hazeburnt,
            item_class,
            tree_unlocks,
            class_unlocks,
        })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), SaveError> {
        writer.write_i32::<LittleEndian>(self.level)?;
        for s in &self.stats {
            writer.write_i32::<LittleEndian>(*s)?;
        }
        writer.write_i64::<LittleEndian>(self.xp)?;
        writer.write_i64::<LittleEndian>(self.silver)?;
        writer.write_i64::<LittleEndian>(self.dropped_xp)?;
        writer.write_i32::<LittleEndian>(self.dropped_xp_area)?;
        writer.write_f32::<LittleEndian>(self.dropped_xp_vec.0)?;
        writer.write_f32::<LittleEndian>(self.dropped_xp_vec.1)?;
        writer.write_f64::<LittleEndian>(self.time_played)?;
        writer.write_u8(if self.hazeburnt { 1 } else { 0 })?;
        for ic in &self.item_class {
            writer.write_i32::<LittleEndian>(*ic)?;
        }
        for tu in &self.tree_unlocks {
            writer.write_i32::<LittleEndian>(*tu)?;
        }
        for cu in &self.class_unlocks {
            writer.write_i32::<LittleEndian>(*cu)?;
        }
        Ok(())
    }
}

impl BinarySerializable for Item {
    fn read<R: Read>(reader: &mut R) -> Result<Self, SaveError> {
        let loot_idx = reader.read_i32::<LittleEndian>()?;
        let count = reader.read_i32::<LittleEndian>()?;
        let upgrade = reader.read_i32::<LittleEndian>()?;
        let stock_piled = reader.read_u8()? != 0;
        Ok(Item { loot_idx, count, upgrade, stock_piled })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), SaveError> {
        writer.write_i32::<LittleEndian>(self.loot_idx)?;
        writer.write_i32::<LittleEndian>(self.count)?;
        writer.write_i32::<LittleEndian>(self.upgrade)?;
        writer.write_u8(if self.stock_piled { 1 } else { 0 })?;
        Ok(())
    }
}

impl BinarySerializable for Equipment {
    fn read<R: Read>(reader: &mut R) -> Result<Self, SaveError> {
        let inv_count = reader.read_i32::<LittleEndian>()?;
        if inv_count < 0 || inv_count > 100000 {
            return Err(SaveError::InvalidData(format!("Invalid inventory count: {}", inv_count)));
        }
        let mut inventory_items = Vec::with_capacity(inv_count as usize);
        for _ in 0..inv_count {
            inventory_items.push(Item::read(reader)?);
        }

        let mut equipped_items = [-1; 31];
        for slot in &mut equipped_items {
            *slot = reader.read_i32::<LittleEndian>()?;
        }

        Ok(Equipment { inventory_items, equipped_items })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), SaveError> {
        writer.write_i32::<LittleEndian>(self.inventory_items.len() as i32)?;
        for item in &self.inventory_items {
            item.write(writer)?;
        }
        for slot in &self.equipped_items {
            writer.write_i32::<LittleEndian>(*slot)?;
        }
        Ok(())
    }
}

impl BinarySerializable for PlayerFlags {
    fn read<R: Read>(reader: &mut R) -> Result<Self, SaveError> {
        let flag_count = reader.read_i32::<LittleEndian>()?;
        if flag_count < 0 || flag_count > 10000 {
            return Err(SaveError::InvalidData(format!("Invalid flag count: {}", flag_count)));
        }
        let mut flags = Vec::with_capacity(flag_count as usize);
        for _ in 0..flag_count {
            flags.push(read_string(reader)?);
        }

        let bounty_seed = reader.read_i32::<LittleEndian>()?;
        let bounties_complete = reader.read_i32::<LittleEndian>()?;

        // Compute ng_level from flags (simple version)
        let ng_level = flags.iter()
            .filter_map(|f| f.strip_prefix("$&ng_").and_then(|s| s.parse::<i32>().ok()))
            .max()
            .unwrap_or(0);

        Ok(PlayerFlags { flags, bounty_seed, bounties_complete, ng_level })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), SaveError> {
        writer.write_i32::<LittleEndian>(self.flags.len() as i32)?;
        for flag in &self.flags {
            write_string(writer, flag)?;
        }
        writer.write_i32::<LittleEndian>(self.bounty_seed)?;
        writer.write_i32::<LittleEndian>(self.bounties_complete)?;
        Ok(())
    }
}

impl BinarySerializable for BestiaryBeast {
    fn read<R: Read>(reader: &mut R) -> Result<Self, SaveError> {
        let kills = reader.read_i32::<LittleEndian>()?;
        let deaths = reader.read_i32::<LittleEndian>()?;
        let mut drops = [false; TOTAL_DROPS];
        for d in &mut drops {
            *d = reader.read_u8()? != 0;
        }
        Ok(BestiaryBeast { kills, deaths, drops })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), SaveError> {
        writer.write_i32::<LittleEndian>(self.kills)?;
        writer.write_i32::<LittleEndian>(self.deaths)?;
        for d in &self.drops {
            writer.write_u8(if *d { 1 } else { 0 })?;
        }
        Ok(())
    }
}

impl BinarySerializable for Bestiary {
    fn read<R: Read>(reader: &mut R) -> Result<Self, SaveError> {
        let beast_count = reader.read_i32::<LittleEndian>()?;
        // Sanity check to prevent huge allocations on corrupted/modded files
        if beast_count < 0 || beast_count > 10000 {
            return Err(SaveError::InvalidData(format!(
                "Invalid bestiary count: {} (likely corrupted or modded format)",
                beast_count
            )));
        }
        let mut beasts = Vec::with_capacity(beast_count as usize);
        for _ in 0..beast_count {
            beasts.push(BestiaryBeast::read(reader)?);
        }
        Ok(Bestiary { beasts })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), SaveError> {
        writer.write_i32::<LittleEndian>(self.beasts.len() as i32)?;
        for beast in &self.beasts {
            beast.write(writer)?;
        }
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Main SaveData structure
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SaveData {
    pub version: i32,
    pub name: String,
    pub stats: Stats,
    pub equipment: Equipment,
    pub flags: PlayerFlags,
    pub bestiary: Bestiary,
    pub cosmetics: [i32; 11],
    pub hash_data: Option<[u8; 16]>,
}

impl SaveData {
    /// Read a save file from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, SaveError> {
        let mut reader = Cursor::new(data);
        let version = reader.read_i32::<LittleEndian>()?;

        // I failed at supporting saltguard here, if anyone wants to help, feel free to contribute
        // MOD SUPPORT: version 120 is a modded save. XOR key is 19 (same as vanilla version 19).
        let is_mod = version == 120;
        let is_vanilla = version == 18 || version == 19;

        // Determine XOR value: for vanilla 18/19 use the version, for mod 120 use 19, else 0.
        let xor_value = if is_mod { 19 } else if is_vanilla { version } else { 0 };

        let payload_len = data.len() - 4;
        let hash_len = if (version >= 18) && !is_mod { 16 } else { 0 };
        let data_len = payload_len - hash_len;

        let mut data_part = data[4..4 + data_len].to_vec();
        let hash_part = if hash_len > 0 {
            &data[4 + data_len..]
        } else {
            &[]
        };

        if xor_value != 0 {
            xor_data(&mut data_part, xor_value);
        }

        let mut payload_reader = Cursor::new(&data_part);

        let name = read_string(&mut payload_reader)?;
        let stats = Stats::read(&mut payload_reader)?;
        let equipment = Equipment::read(&mut payload_reader)?;
        let flags = PlayerFlags::read(&mut payload_reader)?;

        // Skip 10 ints for versions < 19 (only vanilla)
        if version < 19 && !is_mod {
            for _ in 0..10 {
                payload_reader.read_i32::<LittleEndian>()?;
            }
        }

        let bestiary = Bestiary::read(&mut payload_reader)?;

        let mut cosmetics = [0; 11];
        for c in &mut cosmetics {
            *c = payload_reader.read_i32::<LittleEndian>()?;
        }

        // Check that we've consumed exactly the data part
        let pos = payload_reader.position() as usize;
        if pos != data_len {
            return Err(SaveError::InvalidData(format!(
                "Read {} bytes, expected {}",
                pos, data_len
            )));
        }

        // Verify hash if present (vanilla only)
        if !is_mod && version >= 18 {
            let mut stored_hash = [0; 16];
            stored_hash.copy_from_slice(hash_part);
            let computed_hash = md5::compute(&data_part);
            if computed_hash.0 != stored_hash {
                return Err(SaveError::HashMismatch);
            }
        }

        Ok(SaveData {
            version,
            name,
            stats,
            equipment,
            flags,
            bestiary,
            cosmetics,
            hash_data: if !is_mod && version >= 18 {
                Some([0; 16]) // placeholder, not used
            } else {
                None
            },
        })
    }

    /// Write the save data to raw bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SaveError> {
        let is_mod = self.version == 120;
        let is_vanilla = self.version == 18 || self.version == 19;

        let xor_value = if is_mod { 19 } else if is_vanilla { self.version } else { 0 };

        let mut data_part = Vec::new();

        write_string(&mut data_part, &self.name)?;
        self.stats.write(&mut data_part)?;
        self.equipment.write(&mut data_part)?;
        self.flags.write(&mut data_part)?;
        self.bestiary.write(&mut data_part)?;
        for c in &self.cosmetics {
            data_part.write_i32::<LittleEndian>(*c)?;
        }

        // Compute MD5 only for vanilla (not mod)
        let hash = if !is_mod && self.version >= 18 {
            md5::compute(&data_part)
        } else {
            md5::compute(&[])
        };
        let hash_bytes = hash.0;

        if xor_value != 0 {
            xor_data(&mut data_part, xor_value);
        }

        let mut out = Vec::new();
        out.write_i32::<LittleEndian>(self.version)?;
        out.write_all(&data_part)?;
        if !is_mod && self.version >= 18 {
            out.write_all(&hash_bytes)?;
        }

        Ok(out)
    }
}
