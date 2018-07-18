use ::PbfParseError;
use flate2;
use protos;
use std::convert::TryFrom;
use std::io::Read;

const MAX_HEADER_LENGTH: u32 = 64 * 1024;
const MAX_BODY_LENGTH: u32 = 32 * 1024 * 1024;

pub struct Blob {
    pub data_type: BlobType,
    pub data: Vec<u8>,
}

impl Blob {
    pub fn parse(reader: &mut Read) -> Result<Blob, PbfParseError> {
        let header = Blob::parse_header(reader)?;
        let blob = Blob::parse_blob(reader, &header)?;
        let data_type = BlobType::try_from(header.get_field_type())?;
        let data = parse_data(blob)?;
        Ok(Blob { data_type, data })
    }

    fn parse_header(reader: &mut Read) -> Result<protos::file::BlobHeader, PbfParseError> {
        use byteorder::{BigEndian, ReadBytesExt};
        let header_length = reader.read_u32::<BigEndian>()?;
        if header_length >= MAX_HEADER_LENGTH {
            return Err(PbfParseError::InvalidHeaderLength(header_length));
        }
        ::read_message(reader, header_length as usize)
    }

    fn parse_blob(reader: &mut Read, header: &protos::file::BlobHeader) -> Result<protos::file::Blob, PbfParseError> {
        let data_length = header.get_datasize() as u32;
        if data_length > MAX_BODY_LENGTH {
            return Err(PbfParseError::InvalidBodyLength(data_length));
        }
        ::read_message(reader,data_length as usize)
    }
}

fn parse_data(blob: protos::file::Blob) -> Result<Vec<u8>, PbfParseError> {
    if blob.has_zlib_data() {
        let mut deflated = vec![0u8];
        let mut decoder = flate2::read::ZlibDecoder::new(blob.get_zlib_data());
        decoder.read_to_end(&mut deflated)?;
        Ok(deflated)
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
