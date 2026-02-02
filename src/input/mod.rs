mod detector;
mod source;
mod spooler;

pub use detector::{sniff_format, InputFormat};
pub use source::{FileSource, InputSource, StdinSource};
pub use spooler::SpooledInput;
