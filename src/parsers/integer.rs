use std::io::{self, Read, Write};

use crate::{byte_reader::ByteReader, byte_writer::ByteWriter};

#[derive(PartialEq)]
enum ExpectingDigit {
    OrSign,    // 0
    AfterSign, // 1
    OrEnd,     // 2
}

/// It parses an integer bencoded value.
///
/// # Errors
///
/// Will return an error if it can't read from the input or write to the
/// output.
///
/// # Panics
///
/// Will panic if we reach the end of the input without completing the
/// integer (without reaching the end of the integer `e`).
pub fn parse<R: Read, W: Write>(
    reader: &mut ByteReader<R>,
    writer: &mut ByteWriter<W>,
) -> io::Result<()> {
    let mut state = ExpectingDigit::OrSign;

    loop {
        let byte = match reader.read_byte() {
            Ok(byte) => byte,
            Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                panic!("unexpected end of input parsing integer");
            }
            Err(err) => return Err(err),
        };

        let char = byte as char;

        if char.is_ascii_digit() {
            state = ExpectingDigit::OrEnd;
            writer.write_byte(byte)?;
        } else if char == 'e' && state == ExpectingDigit::OrEnd {
            return Ok(());
        } else if char == '-' && state == ExpectingDigit::OrSign {
            state = ExpectingDigit::AfterSign;
            writer.write_byte(byte)?;
        } else {
            panic!("invalid integer");
        }
    }
}
