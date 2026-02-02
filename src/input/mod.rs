#[allow(dead_code)]
mod cached;
mod detector;
#[allow(dead_code)]
mod indexed;
#[allow(dead_code)]
mod source;
#[allow(dead_code)]
mod spooler;

pub use detector::{sniff_format, InputFormat};
