use ::PbfParseError;
use blob::{Blob, BlobType};
use protos::osm::{DenseNodes, HeaderBlock, Node, PrimitiveBlock, PrimitiveGroup, Relation, Relation_MemberType, Way};
use std::io::Read;
use visitor::{BlobVisitor, OsmVisitor};

const NANODEGREE_UNIT: f64 = 1e-9;

pub struct BlobReader<'a> {
    reader: &'a mut Read,
}

impl<'a> BlobReader<'a> {
    pub fn from(reader: &'a mut Read) -> BlobReader<'a> {
        BlobReader { reader }
    }

    pub fn accept(&mut self, visitor: &mut BlobVisitor) {
        loop {
            let result = parse_blob(self.reader, visitor);
            match result {
                Err(PbfParseError::Eof) => break,
                Err(ref e) => {
                    if visitor.handle_error(e) {
                        break;
                    }
                }
                _ => (),
            }
        }
    }
}

fn parse_blob(reader: &mut Read, visitor: &mut BlobVisitor) -> Result<Blob, PbfParseError> {
    match Blob::parse(reader) {
        Ok(blob) => {
            let result = visitor.visit_blob(&blob);
            match result {
                Ok(_) => Ok(blob),
                Err(e) => Err(e),
            }
        }
        err => err,
    }
}

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

    fn visit_group(&mut self, group: &PrimitiveGroup, origin_latitude: i64, origin_longitude: i64, granularity: i64) -> Result<(), PbfParseError> {
        self.delegate.visit_group(group)?;
        let nodes = group.get_nodes();
        let ways = group.get_ways();
        let relations = group.get_relations();
        if !nodes.is_empty() {
            self.visit_nodes(nodes, origin_latitude, origin_longitude, granularity)?;
        } else if !ways.is_empty() {
            self.visit_ways(ways)?;
        } else if !relations.is_empty() {
            self.visit_relations(relations)?;
        } else if group.has_dense() {
            self.visit_dense_nodes(group.get_dense(), origin_latitude, origin_longitude, granularity)?;
        }
        Ok(())
    }

    fn visit_nodes(&mut self, nodes: &[Node], origin_latitude: i64, origin_longitude: i64, granularity: i64) -> Result<(), PbfParseError> {
        for node in nodes {
            let latitude = (node.get_lat() * granularity + origin_latitude) as f64 * NANODEGREE_UNIT;
            let longitude = (node.get_lon() * granularity + origin_longitude) as f64 * NANODEGREE_UNIT;
            self.delegate.visit_node(node.get_id(), latitude, longitude)?;
        }
        Ok(())
    }

    fn visit_ways(&mut self, ways: &[Way]) -> Result<(), PbfParseError> {
        for way in ways {
            self.delegate.visit_way(way.get_id(), way.get_refs())?;
        }
        Ok(())
    }

    fn visit_relations(&mut self, relations: &[Relation]) -> Result<(), PbfParseError> {
        for relation in relations {
        }
        Ok(())
    }

    fn visit_dense_nodes(&mut self, dense: &DenseNodes, origin_latitude: i64, origin_longitude: i64, granularity: i64) -> Result<(), PbfParseError> {
        let mut current_id: i64 = 0;
        let mut current_lat: i64 = 0;
        let mut current_lon: i64 = 0;

        let coord_iter = dense.get_lat().iter().zip(dense.get_lon());
        for (off_id, (off_lat, off_lon)) in dense.get_id().iter().zip(coord_iter) {
            current_id += off_id;
            current_lat += off_lat;
            current_lon += off_lon;
            let latitude = (current_lat * granularity + origin_latitude) as f64 * NANODEGREE_UNIT;
            let longitude = (current_lon * granularity + origin_longitude) as f64 * NANODEGREE_UNIT;
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
                let origin_latitude = block.get_lat_offset();
                let origin_longitude = block.get_lon_offset();
                let granularity = block.get_granularity() as i64;
                self.delegate.visit_block(&block)?;
                for group in block.get_primitivegroup() {
                    self.visit_group(group, origin_latitude, origin_longitude, granularity)?;
                }
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
