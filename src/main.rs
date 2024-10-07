use clap::{Arg, Command};
use std::fs::File;
use std::io::{self, Read, Write};
use torrust_bencode2json::parsers::BencodeParser;

fn main() -> io::Result<()> {
    let matches = Command::new("torrust-bencode2json")
        .version("0.1.0")
        .author("Torrust Organization")
        .about("Converts Bencode to JSON")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .default_value(None)
                .help("Optional input file (defaults to stdin)"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .default_value(None)
                .help("Optional output file (defaults to stdout)"),
        )
        .get_matches();

    // Handle input stream (file or stdin)
    let input: Box<dyn Read> = if let Some(input_path) = matches.get_one::<String>("input") {
        Box::new(File::open(input_path)?)
    } else {
        Box::new(io::stdin())
    };

    // Handle output stream (file or stdout)
    let mut output: Box<dyn Write> = if let Some(output_path) = matches.get_one::<String>("output")
    {
        Box::new(File::create(output_path)?)
    } else {
        Box::new(io::stdout())
    };

    BencodeParser::new(input).write_bytes(&mut output)?;

    Ok(())
}
