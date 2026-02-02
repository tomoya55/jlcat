#[allow(dead_code)]
mod cached;
mod detector;
#[allow(dead_code)]
mod indexed;
#[allow(dead_code)]
mod source;
#[allow(dead_code)]
mod spooler;

#[allow(unused_imports)]
pub use cached::CachedReader;
pub use detector::{sniff_format, InputFormat};
#[allow(unused_imports)]
pub use indexed::IndexedReader;
