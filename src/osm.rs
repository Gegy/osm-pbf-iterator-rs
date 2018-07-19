use ::PbfParseError;
use blob::{Blob, BlobType};
use protos::osm::{DenseNodes, HeaderBlock, Node, PrimitiveBlock, PrimitiveGroup, Relation, Relation_MemberType, StringTable, Way};
use reader::BlobReader;
use std::collections::HashMap;
use std::str;
use visitor::{BlobVisitor, OsmVisitor};

const NANODEGREE_UNIT: f64 = 1e-9;

pub struct OsmReader<'a> {
    reader: BlobReader<'a>,
}

impl<'a> OsmReader<'a> {
    pub fn from(reader: BlobReader<'a>) -> OsmReader<'a> {
        OsmReader { reader }
    }

    pub fn accept(&mut self, visitor: &mut OsmVisitor) {
        self.reader.accept(&mut OsmBlobVisitor::new(visitor));
    }
}

struct OsmBlobVisitor<'a> {
    delegate: &'a mut OsmVisitor,
}

impl<'a> OsmBlobVisitor<'a> {
    fn new(delegate: &'a mut OsmVisitor) -> OsmBlobVisitor<'a> {
        OsmBlobVisitor { delegate }
    }

    fn visit_group(&mut self, parser: &OsmBlockParser, group: &PrimitiveGroup) -> Result<(), PbfParseError> {
        self.delegate.visit_group()?;
        let nodes = group.get_nodes();
        let ways = group.get_ways();
        let relations = group.get_relations();
        if !nodes.is_empty() {
            self.visit_nodes(parser, nodes)?;
        } else if !ways.is_empty() {
            self.visit_ways(parser, ways)?;
        } else if !relations.is_empty() {
            self.visit_relations(parser, relations)?;
        } else if group.has_dense() {
            self.visit_dense_nodes(parser, group.get_dense())?;
        }
        self.delegate.end_group()?;
        Ok(())
    }

    fn visit_nodes(&mut self, parser: &OsmBlockParser, nodes: &[Node]) -> Result<(), PbfParseError> {
        for node in nodes {
            let latitude = parser.get_lat(node.get_lat());
            let longitude = parser.get_lon(node.get_lon());
            self.delegate.visit_node(node.get_id(), latitude, longitude)?;
        }
        Ok(())
    }

    fn visit_ways(&mut self, parser: &OsmBlockParser, ways: &[Way]) -> Result<(), PbfParseError> {
        for way in ways {
            let tags = parser.parse_tags(way.get_keys(), way.get_vals());
            self.delegate.visit_way(way.get_id(), way.get_refs(), &tags)?;
        }
        Ok(())
    }

    fn visit_relations(&mut self, parser: &OsmBlockParser, relations: &[Relation]) -> Result<(), PbfParseError> {
        for relation in relations {}
        Ok(())
    }

    fn visit_dense_nodes(&mut self, parser: &OsmBlockParser, dense: &DenseNodes) -> Result<(), PbfParseError> {
        let mut current_id: i64 = 0;
        let mut current_lat: i64 = 0;
        let mut current_lon: i64 = 0;

        let coord_iter = dense.get_lat().iter().zip(dense.get_lon());
        for (off_id, (off_lat, off_lon)) in dense.get_id().iter().zip(coord_iter) {
            current_id += off_id;
            current_lat += off_lat;
            current_lon += off_lon;
            let latitude = parser.get_lat(current_lat);
            let longitude = parser.get_lon(current_lon);
            self.delegate.visit_node(current_id, latitude, longitude)?;
        }

        Ok(())
    }
}

impl<'a> BlobVisitor for OsmBlobVisitor<'a> {
    fn visit_blob(&mut self, blob: &Blob) -> Result<(), PbfParseError> {
        let data = blob.data.as_ref();
        match blob.data_type {
            BlobType::DATA => {
                let block: PrimitiveBlock = ::read_message_bytes(data)?;
                let block_parser = OsmBlockParser::new(&block);
                self.delegate.visit_block(block.get_lat_offset(), block.get_lon_offset(), block.get_granularity(), block.get_date_granularity())?;
                self.delegate.visit_string_table(&block_parser.strings)?;
                for group in block.get_primitivegroup() {
                    self.visit_group(&block_parser, group)?;
                }
                self.delegate.end_block()?;
            }
            BlobType::HEADER => {
                let block: HeaderBlock = ::read_message_bytes(data)?;
                self.delegate.visit_header(&block)?;
            }
        }
        Ok(())
    }

    fn handle_error(&mut self, error: &PbfParseError) -> bool {
        self.delegate.handle_error(error)
    }
}

struct OsmBlockParser<'a> {
    origin_latitude: i64,
    origin_longitude: i64,
    granularity: i64,
    strings: Vec<&'a str>,
}

impl<'a> OsmBlockParser<'a> {
    fn new(block: &'a PrimitiveBlock) -> OsmBlockParser<'a> {
        OsmBlockParser {
            origin_latitude: block.get_lat_offset(),
            origin_longitude: block.get_lon_offset(),
            granularity: block.get_granularity() as i64,
            strings: parse_string_table(block.get_stringtable()),
        }
    }

    fn get_lat(&self, lat: i64) -> f64 {
        (self.origin_latitude + lat * self.granularity) as f64 * NANODEGREE_UNIT
    }

    fn get_lon(&self, lon: i64) -> f64 {
        (self.origin_longitude + lon * self.granularity) as f64 * NANODEGREE_UNIT
    }

    fn parse_tags<'b>(&self, keys: &'b [u32], values: &'b [u32]) -> Vec<(&'b str, &'b str)>
        where 'a: 'b
    {
        keys.iter().zip(values)
            .map(|(key, value)| (
                self.strings[*key as usize],
                self.strings[*value as usize]
            ))
            .collect()
    }
}

fn parse_string_table<'a>(table: &'a StringTable) -> Vec<&'a str> {
    let table_data = table.get_s();
    table_data.iter()
        .map(|s| str::from_utf8(s.as_ref()))
        .filter_map(|s| s.ok())
        .collect()
}
