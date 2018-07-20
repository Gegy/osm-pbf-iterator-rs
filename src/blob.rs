use ::PbfParseError;
use flate2::{Compression, write::ZlibEncoder};
use flate2::read::ZlibDecoder;
use protos;
use protos::file;
use std::convert::{TryFrom, Into};
use std::io::{Read, Write};

const MAX_HEADER_LENGTH: u32 = 64 * 1024;
const MAX_BODY_LENGTH: u32 = 32 * 1024 * 1024;

#[derive(Debug)]
pub struct Blob {
    pub data_type: BlobType,
    pub data: Vec<u8>,
}

impl Blob {
    pub fn parse(reader: &mut Read) -> Result<Blob, PbfParseError> {
        let header = parse_header(reader)?;
        let blob = parse_blob(reader, &header)?;
        let data_type = BlobType::try_from(header.get_field_type())?;
        let data = parse_data(blob)?;
        Ok(Blob { data_type, data })
    }

    pub fn new(data_type: BlobType, data: Vec<u8>) -> Blob {
        Blob { data_type, data }
    }

    pub fn write(&self, writer: &mut Write) -> Result<(), PbfParseError> {
        write_header(writer, &self.data_type, self.data.len())
    }
}

fn write_header(writer: &mut Write, data_type: &BlobType, data_len: usize) -> Result<(), PbfParseError> {
    use byteorder::{BigEndian, WriteBytesExt};
    use protobuf::Message;

    let mut header = file::BlobHeader::default();
    header.set_field_type((*data_type).into());
    header.set_datasize(data_len as i32);

    let bytes = header.write_to_bytes()?;
    writer.write_u32::<BigEndian>(bytes.len() as u32)?;

    writer.write_all(&bytes)?;

    Ok(())
}

fn parse_header(reader: &mut Read) -> Result<file::BlobHeader, PbfParseError> {
    use byteorder::{BigEndian, ReadBytesExt};
    let header_length = reader.read_u32::<BigEndian>()?;
    if header_length >= MAX_HEADER_LENGTH {
        return Err(PbfParseError::InvalidHeaderLength(header_length));
    }
    ::read_message(reader, header_length as usize)
}

fn write_blob(writer: &mut Write, data: &[u8]) -> Result<(), PbfParseError> {
    use protobuf::Message;

    let mut deflated: Vec<u8> = Vec::new();

    {
        let mut encoder = ZlibEncoder::new(&mut deflated, Compression::new(9));
        encoder.write_all(data)?;
    }

    let mut blob = file::Blob::default();
    blob.set_lzma_data(deflated);
    blob.set_raw_size(data.len() as i32);

    blob.write_to_writer(writer)?;

    Ok(())
}

fn parse_blob(reader: &mut Read, header: &file::BlobHeader) -> Result<file::Blob, PbfParseError> {
    let data_length = header.get_datasize() as u32;
    if data_length > MAX_BODY_LENGTH {
        return Err(PbfParseError::InvalidBodyLength(data_length));
    }
    ::read_message(reader, data_length as usize)
}

fn parse_data(blob: protos::file::Blob) -> Result<Vec<u8>, PbfParseError> {
    if blob.has_zlib_data() {
        let mut inflated: Vec<u8> = vec![];
        let mut decoder = ZlibDecoder::new(blob.get_zlib_data());
        decoder.read_to_end(&mut inflated)?;
        Ok(inflated)
    } else {
        Err(PbfParseError::InvalidBlobFormat)
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum BlobType {
    HEADER,
    DATA,
}

impl<'a> TryFrom<&'a str> for BlobType {
    type Error = PbfParseError;

    fn try_from(value: &'a str) -> Result<Self, PbfParseError> {
        match value {
            "OSMHeader" => Ok(BlobType::HEADER),
            "OSMData" => Ok(BlobType::DATA),
            _ => Err(PbfParseError::InvalidBlobType)
        }
    }
}

impl Into<String> for BlobType {
    fn into(self) -> String {
        match self {
            BlobType::HEADER => "OSMHeader".to_string(),
            BlobType::DATA => "OSMData".to_string(),
        }
    }
}
