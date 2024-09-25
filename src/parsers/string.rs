//! Bencoded string parser.
//!
//! It reads bencoded bytes from the input and writes JSON bytes to the output.
use std::io::{self, Read, Write};

use crate::io::{byte_reader::ByteReader, byte_writer::ByteWriter};

// todo: return errors instead of panicking in StringParser.

use core::str;

/// It parses a string bencoded value.
///
/// # Errors
///
/// Will return an error if it can't read from the input or write to the
/// output.
///
/// # Panics
///
/// Will panic if we reach the end of the input without completing the string.
pub fn parse<R: Read, W: Write>(
    reader: &mut ByteReader<R>,
    writer: &mut ByteWriter<W>,
    byte: u8,
) -> io::Result<()> {
    let mut string_parser = StringParser::default();

    string_parser.new_string_starting_with(byte);

    // Parse length

    loop {
        let byte = match reader.read_byte() {
            Ok(byte) => byte,
            Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                //println!("Reached the end of file.");
                panic!("unexpected end of input parsing string length");
            }
            Err(err) => return Err(err),
        };

        match byte {
            b':' => {
                // End of string length
                string_parser.process_end_of_string_length();
                break;
            }
            _ => {
                string_parser.add_length_byte(byte);
            }
        }
    }

    // Parse value

    for _i in 1..=string_parser.string_length {
        let byte = match reader.read_byte() {
            Ok(byte) => byte,
            Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                //println!("Reached the end of file.");
                panic!("unexpected end of input parsing string chars");
            }
            Err(err) => return Err(err),
        };

        string_parser.add_byte(byte);

        // todo: escape '"' and '\\' with '\\';
    }

    writer.write_str(&string_parser.json())?;

    //println!("string_parser {string_parser:#?}");

    Ok(())
}

#[derive(Default, Debug)]
#[allow(clippy::module_name_repetitions)]
struct StringParser {
    // String length
    bytes_for_string_length: Vec<u8>,
    string_length: usize,

    // String value bytes
    string_bytes: Vec<u8>,
    string_bytes_counter: usize,
}

impl StringParser {
    pub fn new_string_starting_with(&mut self, byte: u8) {
        self.new_string();
        self.add_length_byte(byte);
    }

    pub fn add_length_byte(&mut self, byte: u8) {
        // todo: should we fail here is the byte is not a digit (0..9)?
        // or we can wait until we try to convert all bytes in the into a number?
        self.bytes_for_string_length.push(byte);
    }

    pub fn add_byte(&mut self, byte: u8) {
        // todo: return an error if we try to push a new byte but the end of the
        // string has been reached.
        self.string_bytes.push(byte);
        self.string_bytes_counter += 1;
    }

    /// This function is called when we receive the ':' byte which is the
    /// delimiter for the end of bytes representing the string length.
    ///
    /// # Panics
    ///
    /// Will panic if the length bytes contain invalid UTF-8 chars or don't
    /// represent a valid zero or positive integer.
    pub fn process_end_of_string_length(&mut self) {
        // todo: maybe we should simply fail when we receive a byte that is not a digit (0..9).
        // This error cannot be understood by users because we first convert into a UTF-8 string
        // and later into a number.
        let length_str = str::from_utf8(&self.bytes_for_string_length)
            .expect("invalid string length, non UTF-8 string length");

        //println!("length_str: {length_str}");

        self.string_length = length_str
            .parse::<usize>()
            .expect("invalid string length, non zero or positive integer");

        //println!("string_length_number: {string_length}");
    }

    fn utf8(&self) -> String {
        match str::from_utf8(&self.string_bytes) {
            Ok(string) => {
                // String only contains valid UTF-8 chars -> print it as it's
                string.to_owned()
            }
            Err(_) => {
                // String contains non valid UTF-8 chars -> print it as hex bytes
                Self::bytes_to_hex(&self.string_bytes)
            }
        }
    }

    #[must_use]
    pub fn json(&self) -> String {
        format!("\"{}\"", self.utf8())
    }

    fn new_string(&mut self) {
        self.bytes_for_string_length = Vec::new();
        self.string_length = 0;
        self.string_bytes = Vec::new();
        self.string_bytes_counter = 0;
    }

    fn bytes_to_hex(data: &[u8]) -> String {
        format!("<hex>{}</hex>", hex::encode(data))
    }
}
