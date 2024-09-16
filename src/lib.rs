use std::collections::VecDeque;
use std::io::{self, Read};
use std::str;

#[derive(Debug, PartialEq)]
pub enum JsonValue {
    String(String),
    Integer(i64),
    List(Vec<JsonValue>),
    Dictionary(Vec<(String, JsonValue)>),
    BinaryData(String), // For non-UTF8 strings
}

pub struct BencodeParser<R: Read> {
    reader: R,
    _stack: VecDeque<JsonValue>,
    output: String,
}

impl<R: Read> BencodeParser<R> {
    pub fn new(reader: R) -> Self {
        BencodeParser {
            reader,
            _stack: VecDeque::new(),
            output: String::new(),
        }
    }

    /// # Errors
    ///
    ///
    ///
    /// # Panics
    ///
    /// Will panic if ...
    #[allow(clippy::match_on_vec_items)]
    pub fn parse(&mut self) -> io::Result<String> {
        let byte = self.read_byte()?;

        match byte {
            b'i' => {
                // Read bytes until we find the end of the integer
                loop {
                    let byte = self.read_byte()?;
                    if byte == b'e' {
                        break;
                    }
                    self.output.push(byte as char);
                }
            }
            b'0'..=b'9' => {
                // Read bytes until we find the end of the integer
                let mut length_bytes = Vec::new();

                length_bytes.push(byte);

                loop {
                    let byte = self.read_byte()?;
                    if byte == b':' {
                        break;
                    }
                    length_bytes.push(byte);
                }

                //println!("length_bytes: {length_bytes:#?}");

                let length_str = str::from_utf8(&length_bytes).unwrap();

                //println!("length_str: {length_str}");

                let length = length_str.parse::<usize>().unwrap();

                //println!("length: {length}");

                // Read "length" bytes until the end of the string
                let string_bytes = self.read_n_bytes(length)?;

                let string = match str::from_utf8(&string_bytes) {
                    Ok(string) => string,
                    Err(_) => &bytes_to_hex(&string_bytes),
                };

                //println!("utf8 string: {string}");

                self.output.push_str(string);
            }
            /*
            b'l' => {}
            b'd' => {}
            b'e' => {}
            b'0'..=b'9' => {}
            10 => {
                // Ignore New Line byte (NL)
            }
             */
            _ => panic!("Unknown token"),
        }

        Ok(self.output.clone())
    }

    fn read_byte(&mut self) -> io::Result<u8> {
        let mut byte = [0; 1];
        self.reader.read_exact(&mut byte)?;
        Ok(byte[0])
    }

    fn read_n_bytes(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();

        for _i in 1..=n {
            bytes.push(self.read_byte()?);
        }

        Ok(bytes)
    }
}

fn _is_valid_utf8(data: &[u8]) -> bool {
    str::from_utf8(data).is_ok()
}

fn bytes_to_hex(data: &[u8]) -> String {
    format!("<hex>{}</hex>", hex::encode(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_string() {
        let data = b"4:spam";
        let mut parser = BencodeParser::new(&data[..]);
        let result = parser.parse().unwrap();
        assert_eq!(result, "spam".to_string());
    }

    #[test]
    fn test_non_utf8_string() {
        let data = b"4:\xFF\xFE\xFD\xFC";
        let mut parser = BencodeParser::new(&data[..]);
        let result = parser.parse().unwrap();
        assert_eq!(result, "<hex>fffefdfc</hex>".to_string());
    }

    #[test]
    fn test_integer() {
        let data = b"i42e";
        let mut parser = BencodeParser::new(&data[..]);
        let result = parser.parse().unwrap();
        assert_eq!(result, "42".to_string());
    }

    /* Not implemented
    #[test]
    fn test_list() {
        let data = b"l4:spam4:eggse";
        let mut parser = BencodeParser::new(&data[..]);
        let result = parser.parse().unwrap();
        assert_eq!(
            result,
            JsonValue::List(vec![
                JsonValue::String("spam".to_string()),
                JsonValue::String("eggs".to_string())
            ])
        );
    }
     */
}
