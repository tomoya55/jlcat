mod cli;
mod error;
mod input;

use clap::Parser;
use cli::Cli;
use error::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.file.is_none() && atty::is(atty::Stream::Stdin) {
        eprintln!("Usage: jlcat [OPTIONS] [FILE]");
        eprintln!("Try 'jlcat --help' for more information.");
        std::process::exit(1);
    }

    println!("{:?}", cli);
    Ok(())
}
