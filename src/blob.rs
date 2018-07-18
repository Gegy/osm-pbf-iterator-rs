use ::OsmParseError;
use flate2;
use parser::Parser;
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
    pub fn parse(parser: &mut Parser) -> Result<Blob, OsmParseError> {
        let header = Blob::parse_header(parser)?;
        let blob = Blob::parse_blob(parser, &header)?;
        let data_type = BlobType::try_from(header.get_field_type())?;
        let data = parse_data(blob)?;
        Ok(Blob { data_type, data })
    }

    fn parse_header(parser: &mut Parser) -> Result<protos::file::BlobHeader, OsmParseError> {
        let header_length = parser.read_u32()?;
        if header_length >= MAX_HEADER_LENGTH {
            return Err(OsmParseError::InvalidHeaderLength(header_length));
        }
        parser.read_message(header_length as usize)
    }

    fn parse_blob(parser: &mut Parser, header: &protos::file::BlobHeader) -> Result<protos::file::Blob, OsmParseError> {
        let data_length = header.get_datasize() as u32;
        if data_length > MAX_BODY_LENGTH {
            return Err(OsmParseError::InvalidBodyLength(data_length));
        }
        parser.read_message(data_length as usize)
    }
}

fn parse_data(blob: protos::file::Blob) -> Result<Vec<u8>, OsmParseError> {
    if blob.has_zlib_data() {
        let mut deflated = vec![0u8];
        let mut decoder = flate2::read::ZlibDecoder::new(blob.get_zlib_data());
        decoder.read_to_end(&mut deflated)?;
        Ok(deflated)
    } else {
        Err(OsmParseError::InvalidBlobFormat)
    }
}

#[derive(Debug)]
pub enum BlobType {
    HEADER,
    DATA,
}

impl<'a> TryFrom<&'a str> for BlobType {
    type Error = OsmParseError;

    fn try_from(value: &'a str) -> Result<Self, OsmParseError> {
        match value {
            "OSMHeader" => Ok(BlobType::HEADER),
            "OSMData" => Ok(BlobType::DATA),
            _ => Err(OsmParseError::InvalidBlobType)
        }
    }
}
