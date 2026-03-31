use std::io::{Read, Write};
use crate::utils::SaveError;

pub trait BinarySerializable: Sized {
    fn read<R: Read>(reader: &mut R, version: i32) -> Result<Self, SaveError>;
    fn write<W: Write>(&self, writer: &mut W, version: i32) -> Result<(), SaveError>;
}