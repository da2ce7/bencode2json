use std::io;

/* code-review:

   The function `write_byte` only writes:

   - Bytes used in integers:
     - Digits: '0','1','2','3','4','5','6','7','8','9'
     - 'e', '-'
   - JSON reservers chars: '[', ',', ']', '{', ',', ':', '}' defined as constants.

   It could be refactored to be more restrictive. However, in the future we also
   want to print Bencoded strings as bytes streams, without trying to convert
   them into UTF-8 strings.
*/

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
