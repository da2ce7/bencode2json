use std::io::{self, Read, Write};

use crate::{byte_reader::ByteReader, byte_writer::ByteWriter};

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
    /*
    st = 0 -> Parsed begin integer (`i`)
    st = 1 -> Parsed sign (only negative is allowed)
    st = 2 -> Parsing digits
    st = 3 -> Parsed end integer (`e`)
    */

    let mut st = 0;

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
            st = 2;
            writer.write_byte(byte)?;
        } else if char == 'e' && st == 2 {
            return Ok(());
        } else if char == '-' && st == 0 {
            st = 1;
            writer.write_byte(byte)?;
        } else {
            panic!("invalid integer");
        }
    }
}
