#![feature(try_from)]
#![feature(nll)]

extern crate byteorder;
extern crate flate2;
extern crate protobuf;

use osm::OsmReader;
use protos::osm::HeaderBlock;
use reader::BlobReader;
use std::convert::From;
use std::fs::File;
use std::io::Read;

mod protos;
mod blob;
mod visitor;
mod reader;
mod osm;

fn main() {
    let mut input_file = File::open("inputs/antarctica-latest.osm.pbf").expect("failed to open input file");

    let mut reader = OsmReader::from(BlobReader::from(&mut input_file));
    reader.accept(&mut Visitor);
}

pub struct Visitor;

impl visitor::OsmVisitor for Visitor {
    fn visit_block(&mut self, _lat_offset: i64, _lon_offset: i64, _granularity: i32, _date_granularity: i32) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn visit_string_table(&mut self, _strings: &Vec<&str>) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn end_block(&mut self) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn visit_group(&mut self) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn end_group(&mut self) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn visit_node(&mut self, id: i64, latitude: f64, longitude: f64) -> Result<(), PbfParseError> {
//        println!("found node with id {} at {} {}", id, latitude, longitude);
        Ok(())
    }

    fn visit_way(&mut self, id: i64, refs: &[i64], tags: &Vec<(&str, &str)>) -> Result<(), PbfParseError> {
        if !tags.is_empty() {
            println!("found way with id {} and tags: {:?}", id, tags);
        }
        Ok(())
    }

    fn visit_header(&mut self, block: &HeaderBlock) -> Result<(), PbfParseError> {
        println!("found header {:?}", block);
        Ok(())
    }

    fn handle_error(&mut self, error: &PbfParseError) -> bool {
        println!("found error {:?}", error);
        false
    }
}

pub fn read_message<M: protobuf::Message>(reader: &mut Read, length: usize) -> Result<M, PbfParseError> {
    let mut buffer = vec!(0u8; length as usize);
    reader.read_exact(&mut buffer)?;
    Ok(protobuf::parse_from_bytes(&buffer)?)
}

pub fn read_message_bytes<M: protobuf::Message>(buffer: &[u8]) -> Result<M, PbfParseError> {
    Ok(protobuf::parse_from_bytes(buffer)?)
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
