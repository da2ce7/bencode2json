use core::str;
use std::io::{self, Read};

/// A reader that reads bytes from an input.
///
/// It's wrapper of a basic reader with extra functionality.
pub struct ByteReader<R: Read> {
    /// Number of bytes read from the input.
    pub input_byte_counter: u64,

    /// It optionally captures the input.
    pub opt_captured_input: Option<Vec<u8>>,

    reader: R,
}

impl<R: Read> ByteReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            input_byte_counter: 0,
            opt_captured_input: Some(Vec::new()),
            reader,
        }
    }

    /// It reads one byte from the input (stdin or file).
    ///
    /// # Errors
    ///
    /// Will return an error if it can't read the byte from the input.
    pub fn read_byte(&mut self) -> io::Result<u8> {
        let mut byte = [0; 1];

        self.reader.read_exact(&mut byte)?;

        self.input_byte_counter += 1;

        let byte = byte[0];

        if let Some(ref mut captured_input) = self.opt_captured_input {
            captured_input.push(byte);
        }

        Ok(byte)
    }

    /// It prints the captured input is enabled.
    ///
    /// It will print a string it the captured input so far is a UTF-8 string,
    /// the debug info otherwise.
    pub fn print_captured_input(&self) {
        if let Some(input) = &self.opt_captured_input {
            match str::from_utf8(input) {
                Ok(string) => println!("input: {string}"),
                Err(_) => println!("input: {input:#?}"),
            }
        }
    }
}
