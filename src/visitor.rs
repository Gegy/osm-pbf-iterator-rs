use ::PbfParseError;
use blob::Blob;
use osm::{MemberReference, NodeReference};
use protos::osm::{HeaderBlock};

pub trait BlobVisitor {
    fn visit_blob(&mut self, blob: &Blob) -> Result<(), PbfParseError>;

    fn end(&mut self) -> Result<(), PbfParseError>;

    fn handle_error(&mut self, _error: &PbfParseError) -> bool {
        false
    }
}

pub trait OsmVisitor {
    fn visit_block(&mut self, lat_offset: i64, lon_offset: i64, granularity: i32, date_granularity: i32) -> Result<(), PbfParseError>;

    fn visit_string_table(&mut self, strings: &Vec<&str>) -> Result<(), PbfParseError>;

    fn end_block(&mut self) -> Result<(), PbfParseError>;

    fn visit_group(&mut self) -> Result<(), PbfParseError>;

    fn end_group(&mut self) -> Result<(), PbfParseError>;

    fn visit_node(&mut self, id: i64, latitude: f64, longitude: f64) -> Result<(), PbfParseError>;

    fn visit_way(&mut self, id: i64, nodes: Vec<NodeReference>, tags: Vec<(String, String)>) -> Result<(), PbfParseError>;

    fn visit_relation(&mut self, id: i64, members: Vec<MemberReference>, tags: Vec<(String, String)>) -> Result<(), PbfParseError>;

    fn visit_info(&mut self, version: i32, timestamp: i64, changeset: i64, uid: i32, user_sid: u32, visible: bool) -> Result<(), PbfParseError>;

    fn visit_header(&mut self, block: &HeaderBlock) -> Result<(), PbfParseError>;

    fn end(&mut self) -> Result<(), PbfParseError>;

    fn handle_error(&mut self, _error: &PbfParseError) -> bool {
        false
    }
}
