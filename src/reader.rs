use ::PbfParseError;
use blob::Blob;
use std::io::Read;
use visitor::BlobVisitor;

pub struct BlobReader<'a> {
    reader: &'a mut Read
}

impl<'a> BlobReader<'a> {
    pub fn from(input: &'a mut Read) -> BlobReader<'a> {
        BlobReader { reader: input }
    }

    pub fn accept(&mut self, visitor: &mut BlobVisitor) {
        loop {
            match Blob::parse(self.reader) {
                Ok(blob) => visitor.visit_blob(blob),
                Err(PbfParseError::Eof) => break,
                Err(ref e) => {
                    if visitor.handle_error(e) {
                        break;
                    }
                },
            }
        }
    }
}

impl<'a> Iterator for BlobReader<'a> {
    type Item = Blob;

    fn next(&mut self) -> Option<Blob> {
        let blob = Blob::parse(self.reader);
        match blob {
            Err(PbfParseError::Eof) => return None,
            Err(ref e) => eprintln!("encountered error while parsing blob: {:?}", e),
            _ => (),
        }
        blob.ok()
    }
}

