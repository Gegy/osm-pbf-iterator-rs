use ::PbfParseError;
use blob::Blob;
use osm::{MemberReference, NodeReference};
use protos::osm::HeaderBlock;

pub trait BlobVisitor {
    fn visit_blob(&mut self, blob: &Blob) -> Result<(), PbfParseError>;

    fn end(&mut self) -> Result<(), PbfParseError>;

    fn handle_error(&mut self, _error: &PbfParseError) -> bool {
        false
    }
}

pub trait OsmVisitor {
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

    fn visit_node(&mut self, _id: i64, _latitude: f64, _longitude: f64) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn visit_way(&mut self, _id: i64, _nodes: Vec<NodeReference>, _tags: Vec<(String, String)>) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn visit_relation(&mut self, _id: i64, _members: Vec<MemberReference>, _tags: Vec<(String, String)>) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn visit_info(&mut self, _version: i32, _timestamp: i64, _changeset: i64, _uid: i32, _user_sid: u32, _visible: bool) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn visit_header(&mut self, _block: &HeaderBlock) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn end(&mut self) -> Result<(), PbfParseError> {
        Ok(())
    }

    fn handle_error(&mut self, _error: &PbfParseError) -> bool {
        false
    }
}
