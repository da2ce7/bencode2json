use core::str;
use std::fmt::{self, Write};

/// A writer that writes chars to an output.
///
/// It's wrapper of a basic writer with extra functionality.
pub struct CharWriter<W: Write> {
    /// Number of bytes write to the output.
    pub output_char_counter: u64,

    /// It optionally captures the output.
    pub opt_captured_output: Option<String>,

    writer: W,
}

impl<W: Write> CharWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            output_char_counter: 0,
            opt_captured_output: Some(String::new()),
            writer,
        }
    }

    /// It writes one byte to the output (stdout or file).
    ///
    /// # Errors
    ///
    /// Will return an error if it can't write the byte to the output.
    pub fn write_char(&mut self, c: char) -> fmt::Result {
        self.writer.write_char(c)?;

        self.output_char_counter += 1;

        if let Some(ref mut captured_output) = self.opt_captured_output {
            captured_output.push(c);
        }

        Ok(())
    }

    /// It writes a string to the output (stdout or file).
    ///
    /// # Errors
    ///
    /// Will return an error if it can't write the string (as bytes) to the output.
    pub fn write_str(&mut self, value: &str) -> fmt::Result {
        self.writer.write_str(value)?;

        self.output_char_counter += value.len() as u64;

        if let Some(ref mut captured_output) = self.opt_captured_output {
            captured_output.push_str(value);
        }

        Ok(())
    }

    /// It prints the captured output is enabled.
    pub fn print_captured_output(&self) {
        if let Some(output) = &self.opt_captured_output {
            println!("output: {output}");
        }
    }
}
