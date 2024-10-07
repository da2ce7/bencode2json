use std::io;

pub trait Writer {
    /// It writes one byte to the output (stdout or file).
    ///
    /// # Errors
    ///
    /// Will return an error if it can't write the byte to the output.
    fn write_byte(&mut self, byte: u8) -> io::Result<()>;

    /// It writes a string to the output (stdout or file).
    ///
    /// # Errors
    ///
    /// Will return an error if it can't write the string (as bytes) to the output.
    fn write_str(&mut self, value: &str) -> io::Result<()>;

    /// It gets the captured output is enabled.
    fn get_captured_output(&mut self) -> Option<String>;

    /// It prints the captured output is enabled.
    fn print_captured_output(&self);
}
