use core::str;
use std::{fmt::Write, io};

use super::writer::Writer;

/// A writer that writes chars to an output.
///
/// It's wrapper of a basic writer with extra functionality.
pub struct StringWriter<W: Write> {
    /// Number of bytes write to the output.
    pub output_string_length: u64,

    /// It optionally captures the output.
    pub opt_captured_output: Option<String>,

    writer: W,
}

impl<W: Write> StringWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            output_string_length: 0,
            opt_captured_output: Some(String::new()),
            writer,
        }
    }
}

impl<W: Write> Writer for StringWriter<W> {
    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        let c = byte as char;

        self.writer.write_char(c).expect("error writing str");

        self.output_string_length += 1;

        if let Some(ref mut captured_output) = self.opt_captured_output {
            captured_output.push(c);
        }

        Ok(())
    }

    fn write_str(&mut self, value: &str) -> io::Result<()> {
        self.writer.write_str(value).expect("error writing str");

        self.output_string_length += value.len() as u64;

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
