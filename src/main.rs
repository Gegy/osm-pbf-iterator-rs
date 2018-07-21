#![feature(try_from)]

extern crate byteorder;
extern crate flate2;
extern crate protobuf;

use osm::{MemberReference, NodeReference, OsmReader};
use protos::osm::HeaderBlock;
use reader::BlobReader;
use std::collections::HashSet;
use std::convert::From;
use std::fs::File;
use std::io::Read;
use visitor::OsmVisitor;
use writer::OsmWriterVisitor;

mod protos;
mod blob;
mod visitor;
mod reader;
mod writer;
mod osm;

const INPUT_PATH: &str = "inputs/antarctica-latest.osm.pbf";
const OUTPUT_PATH: &str = "outputs/coastline.osm.pbf";

fn main() {
    std::fs::create_dir_all("outputs").expect("failed to create output directory");

    let mut input_file = File::open(INPUT_PATH).expect("failed to open input file");
    let mut output_file = File::create(OUTPUT_PATH).expect("failed to create output file");

    let mut node_collector = NodeCollectionVisitor::new();

    let mut reader = OsmReader::from(BlobReader::from(&mut input_file));

    println!("collecting coastline nodes");
    reader.accept(&mut node_collector);
    println!("collected {} nodes", node_collector.nodes.len());

    let mut writer = OsmWriterVisitor::new(&mut output_file);
    reader.accept(&mut CoastlineVisitor { parent: &mut writer, nodes: &node_collector.nodes });
}

pub struct NodeCollectionVisitor {
    nodes: HashSet<i64>,
}

impl NodeCollectionVisitor {
    fn new() -> NodeCollectionVisitor {
        NodeCollectionVisitor {
            nodes: HashSet::new()
        }
    }
}

// TODO: Relation members?
impl OsmVisitor for NodeCollectionVisitor {
    fn visit_way(&mut self, _id: i64, nodes: Vec<NodeReference>, tags: Vec<(String, String)>) -> Result<(), PbfParseError> {
        if tags.iter().any(|(k, v)| *k == "natural" && *v == "coastline") {
            for node in nodes {
                self.nodes.insert(node.id);
            }
        }
        Ok(())
    }
}

pub struct CoastlineVisitor<'a> {
    parent: &'a mut OsmVisitor,
    nodes: &'a HashSet<i64>,
}

impl<'a> OsmVisitor for CoastlineVisitor<'a> {
    fn visit_block(&mut self, _lat_offset: i64, _lon_offset: i64, _granularity: i32, _date_granularity: i32) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn end_block(&mut self) -> Result<(), PbfParseError> {
        self.parent.end_block()
    }

    fn visit_node(&mut self, id: i64, latitude: f64, longitude: f64) -> Result<(), PbfParseError> {
        if self.nodes.contains(&id) {
            self.parent.visit_node(id, latitude, longitude)
        } else {
            Ok(())
        }
    }

    fn visit_way(&mut self, id: i64, nodes: Vec<NodeReference>, tags: Vec<(String, String)>) -> Result<(), PbfParseError> {
        if tags.iter().any(|(k, v)| *k == "natural" && *v == "coastline") {
            self.parent.visit_way(id, nodes, tags)
        } else {
            Ok(())
        }
    }

    fn visit_relation(&mut self, id: i64, members: Vec<MemberReference>, tags: Vec<(String, String)>) -> Result<(), PbfParseError> {
        if tags.iter().any(|(k, v)| *k == "natural" && *v == "coastline") {
            self.parent.visit_relation(id, members, tags)
        } else {
            Ok(())
        }
    }

    fn visit_info(&mut self, version: i32, timestamp: i64, changeset: i64, uid: i32, user_sid: u32, visible: bool) -> Result<(), PbfParseError> {
        self.parent.visit_info(version, timestamp, changeset, uid, user_sid, visible)
    }

    fn visit_header(&mut self, block: &HeaderBlock) -> Result<(), PbfParseError> {
//        println!("found header {:?}", block);
        self.parent.visit_header(block)
    }

    fn end(&mut self) -> Result<(), PbfParseError> {
        self.parent.end()
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
