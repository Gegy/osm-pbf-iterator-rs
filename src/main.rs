#![feature(try_from)]

extern crate byteorder;
extern crate flate2;
extern crate protobuf;

use blob::Blob;
use iterator::{BlobIterator, OsmIterator};
use parser::Parser;
use std::convert::From;
use std::fs::File;
use std::io::Read;

mod protos;
mod parser;
mod blob;
mod iterator;

fn main() {
    let mut input_file = File::open("inputs/antarctica-latest.osm.pbf").expect("failed to open input file");

    let mut parser = Parser::of(&mut input_file);
    let iterator = BlobIterator::of(&mut parser);
    iterator.for_each(|blob| {
        println!("found blob of type {:?}", blob.data_type);
    });
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum OsmEntity {
    Node,
    Way,
    Relation,
}

#[derive(Debug)]
pub enum OsmParseError {
    Io(std::io::Error),
    Eof,
    InvalidHeaderLength(u32),
    InvalidBodyLength(u32),
    InvalidMessage(protobuf::ProtobufError),
    InvalidBlobFormat,
    InvalidBlobType,
}

impl From<std::io::Error> for OsmParseError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::UnexpectedEof {
            OsmParseError::Eof
        } else {
            OsmParseError::Io(err)
        }
    }
}

impl From<protobuf::ProtobufError> for OsmParseError {
    fn from(err: protobuf::ProtobufError) -> Self {
        OsmParseError::InvalidMessage(err)
    }
}
