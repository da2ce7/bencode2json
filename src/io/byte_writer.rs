use core::str;
use std::io::{self, Write};

use super::writer::Writer;

/// A writer that writes bytes to an output.
///
/// It's wrapper of a basic writer with extra functionality.
pub struct ByteWriter<W: Write> {
    /// Number of bytes write to the output.
    pub output_byte_counter: u64,

    /// It optionally captures the output.
    pub opt_captured_output: Option<String>,

    writer: W,
}

impl<W: Write> ByteWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            output_byte_counter: 0,
            opt_captured_output: Some(String::new()),
            writer,
        }
    }
}

impl<W: Write> Writer for ByteWriter<W> {
    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        let bytes = [byte];

        self.writer.write_all(&bytes)?;

        self.output_byte_counter += 1;

        if let Some(ref mut captured_output) = self.opt_captured_output {
            captured_output.push(byte as char);
        }

        Ok(())
    }

    fn write_str(&mut self, value: &str) -> io::Result<()> {
        self.writer.write_all(value.as_bytes())?;

        self.output_byte_counter += value.as_bytes().len() as u64;

        if let Some(ref mut captured_output) = self.opt_captured_output {
            captured_output.push_str(value);
        }

        Ok(())
    }

    fn get_captured_output(&mut self) -> Option<String> {
        match &self.opt_captured_output {
            Some(output) => Some(output.to_string()),
            None => todo!(),
        }
    }

    fn print_captured_output(&self) {
        if let Some(output) = &self.opt_captured_output {
            println!("output: {output}");
        }
    }
}
