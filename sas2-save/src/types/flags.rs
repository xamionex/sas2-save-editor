use crate::utils::{read_string, write_string, SaveError};
use crate::types::serializable::BinarySerializable;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerFlags {
    pub flags: Vec<String>,
    pub bounty_seed: i32,
    pub bounties_complete: i32,
    pub ng_level: i32,
}

impl BinarySerializable for PlayerFlags {
    fn read<R: Read>(reader: &mut R, _version: i32) -> Result<Self, SaveError> {
        let flag_count = reader.read_i32::<LittleEndian>()?;
        if flag_count < 0 || flag_count > 10000 {
            return Err(SaveError::InvalidData(format!(
                "Invalid flag count: {}",
                flag_count
            )));
        }
        let mut flags = Vec::with_capacity(flag_count as usize);
        for _ in 0..flag_count {
            flags.push(read_string(reader)?);
        }

        let bounty_seed = reader.read_i32::<LittleEndian>()?;
        let bounties_complete = reader.read_i32::<LittleEndian>()?;

        let ng_level = flags
            .iter()
            .filter_map(|f| f.strip_prefix("$&ng_").and_then(|s| s.parse::<i32>().ok()))
            .max()
            .unwrap_or(0);

        Ok(PlayerFlags {
            flags,
            bounty_seed,
            bounties_complete,
            ng_level,
        })
    }

    fn write<W: Write>(&self, writer: &mut W, _version: i32) -> Result<(), SaveError> {
        writer.write_i32::<LittleEndian>(self.flags.len() as i32)?;
        for flag in &self.flags {
            write_string(writer, flag)?;
        }
        writer.write_i32::<LittleEndian>(self.bounty_seed)?;
        writer.write_i32::<LittleEndian>(self.bounties_complete)?;
        Ok(())
    }
}