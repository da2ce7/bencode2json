use std::io::{self, Read, Write};

use crate::io::{byte_reader::ByteReader, byte_writer::ByteWriter};

/// The current state parsing the integer.
#[derive(PartialEq)]
#[allow(clippy::enum_variant_names)]
enum StateExpecting {
    DigitOrSign,    // 0
    DigitAfterSign, // 1
    DigitOrEnd,     // 2
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
/// Will panic if we reach the end of the input without completing the integer
/// (without reaching the end of the integer `e`).
pub fn parse<R: Read, W: Write>(
    reader: &mut ByteReader<R>,
    writer: &mut ByteWriter<W>,
) -> io::Result<()> {
    let mut state = StateExpecting::DigitOrSign;

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
            state = StateExpecting::DigitOrEnd;
            writer.write_byte(byte)?;
        } else if char == 'e' && state == StateExpecting::DigitOrEnd {
            return Ok(());
        } else if char == '-' && state == StateExpecting::DigitOrSign {
            state = StateExpecting::DigitAfterSign;
            writer.write_byte(byte)?;
        } else {
            panic!("invalid integer");
        }
    }
}
