#![feature(try_from)]

extern crate byteorder;
extern crate flate2;
extern crate protobuf;

use blob::Blob;
use reader::BlobReader;
use std::convert::From;
use std::fs::File;
use std::io::Read;

mod protos;
mod blob;
mod visitor;
mod reader;

fn main() {
    let mut input_file = File::open("inputs/antarctica-latest.osm.pbf").expect("failed to open input file");

    let mut reader = BlobReader::from(&mut input_file);
    reader.accept(&mut Visitor);
}

pub struct Visitor;

impl visitor::BlobVisitor for Visitor {
    fn visit_blob(&mut self, blob: Blob) {
        println!("found blob of type {:?}", blob.data_type);
    }

    fn handle_error(&mut self, error: &PbfParseError) -> bool {
        eprintln!("encountered error {:?}", error);
        false
    }
}

pub fn read_message<M: protobuf::Message>(reader: &mut Read, length: usize) -> Result<M, PbfParseError> {
    let mut buffer = vec!(0u8; length as usize);
    reader.read_exact(&mut buffer)?;
    Ok(protobuf::parse_from_bytes(&buffer)?)
}

#[derive(Debug)]
pub enum PbfParseError {
    Io(std::io::Error),
    Eof,
    InvalidHeaderLength(u32),
    InvalidBodyLength(u32),
    InvalidMessage(protobuf::ProtobufError),
    InvalidBlobFormat,
    InvalidBlobType,
}

impl From<std::io::Error> for PbfParseError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::UnexpectedEof {
            PbfParseError::Eof
        } else {
            PbfParseError::Io(err)
        }
    }
}

impl From<protobuf::ProtobufError> for PbfParseError {
    fn from(err: protobuf::ProtobufError) -> Self {
        PbfParseError::InvalidMessage(err)
    }
}
