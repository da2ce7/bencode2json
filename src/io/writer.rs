use std::io;

pub trait Writer {
    /// It writes one byte to the output.
    ///
    /// # Errors
    ///
    /// Will return an error if it can't write the byte.
    fn write_byte(&mut self, byte: u8) -> io::Result<()>;

    /// It writes a string to the output.
    ///
    /// # Errors
    ///
    /// Will return an error if it can't write the string.
    fn write_str(&mut self, value: &str) -> io::Result<()>;

    /// It gets the captured output if enabled.
    fn get_captured_output(&mut self) -> Option<String>;

    /// It prints the captured output if enabled.
    fn print_captured_output(&self);
}
