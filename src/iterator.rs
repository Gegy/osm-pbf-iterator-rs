use ::{OsmEntity, OsmParseError};
use blob::Blob;
use parser::Parser;
use protos::osm::HeaderBlock;

pub struct OsmIterator<'a> {
    blob_iterator: BlobIterator<'a>,
    blob: Option<Blob>,
    cursor: u64,
}

impl<'a> OsmIterator<'a> {
    pub fn of(blob_iterator: BlobIterator<'a>) -> OsmIterator<'a> {
        OsmIterator { blob_iterator, blob: None, cursor: 0 }
    }
}

impl<'a> Iterator for OsmIterator<'a> {
    type Item = OsmEntity;

    fn next(&mut self) -> Option<OsmEntity> {
        None
    }
}

pub struct BlobIterator<'a> {
    parser: &'a mut Parser<'a>
}

impl<'a> BlobIterator<'a> {
    pub fn of(input: &'a mut Parser<'a>) -> BlobIterator<'a> {
        BlobIterator { parser: input }
    }
}

impl<'a> Iterator for BlobIterator<'a> {
    type Item = Blob;

    fn next(&mut self) -> Option<Blob> {
        let blob = Blob::parse(self.parser);
        match blob {
            Err(OsmParseError::Eof) => return None,
            Err(ref e) => eprintln!("encountered error while parsing blob: {:?}", e),
            _ => (),
        }
        blob.ok()
    }
}

