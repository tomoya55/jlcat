mod cached;
mod detector;
mod indexed;
mod source;
mod spooler;

pub use cached::CachedReader;
pub use detector::{sniff_format, InputFormat};
pub use indexed::{IndexedReader, IndexedRowIterator};
pub use source::{FileSource, InputSource, StdinSource};
pub use spooler::SpooledInput;
