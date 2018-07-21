use ::PbfParseError;
use blob::Blob;
use std::io::{Read, Seek, SeekFrom};
use visitor::BlobVisitor;

pub struct BlobReader<'a, T: 'a + Read + Seek> {
    reader: &'a mut T,
}

impl<'a, T: 'a + Read + Seek> BlobReader<'a, T> {
    pub fn from(reader: &'a mut T) -> BlobReader<'a, T> {
        BlobReader { reader }
    }

    pub fn accept(&mut self, visitor: &mut BlobVisitor) {
        match self.try_accept(visitor) {
            Err(ref e) => {
                visitor.handle_error(e);
            }
            _ => (),
        }
    }

    fn try_accept(&mut self, visitor: &mut BlobVisitor) -> Result<(), PbfParseError> {
        self.reader.seek(SeekFrom::Start(0))?;
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
        visitor.end()?;
        Ok(())
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
