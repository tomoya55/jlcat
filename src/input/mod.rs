mod detector;
mod source;

pub use detector::{sniff_format, InputFormat};
pub use source::{FileSource, InputSource, StdinSource};
