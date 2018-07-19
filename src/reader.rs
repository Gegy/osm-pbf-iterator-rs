use ::PbfParseError;
use blob::{Blob, BlobType};
use protos::osm::{HeaderBlock, PrimitiveBlock};
use std::io::Read;
use visitor::{BlobVisitor, OsmVisitor};

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
}

impl<'a> BlobVisitor for OsmBlobVisitor<'a> {
    fn visit_blob(&mut self, blob: &Blob) -> Result<(), PbfParseError> {
        let data = blob.data.as_ref();
        match blob.data_type {
            BlobType::DATA => {
                let block: PrimitiveBlock = ::read_message_bytes(data)?;
                self.delegate.visit_block(&block)?;
            },
            BlobType::HEADER => {
                let block: HeaderBlock = ::read_message_bytes(data)?;
                self.delegate.visit_header(&block)?;
            },
        }
        Ok(())
    }

    fn handle_error(&mut self, error: &PbfParseError) -> bool {
        self.delegate.handle_error(error)
    }
}
