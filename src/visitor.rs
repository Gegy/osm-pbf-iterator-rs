use blob::Blob;
use ::PbfParseError;
use protos::osm::{PrimitiveBlock, PrimitiveGroup, HeaderBlock};

pub trait BlobVisitor {
    fn visit_blob(&mut self, blob: &Blob) -> Result<(), PbfParseError>;

    fn handle_error(&mut self, _error: &PbfParseError) -> bool {
        false
    }
}

pub trait OsmVisitor {
    fn visit_block(&mut self, block: &PrimitiveBlock) -> Result<(), PbfParseError>;

    fn visit_group(&mut self, group: &PrimitiveGroup) -> Result<(), PbfParseError>;

    fn visit_node(&mut self, id: i64, latitude: f64, longitude: f64) -> Result<(), PbfParseError>;

    fn visit_way(&mut self, id: i64, refs: &[i64]) -> Result<(), PbfParseError>;

    fn visit_header(&mut self, block: &HeaderBlock) -> Result<(), PbfParseError>;

    fn handle_error(&mut self, _error: &PbfParseError) -> bool {
        false
    }
}
