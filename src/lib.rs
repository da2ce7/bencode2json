use std::io::{self, Read};
use std::str;

/*
#[derive(Debug, PartialEq)]
pub enum JsonValue {
    String(String),
    Integer(i64),
    List(Vec<JsonValue>),
    Dictionary(Vec<(String, JsonValue)>),
    BinaryData(String), // For non-UTF8 strings
}
    */

#[derive(Debug, PartialEq)]
pub enum State {
    ParsingInteger,
    ParsingString(ParsingString),
    ParsingList(ParsingList),
    ParsingDictionary(ParsingDictionary),
}

#[derive(Debug, PartialEq)]
pub enum ParsingString {
    ParsingLength,
    ParsingChars,
}

/// l y m
#[derive(Debug, PartialEq)]
pub enum ParsingList {
    Start, // l
    Rest,  // m
}

#[derive(Debug, PartialEq)]
pub enum ParsingDictionary {
    Start,        // d
    ExpectingKey, // e
    EndKeyValue,  // f
}

pub struct BencodeParser<R: Read> {
    reader: R,
    stack: Vec<State>,
    pub output: String,
    pub pos: u64,
}

impl<R: Read> BencodeParser<R> {
    pub fn new(reader: R) -> Self {
        BencodeParser {
            reader,
            stack: Vec::new(),
            output: String::new(),
            pos: 0,
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
    #[allow(clippy::single_match)]
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::match_same_arms)]
    #[allow(clippy::single_match_else)]
    pub fn parse(&mut self) -> io::Result<()> {
        let mut iter = 1;

        let mut bytes_for_string_length = Vec::new();
        let mut string_length = 0;
        let mut string_bytes = Vec::new();
        let mut string_bytes_counter = 0;

        loop {
            let byte = match self.read_byte() {
                Ok(byte) => byte,
                Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                    //println!("Reached the end of file.");
                    break; // Handle EOF gracefully, exit the loop
                }
                Err(err) => return Err(err),
            };

            println!("iter: {iter}");
            println!("pos: {}", self.pos);
            println!("byte: {} ({})", byte, byte as char);
            println!("stack: {:#?}", self.stack);
            println!("bytes_for_string_length: {bytes_for_string_length:#?}");
            println!("string_length: {string_length}");
            println!("string_bytes: {string_bytes:#?}");
            println!("string_bytes_counter: {string_bytes_counter}");
            println!();

            match byte {
                b'i' => {
                    // State machine

                    match self.stack.last() {
                        Some(head) => match head {
                            State::ParsingList(parsing_list) => match parsing_list {
                                ParsingList::Start => {
                                    self.stack.push(State::ParsingInteger);
                                }
                                ParsingList::Rest => {
                                    self.stack.push(State::ParsingInteger);
                                    self.output.push(',');
                                }
                            },
                            State::ParsingDictionary(_) => {
                                panic!("invalid byte, expected list item")
                            }
                            State::ParsingInteger => {
                                panic!("invalid byte, parsing integer expected digit")
                            }
                            State::ParsingString(parsing_string) => match parsing_string {
                                ParsingString::ParsingLength => {
                                    panic!("unexpected byte 'i', parsing string length ")
                                }
                                ParsingString::ParsingChars => {
                                    string_bytes.push(byte);
                                    string_bytes_counter += 1;
                                    if string_bytes_counter == string_length {
                                        // We have finishing capturing the string bytes

                                        let string = match str::from_utf8(&string_bytes) {
                                            Ok(string) => string,
                                            Err(_) => {
                                                // String contains non valid UTF-8 chars -> print as hex bytes list
                                                &bytes_to_hex(&string_bytes)
                                            }
                                        };

                                        self.output.push_str(&format!("\"{string}\""));
                                        self.stack.pop();
                                    }
                                }
                            },
                        },
                        None => {
                            self.stack.push(State::ParsingInteger);
                        }
                    }
                }
                b'0'..=b'9' => {
                    // State machine

                    match self.stack.last() {
                        Some(state) => match state {
                            State::ParsingInteger => {
                                self.output.push(byte as char);
                            }
                            State::ParsingString(parsing_string) => match parsing_string {
                                ParsingString::ParsingLength => {
                                    // Add a digit for the string length
                                    bytes_for_string_length.push(byte);
                                }
                                ParsingString::ParsingChars => {
                                    string_bytes.push(byte);
                                    string_bytes_counter += 1;
                                    if string_bytes_counter == string_length {
                                        // We have finishing capturing the string bytes

                                        let string = match str::from_utf8(&string_bytes) {
                                            Ok(string) => string,
                                            Err(_) => {
                                                // String contains non valid UTF-8 chars -> print as hex bytes list
                                                &bytes_to_hex(&string_bytes)
                                            }
                                        };

                                        self.output.push_str(&format!("\"{string}\""));
                                        self.stack.pop();
                                    }
                                }
                            },
                            State::ParsingList(parsing_list) => {
                                match parsing_list {
                                    ParsingList::Start => {
                                        // First item in the list and it is a string

                                        // New string -> reset current string being parsed
                                        bytes_for_string_length = Vec::new();
                                        string_length = 0;
                                        string_bytes = Vec::new();
                                        string_bytes_counter = 0;

                                        bytes_for_string_length.push(byte);

                                        self.stack.push(State::ParsingString(
                                            ParsingString::ParsingLength,
                                        ));
                                    }
                                    ParsingList::Rest => {
                                        // Non first item in the list and it is a string

                                        // New string -> reset current string being parsed
                                        bytes_for_string_length = Vec::new();
                                        string_length = 0;
                                        string_bytes = Vec::new();
                                        string_bytes_counter = 0;

                                        bytes_for_string_length.push(byte);

                                        self.stack.push(State::ParsingString(
                                            ParsingString::ParsingLength,
                                        ));
                                    }
                                }
                            }
                            State::ParsingDictionary(_parsing_dictionary) => todo!(),
                        },
                        None => {
                            // First byte in input
                            self.stack
                                .push(State::ParsingString(ParsingString::ParsingLength));
                            bytes_for_string_length.push(byte);
                        }
                    };

                    /*
                    // Parse int

                    // Read bytes until we find the end of the string length
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

                    self.output.push_str(&format!("\"{string}\""));
                    */
                }
                b':' => match self.stack.last() {
                    Some(state) => match state {
                        State::ParsingString(parsing_string) => {
                            match parsing_string {
                                ParsingString::ParsingLength => {
                                    // We reach the end of the string length
                                    let length_str = str::from_utf8(&bytes_for_string_length)
                                        .expect("non UTF8 string length");
                                    println!("length_str: {length_str}");

                                    string_length = length_str
                                        .parse::<usize>()
                                        .expect("invalid number for string length");
                                    println!("string_length_number: {string_length}");

                                    // todo: is string with size 0 (empty string) allowed in bencode?

                                    self.stack.pop();
                                    self.stack
                                        .push(State::ParsingString(ParsingString::ParsingChars));
                                }
                                ParsingString::ParsingChars => {
                                    string_bytes.push(byte);
                                    string_bytes_counter += 1;
                                    if string_bytes_counter == string_length {
                                        // We have finishing capturing the string bytes

                                        let string = match str::from_utf8(&string_bytes) {
                                            Ok(string) => string,
                                            Err(_) => {
                                                // String contains non valid UTF-8 chars -> print as hex bytes list
                                                &bytes_to_hex(&string_bytes)
                                            }
                                        };

                                        self.output.push_str(&format!("\"{string}\""));
                                    }
                                }
                            }
                        }
                        _ => panic!("unexpected byte: ':', not parsing a string"),
                    },
                    None => {
                        panic!("unexpected byte: ':', not parsing a string");
                    }
                },
                b'l' => {
                    match self.stack.last() {
                        Some(head) => match head {
                            State::ParsingList(parsing_list) => match parsing_list {
                                ParsingList::Start => {
                                    self.stack.push(State::ParsingList(ParsingList::Rest));
                                }
                                ParsingList::Rest => {}
                            },
                            State::ParsingDictionary(_) => {
                                panic!("invalid byte, expected list item")
                            }
                            State::ParsingInteger => {}
                            State::ParsingString(parsing_string) => match parsing_string {
                                ParsingString::ParsingLength => {
                                    panic!("unexpected byte: 'l', parsing string length")
                                }
                                ParsingString::ParsingChars => {
                                    string_bytes.push(byte);
                                    string_bytes_counter += 1;
                                    if string_bytes_counter == string_length {
                                        // We have finishing capturing the string bytes

                                        let string = match str::from_utf8(&string_bytes) {
                                            Ok(string) => string,
                                            Err(_) => {
                                                // String contains non valid UTF-8 chars -> print as hex bytes list
                                                &bytes_to_hex(&string_bytes)
                                            }
                                        };

                                        self.output.push_str(&format!("\"{string}\""));
                                    }
                                }
                            },
                        },
                        None => self.stack.push(State::ParsingList(ParsingList::Start)),
                    }

                    self.output.push('[');

                    //self.parse()?;
                }
                b'e' => {
                    match self.stack.last() {
                        Some(head) => match head {
                            State::ParsingList(_) => {
                                self.stack.pop();
                                self.output.push(']');
                            }
                            State::ParsingDictionary(_) => {
                                panic!("invalid byte, expected list item")
                            }
                            State::ParsingInteger => {
                                // We have finished parsing the integer
                                self.stack.pop();
                            }
                            State::ParsingString(parsing_string) => match parsing_string {
                                ParsingString::ParsingLength => {
                                    panic!("unexpected byte: 'e', parsing string length")
                                }
                                ParsingString::ParsingChars => {
                                    string_bytes.push(byte);
                                    string_bytes_counter += 1;
                                    if string_bytes_counter == string_length {
                                        // We have finishing parsing the integer

                                        let string = match str::from_utf8(&string_bytes) {
                                            Ok(string) => string,
                                            Err(_) => {
                                                // String contains non valid UTF-8 chars -> print as hex bytes list
                                                &bytes_to_hex(&string_bytes)
                                            }
                                        };

                                        self.output.push_str(&format!("\"{string}\""));
                                        self.stack.pop();
                                    }
                                }
                            },
                        },
                        None => panic!("invalid byte, unexpected end byte `e`"),
                    }

                    match self.stack.last() {
                        Some(state) => match state {
                            State::ParsingList(parsing_list) => match parsing_list {
                                ParsingList::Start => {
                                    self.stack.pop();
                                    self.stack.push(State::ParsingList(ParsingList::Rest));
                                }
                                ParsingList::Rest => {}
                            },
                            State::ParsingInteger => {}
                            State::ParsingString(_parsing_string) => {}
                            State::ParsingDictionary(_parsing_dictionary) => {}
                        },
                        None => {}
                    }
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
                _ => {
                    match self.stack.last() {
                        Some(head) => match head {
                            State::ParsingList(_) => {}
                            State::ParsingDictionary(_) => {}
                            State::ParsingInteger => {}
                            State::ParsingString(parsing_string) => match parsing_string {
                                ParsingString::ParsingLength => {}
                                ParsingString::ParsingChars => {
                                    string_bytes.push(byte);
                                    string_bytes_counter += 1;
                                    if string_bytes_counter == string_length {
                                        // We have finishing parsing the integer

                                        let string = match str::from_utf8(&string_bytes) {
                                            Ok(string) => string,
                                            Err(_) => {
                                                // String contains non valid UTF-8 chars -> print as hex bytes list
                                                &bytes_to_hex(&string_bytes)
                                            }
                                        };

                                        self.output.push_str(&format!("\"{string}\""));
                                        self.stack.pop();
                                    }
                                }
                            },
                        },
                        None => {}
                    }
                }
            }

            iter += 1;
        }

        Ok(())
    }

    fn read_byte(&mut self) -> io::Result<u8> {
        let mut byte = [0; 1];
        self.reader.read_exact(&mut byte)?;
        self.pos += 1;
        Ok(byte[0])
    }

    fn _read_n_bytes(&mut self, n: usize) -> io::Result<Vec<u8>> {
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
    fn integer() {
        let data = b"i42e";
        let mut parser = BencodeParser::new(&data[..]);
        parser.parse().unwrap();
        assert_eq!(parser.output, "42".to_string());
    }

    #[test]
    fn utf8_string() {
        let data = b"4:spam";
        let mut parser = BencodeParser::new(&data[..]);
        parser.parse().unwrap();
        assert_eq!(parser.output, "\"spam\"".to_string());
    }

    #[test]
    fn non_utf8_string() {
        let data = b"4:\xFF\xFE\xFD\xFC";
        let mut parser = BencodeParser::new(&data[..]);
        parser.parse().unwrap();
        assert_eq!(parser.output, "\"<hex>fffefdfc</hex>\"".to_string());
    }

    mod lists {
        use crate::BencodeParser;

        #[test]
        fn empty_list() {
            let data = b"le";
            let mut parser = BencodeParser::new(&data[..]);
            parser.parse().unwrap();
            assert_eq!(parser.output, "[]".to_string());
        }

        mod with_one_item {
            use crate::BencodeParser;

            #[test]
            fn with_one_integer() {
                let data = b"li42ee";
                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();
                assert_eq!(parser.output, "[42]".to_string());
            }

            #[test]
            fn with_one_utf8_string() {
                // List with one UTF8 string: l4:spame
                //   1   2   3   4   5   6   7   8 (pos)
                //   l   4   :   s   p   a   m   e (byte)
                // 108  52  58 115 112  97 109 101 (byte decimal)

                let data = b"l4:spame";
                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();
                assert_eq!(parser.output, "[\"spam\"]".to_string());
            }
        }

        mod with_two_items {
            use crate::BencodeParser;

            #[test]
            fn with_two_integers() {
                // List with two integers: li42ei43ee
                //   1   2   3   4   5   6   7   8   9  10 (pos)
                //   l   i   4   2   e   i   4   3   e   e (byte)
                // 108 105  52  50 101 105  52  51 101 101 (byte decimal)

                let data = b"li42ei43ee";
                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();
                assert_eq!(parser.output, "[42,43]".to_string());
            }

            #[test]
            fn with_two_utf8_strings() {
                let data = b"l5:alice3:bobe";
                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();
                assert_eq!(parser.output, "[\"alice\",\"bob\"]".to_string());
            }
        }
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
