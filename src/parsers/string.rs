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
    initial_byte: u8,
) -> io::Result<()> {
    let mut string_parser = StringParser::default();
    string_parser.parse(reader, writer, initial_byte)
}

/// Strings bencode format have two parts: `length:value`.
///
/// - Length is a sequence of bytes (only digits 0..9).
/// - Value is an arbitrary sequence of bytes (not only valid UTF-8).
#[derive(Default, Debug)]
#[allow(clippy::module_name_repetitions)]
struct StringParser {
    length: Length,
    value: Value,
}

#[derive(Default, Debug)]
struct Length {
    bytes: Vec<u8>,
    number: usize,
}

impl Length {
    fn add_byte(&mut self, byte: u8) {
        // todo: should we fail here is the byte is not a digit (0..9)?
        // or we can wait until we try to convert all bytes in the into a number?
        self.bytes.push(byte);
    }

    /// This function convert the current bytes representing the length to a
    /// number.
    ///
    /// # Panics
    ///
    /// Will panic if the length bytes contain invalid UTF-8 chars or don't
    /// represent a valid zero or positive integer.
    fn convert_to_number(&mut self) -> usize {
        // todo: maybe we should simply fail when we receive a byte that is not a digit (0..9).
        // This error cannot be understood by users because we first convert into a UTF-8 string
        // and later into a number.
        let length_str =
            str::from_utf8(&self.bytes).expect("invalid string length, non UTF-8 string length");

        self.number = length_str
            .parse::<usize>()
            .expect("invalid string length, non zero or positive integer");

        self.number
    }
}

#[derive(Default, Debug)]
struct Value {
    bytes: Vec<u8>,
    bytes_counter: usize,
}

impl Value {
    fn add_byte(&mut self, byte: u8) {
        // todo: return an error if we try to push a new byte but the end of the
        // string has been reached.
        self.bytes.push(byte);
        self.bytes_counter += 1;
    }

    fn utf8(&self) -> String {
        match str::from_utf8(&self.bytes) {
            Ok(string) => {
                // String only contains valid UTF-8 chars -> print it as it's
                string.to_owned()
            }
            Err(_) => {
                // String contains non valid UTF-8 chars -> print it as hex bytes
                Self::bytes_to_hex(&self.bytes)
            }
        }
    }

    fn bytes_to_hex(data: &[u8]) -> String {
        format!("<hex>{}</hex>", hex::encode(data))
    }
}

impl StringParser {
    fn parse<R: Read, W: Write>(
        &mut self,
        reader: &mut ByteReader<R>,
        writer: &mut ByteWriter<W>,
        initial_byte: u8,
    ) -> io::Result<()> {
        self.parse_length(reader, initial_byte)?;

        self.parse_value(reader)?;

        writer.write_str(&self.json())?;

        Ok(())
    }

    fn parse_length<R: Read>(
        &mut self,
        reader: &mut ByteReader<R>,
        initial_byte: u8,
    ) -> io::Result<()> {
        // code-review: length can be calculated on the fly as the original C implementation.

        self.length.add_byte(initial_byte);

        loop {
            let byte = match reader.read_byte() {
                Ok(byte) => byte,
                Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                    //println!("Reached the, byte end of file.");
                    panic!("unexpected end of input parsing string length");
                }
                Err(err) => return Err(err),
            };

            match byte {
                b':' => {
                    // End of string length
                    self.process_end_of_string_length();
                    self.length.convert_to_number();
                    break;
                }
                _ => {
                    self.length.add_byte(byte);
                }
            }
        }

        Ok(())
    }

    fn parse_value<R: Read>(&mut self, reader: &mut ByteReader<R>) -> io::Result<()> {
        for _i in 1..=self.length.number {
            let byte = match reader.read_byte() {
                Ok(byte) => byte,
                Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                    //println!("Reached the end of file.");
                    panic!("unexpected end of input parsing string value");
                }
                Err(err) => return Err(err),
            };

            self.value.add_byte(byte);

            // todo: escape '"' and '\\' with '\\';
        }

        Ok(())
    }

    /// This function is called when we receive the ':' byte which is the
    /// delimiter for the end of bytes representing the string length.
    fn process_end_of_string_length(&mut self) -> usize {
        self.length.convert_to_number()
    }

    fn utf8(&self) -> String {
        self.value.utf8()
    }

    #[must_use]
    fn json(&self) -> String {
        format!("\"{}\"", self.utf8())
    }
}
