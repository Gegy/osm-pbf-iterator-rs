use std::io::Read;
use protobuf;
use byteorder::{BigEndian, ReadBytesExt};
use ::OsmParseError;

pub struct Parser<'a> {
    reader: &'a mut Read,
}

impl<'a> Parser<'a> {
    pub fn of(reader: &'a mut Read) -> Parser<'a> {
        Parser { reader }
    }

    pub fn read_u32(&mut self) -> Result<u32, OsmParseError> {
        Ok(self.reader.read_u32::<BigEndian>()?)
    }

    pub fn read_message<M: protobuf::Message>(&mut self, length: usize) -> Result<M, OsmParseError> {
        let mut buffer = vec!(0u8; length as usize);
        self.reader.read_exact(&mut buffer)?;
        Ok(protobuf::parse_from_bytes(&buffer)?)
    }
}
