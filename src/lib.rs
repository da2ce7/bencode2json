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
    stack: VecDeque<JsonValue>,
}

impl<R: Read> BencodeParser<R> {
    pub fn new(reader: R) -> Self {
        BencodeParser {
            reader,
            stack: VecDeque::new(),
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
    pub fn parse(&mut self) -> io::Result<JsonValue> {
        let mut data = Vec::new();

        self.reader.read_to_end(&mut data)?;

        let mut i = 0;

        //println!("buffer: {data:#?}");

        while i < data.len() {
            //print!(" {}", data[i]);

            match data[i] {
                b'i' => {
                    i += 1;
                    let end_idx = data[i..].iter().position(|&x| x == b'e').unwrap() + i;
                    let num = std::str::from_utf8(&data[i..end_idx])
                        .unwrap()
                        .parse::<i64>()
                        .unwrap();
                    self.stack.push_back(JsonValue::Integer(num));
                    i = end_idx + 1;
                }
                b'l' => {
                    self.stack.push_back(JsonValue::List(vec![]));
                    i += 1;
                }
                b'd' => {
                    self.stack.push_back(JsonValue::Dictionary(vec![]));
                    i += 1;
                }
                b'e' => {
                    if let Some(JsonValue::List(list)) = self.stack.pop_back() {
                        if let Some(JsonValue::List(parent_list)) = self.stack.back_mut() {
                            parent_list.push(JsonValue::List(list));
                        } else {
                            self.stack.push_back(JsonValue::List(list));
                        }
                    }
                    i += 1;
                }
                b'0'..=b'9' => {
                    let colon_pos = data[i..].iter().position(|&x| x == b':').unwrap() + i;
                    let length = std::str::from_utf8(&data[i..colon_pos])
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    let start = colon_pos + 1;
                    let end = start + length;

                    let bytes = &data[start..end];
                    if is_valid_utf8(bytes) {
                        self.stack.push_back(JsonValue::String(
                            String::from_utf8(bytes.to_vec()).unwrap(),
                        ));
                    } else {
                        self.stack
                            .push_back(JsonValue::BinaryData(bytes_to_hex(bytes)));
                    }
                    i = end;
                }
                10 => {
                    // Ignore New Line byte (NL)
                    i += 1;
                }
                _ => panic!("Unknown token"),
            }
        }

        Ok(self.stack.pop_back().unwrap())
    }
}

fn is_valid_utf8(data: &[u8]) -> bool {
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
        assert_eq!(result, JsonValue::String("spam".to_string()));
    }

    #[test]
    fn test_non_utf8_string() {
        let data = b"4:\xFF\xFE\xFD\xFC";
        let mut parser = BencodeParser::new(&data[..]);
        let result = parser.parse().unwrap();
        assert_eq!(
            result,
            JsonValue::BinaryData("<hex>fffefdfc</hex>".to_string())
        );
    }

    #[test]
    fn test_integer() {
        let data = b"i42e";
        let mut parser = BencodeParser::new(&data[..]);
        let result = parser.parse().unwrap();
        assert_eq!(result, JsonValue::Integer(42));
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
