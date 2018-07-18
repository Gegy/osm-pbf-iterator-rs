use blob::Blob;
use ::PbfParseError;

pub trait BlobVisitor {
    fn visit_blob(&mut self, blob: Blob);

    fn handle_error(&mut self, _error: &PbfParseError) -> bool {
        false
    }
}
