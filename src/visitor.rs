use blob::Blob;
use ::PbfParseError;
use protos::osm::{PrimitiveBlock, HeaderBlock};

pub trait BlobVisitor {
    fn visit_blob(&mut self, blob: &Blob) -> Result<(), PbfParseError>;

    fn handle_error(&mut self, _error: &PbfParseError) -> bool {
        false
    }
}

pub trait OsmVisitor {
    fn visit_block(&mut self, block: &PrimitiveBlock) -> Result<(), PbfParseError>;

    fn visit_header(&mut self, block: &HeaderBlock) -> Result<(), PbfParseError>;

    fn handle_error(&mut self, _error: &PbfParseError) -> bool {
        false
    }
}
