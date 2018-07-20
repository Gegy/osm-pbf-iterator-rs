use ::PbfParseError;
use blob::Blob;
use std::io::Read;
use visitor::BlobVisitor;

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
        match visitor.end() {
            Err(ref e) => {
                visitor.handle_error(e);
            },
            _ => (),
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
