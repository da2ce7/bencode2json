use std::io::{self, Read};
use std::str;

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

#[derive(Debug, PartialEq)]
pub enum ParsingList {
    Start,
    // review: add FirstItem? to make it clear
    Rest,
}

#[derive(Debug, PartialEq)]
pub enum ParsingDictionary {
    Start,
    FirstKeyValuePair(ParsingKeyValuePair),
    NextKeyValuePair(ParsingKeyValuePair),
}

#[derive(Debug, PartialEq)]
pub enum ParsingKeyValuePair {
    Key,
    Value,
}

pub struct BencodeParser<R: Read> {
    pub json: String,
    pub iter: u64,
    pub pos: u64,
    reader: R,
    stack: Vec<State>,
}

#[derive(Default, Debug)]
struct CurrentStringBeingParsed {
    // String length
    pub bytes_for_string_length: Vec<u8>,
    pub string_length: usize,

    // String bytes
    pub string_bytes: Vec<u8>,
    pub string_bytes_counter: usize,
}

impl CurrentStringBeingParsed {
    fn push(&mut self, byte: u8) {
        // todo: return an error if we try to push a new byte but the end of the
        // string has been reached.

        self.string_bytes.push(byte);
        self.string_bytes_counter += 1;
    }

    fn has_finishing_capturing_bytes(&self) -> bool {
        self.string_bytes_counter == self.string_length
    }

    fn utf8(&self) -> String {
        match str::from_utf8(&self.string_bytes) {
            Ok(string) => {
                // String only contains valid UTF-8 chars -> print it as it's
                string.to_owned()
            }
            Err(_) => {
                // String contains non valid UTF-8 chars -> print it as hex bytes
                Self::bytes_to_hex(&self.string_bytes)
            }
        }
    }

    fn bytes_to_hex(data: &[u8]) -> String {
        format!("<hex>{}</hex>", hex::encode(data))
    }
}

impl<R: Read> BencodeParser<R> {
    pub fn new(reader: R) -> Self {
        BencodeParser {
            reader,
            stack: Vec::new(),
            json: String::new(),
            pos: 0,
            iter: 1,
        }
    }

    /// todo
    ///
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
        // todo: use optional
        let mut current_string_being_parsed = CurrentStringBeingParsed::default();

        loop {
            let byte = match self.read_byte() {
                Ok(byte) => byte,
                Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                    //println!("Reached the end of file.");
                    break;
                }
                Err(err) => return Err(err),
            };

            /*println!("iter: {}", self.iter);
            println!("pos: {}", self.pos);
            println!("byte: {} ({})", byte, byte as char);
            println!("stack: {:#?}", self.stack);
            println!("current_string_being_parsed: {current_string_being_parsed:#?}");
            println!("output: {}", self.json);
            println!();*/

            match byte {
                b'i' => {
                    match self.stack.last() {
                        Some(state) => {
                            match state {
                                State::ParsingList(parsing_list) => match parsing_list {
                                    ParsingList::Start => {
                                        self.stack.push(State::ParsingInteger);
                                    }
                                    ParsingList::Rest => {
                                        self.stack.push(State::ParsingInteger);
                                        self.json.push(',');
                                    }
                                },
                                State::ParsingDictionary(parsing_dictionary) => {
                                    match parsing_dictionary {
                                        ParsingDictionary::Start => {
                                            panic!("invalid byte 'i', expecting string for dictionary key");
                                        }
                                        ParsingDictionary::FirstKeyValuePair(
                                            first_key_value_pair,
                                        ) => {
                                            match first_key_value_pair {
                                                ParsingKeyValuePair::Key => {
                                                    panic!("invalid byte 'i', dictionary key can't be an integer");
                                                }
                                                ParsingKeyValuePair::Value => {
                                                    // First key value in the dictionary is an integer
                                                    self.stack.push(State::ParsingInteger);
                                                }
                                            }
                                        }
                                        ParsingDictionary::NextKeyValuePair(_) => todo!(),
                                    }
                                }
                                State::ParsingInteger => {
                                    panic!("invalid byte, parsing integer expected digit")
                                }
                                State::ParsingString(parsing_string) => match parsing_string {
                                    ParsingString::ParsingLength => {
                                        panic!("unexpected byte 'i', parsing string length ")
                                    }
                                    ParsingString::ParsingChars => {
                                        current_string_being_parsed.push(byte);

                                        if current_string_being_parsed
                                            .has_finishing_capturing_bytes()
                                        {
                                            // We have finishing capturing the string bytes

                                            let string = current_string_being_parsed.utf8();

                                            self.json.push_str(&format!("\"{string}\""));

                                            // We have finished parsing the string
                                            self.stack.pop();
                                            self.check_first_list_item();
                                            self.check_end_dictionary_key();
                                        }
                                    }
                                },
                            }
                        }
                        None => {
                            self.stack.push(State::ParsingInteger);
                        }
                    }
                }
                b'0'..=b'9' => {
                    match self.stack.last() {
                        Some(state) => match state {
                            State::ParsingInteger => {
                                self.json.push(byte as char);
                            }
                            State::ParsingString(parsing_string) => match parsing_string {
                                ParsingString::ParsingLength => {
                                    // Add a digit for the string length
                                    current_string_being_parsed
                                        .bytes_for_string_length
                                        .push(byte);
                                }
                                ParsingString::ParsingChars => {
                                    current_string_being_parsed.push(byte);

                                    if current_string_being_parsed.has_finishing_capturing_bytes() {
                                        // We have finishing capturing the string bytes

                                        let string = current_string_being_parsed.utf8();

                                        self.json.push_str(&format!("\"{string}\""));

                                        // We have finished parsing the string
                                        self.stack.pop();
                                        self.check_first_list_item();
                                        self.check_end_dictionary_key();
                                    }
                                }
                            },
                            State::ParsingList(parsing_list) => {
                                match parsing_list {
                                    ParsingList::Start => {
                                        // First item in the list and it is a string

                                        // New string -> reset current string being parsed
                                        current_string_being_parsed.bytes_for_string_length =
                                            Vec::new();
                                        current_string_being_parsed.string_length = 0;
                                        current_string_being_parsed.string_bytes = Vec::new();
                                        current_string_being_parsed.string_bytes_counter = 0;

                                        current_string_being_parsed
                                            .bytes_for_string_length
                                            .push(byte);

                                        self.stack.push(State::ParsingString(
                                            ParsingString::ParsingLength,
                                        ));
                                    }
                                    ParsingList::Rest => {
                                        // Non first item in the list and it is a string

                                        // New string -> reset current string being parsed
                                        current_string_being_parsed.bytes_for_string_length =
                                            Vec::new();
                                        current_string_being_parsed.string_length = 0;
                                        current_string_being_parsed.string_bytes = Vec::new();
                                        current_string_being_parsed.string_bytes_counter = 0;

                                        current_string_being_parsed
                                            .bytes_for_string_length
                                            .push(byte);

                                        self.stack.push(State::ParsingString(
                                            ParsingString::ParsingLength,
                                        ));

                                        self.json.push(',');
                                    }
                                }
                            }
                            State::ParsingDictionary(parsing_dictionary) => {
                                match parsing_dictionary {
                                    ParsingDictionary::Start => {
                                        // First key in the dictionary

                                        self.stack.push(State::ParsingDictionary(
                                            ParsingDictionary::FirstKeyValuePair(
                                                ParsingKeyValuePair::Key,
                                            ),
                                        ));

                                        // New string -> reset current string being parsed
                                        current_string_being_parsed.bytes_for_string_length =
                                            Vec::new();
                                        current_string_being_parsed.string_length = 0;
                                        current_string_being_parsed.string_bytes = Vec::new();
                                        current_string_being_parsed.string_bytes_counter = 0;

                                        current_string_being_parsed
                                            .bytes_for_string_length
                                            .push(byte);

                                        self.stack.push(State::ParsingString(
                                            ParsingString::ParsingLength,
                                        ));
                                    }
                                    ParsingDictionary::FirstKeyValuePair(
                                        parsing_first_key_value_pair,
                                    ) => {
                                        match parsing_first_key_value_pair {
                                            ParsingKeyValuePair::Key => {
                                                todo!()
                                            }
                                            ParsingKeyValuePair::Value => {
                                                // First key value in the dictionary and it's an string

                                                // New string -> reset current string being parsed
                                                current_string_being_parsed
                                                    .bytes_for_string_length = Vec::new();
                                                current_string_being_parsed.string_length = 0;
                                                current_string_being_parsed.string_bytes =
                                                    Vec::new();
                                                current_string_being_parsed.string_bytes_counter =
                                                    0;

                                                current_string_being_parsed
                                                    .bytes_for_string_length
                                                    .push(byte);

                                                self.stack.push(State::ParsingString(
                                                    ParsingString::ParsingLength,
                                                ));
                                            }
                                        }
                                    }
                                    ParsingDictionary::NextKeyValuePair(_) => todo!(),
                                }
                            }
                        },
                        None => {
                            // First byte in input and it is a string
                            self.stack
                                .push(State::ParsingString(ParsingString::ParsingLength));

                            // New string -> reset current string being parsed
                            current_string_being_parsed.bytes_for_string_length = Vec::new();
                            current_string_being_parsed.string_length = 0;
                            current_string_being_parsed.string_bytes = Vec::new();
                            current_string_being_parsed.string_bytes_counter = 0;

                            current_string_being_parsed
                                .bytes_for_string_length
                                .push(byte);
                        }
                    };
                }
                b':' => match self.stack.last() {
                    Some(state) => match state {
                        State::ParsingString(parsing_string) => {
                            match parsing_string {
                                ParsingString::ParsingLength => {
                                    // We reach the end of the string length
                                    let length_str = str::from_utf8(
                                        &current_string_being_parsed.bytes_for_string_length,
                                    )
                                    .expect("non UTF8 string length");
                                    //println!("length_str: {length_str}");

                                    current_string_being_parsed.string_length = length_str
                                        .parse::<usize>()
                                        .expect("invalid number for string length");
                                    //println!("string_length_number: {string_length}");

                                    // todo: is string with size 0 (empty string) allowed in bencode?

                                    // We have finished parsing the string length
                                    self.stack.pop();
                                    self.stack
                                        .push(State::ParsingString(ParsingString::ParsingChars));
                                }
                                ParsingString::ParsingChars => {
                                    current_string_being_parsed.push(byte);

                                    if current_string_being_parsed.has_finishing_capturing_bytes() {
                                        // We have finishing capturing the string bytes

                                        let string = current_string_being_parsed.utf8();

                                        self.json.push_str(&format!("\"{string}\""));

                                        // We have finished parsing the string
                                        self.stack.pop();
                                        self.check_first_list_item();
                                        self.check_end_dictionary_key();
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
                        Some(state) => match state {
                            State::ParsingList(parsing_list) => match parsing_list {
                                ParsingList::Start => {
                                    self.stack.push(State::ParsingList(ParsingList::Start));
                                    self.json.push('[');
                                }
                                ParsingList::Rest => {}
                            },
                            State::ParsingDictionary(parsing_dictionary) => {
                                match parsing_dictionary {
                                    ParsingDictionary::Start => todo!(),
                                    ParsingDictionary::FirstKeyValuePair(_) => todo!(),
                                    ParsingDictionary::NextKeyValuePair(_) => todo!(),
                                }
                            }
                            State::ParsingInteger => {}
                            State::ParsingString(parsing_string) => match parsing_string {
                                ParsingString::ParsingLength => {
                                    panic!("unexpected byte: 'l', parsing string length")
                                }
                                ParsingString::ParsingChars => {
                                    current_string_being_parsed.push(byte);

                                    if current_string_being_parsed.has_finishing_capturing_bytes() {
                                        // We have finishing capturing the string bytes

                                        let string = current_string_being_parsed.utf8();

                                        self.json.push_str(&format!("\"{string}\""));

                                        // We have finished parsing the string
                                        self.stack.pop();
                                        self.check_first_list_item();
                                        self.check_end_dictionary_key();
                                    }
                                }
                            },
                        },
                        None => {
                            self.stack.push(State::ParsingList(ParsingList::Start));
                            self.json.push('[');
                        }
                    }
                }
                b'd' => match self.stack.last() {
                    Some(_) => todo!(),
                    None => {
                        self.stack
                            .push(State::ParsingDictionary(ParsingDictionary::Start));
                        self.json.push('{');
                    }
                },
                b'e' => {
                    match self.stack.last() {
                        Some(state) => match state {
                            State::ParsingList(_) => {
                                // We have finished parsing the list
                                self.stack.pop();
                                self.json.push(']');
                            }
                            State::ParsingDictionary(parsing_dictionary) => {
                                match parsing_dictionary {
                                    ParsingDictionary::Start => {
                                        // We have finished parsing the dictionary (empty dictionary)
                                        self.stack.pop();
                                        self.json.push('}');
                                    }
                                    ParsingDictionary::FirstKeyValuePair(
                                        parsing_first_key_value,
                                    ) => {
                                        match parsing_first_key_value {
                                            ParsingKeyValuePair::Key => todo!(),
                                            ParsingKeyValuePair::Value => {
                                                {
                                                    // We have finished parsing the dictionary (non empty dictionary)
                                                    self.stack.pop(); // FirstKeyValue

                                                    self.json.push('}');

                                                    self.stack.pop(); // Start
                                                }
                                            }
                                        }
                                    }
                                    ParsingDictionary::NextKeyValuePair(_) => todo!(),
                                }
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
                                    current_string_being_parsed.push(byte);

                                    if current_string_being_parsed.has_finishing_capturing_bytes() {
                                        // We have finishing parsing the string

                                        let string = current_string_being_parsed.utf8();

                                        self.json.push_str(&format!("\"{string}\""));

                                        // We have finished parsing the string
                                        self.stack.pop();
                                        self.check_first_list_item();
                                        self.check_end_dictionary_key();
                                    }
                                }
                            },
                        },
                        None => panic!("invalid byte, unexpected end byte `e`"),
                    }

                    self.check_first_list_item();
                }
                _ => {
                    match self.stack.last() {
                        Some(state) => match state {
                            State::ParsingList(_) => {}
                            State::ParsingDictionary(_) => {}
                            State::ParsingInteger => {}
                            State::ParsingString(parsing_string) => match parsing_string {
                                ParsingString::ParsingLength => {}
                                ParsingString::ParsingChars => {
                                    current_string_being_parsed.push(byte);

                                    if current_string_being_parsed.has_finishing_capturing_bytes() {
                                        // We have finishing capturing the string bytes

                                        let string = current_string_being_parsed.utf8();

                                        self.json.push_str(&format!("\"{string}\""));

                                        // We have finished parsing the string
                                        self.stack.pop();
                                        self.check_first_list_item();
                                        self.check_end_dictionary_key();
                                    }
                                }
                            },
                        },
                        None => {}
                    }
                }
            }

            self.iter += 1;
        }

        // todo: if we exit the loop with a non empty stack, that's an error (incomplete bencode value).

        Ok(())
    }

    fn read_byte(&mut self) -> io::Result<u8> {
        let mut byte = [0; 1];
        self.reader.read_exact(&mut byte)?;
        self.pos += 1;
        Ok(byte[0])
    }

    #[allow(clippy::single_match)]
    fn check_first_list_item(&mut self) {
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

    #[allow(clippy::single_match)]
    #[allow(clippy::match_same_arms)]
    fn check_end_dictionary_key(&mut self) {
        match self.stack.last() {
            Some(state) => match state {
                State::ParsingInteger => {}
                State::ParsingString(_) => {}
                State::ParsingList(_) => {}
                State::ParsingDictionary(parsing_dictionary) => match parsing_dictionary {
                    ParsingDictionary::Start => {}
                    ParsingDictionary::FirstKeyValuePair(parsing_first_key_value_pair) => {
                        match parsing_first_key_value_pair {
                            ParsingKeyValuePair::Key => {
                                self.stack.pop();
                                self.stack.push(State::ParsingDictionary(
                                    ParsingDictionary::FirstKeyValuePair(
                                        ParsingKeyValuePair::Value,
                                    ),
                                ));
                                self.json.push(':');
                            }
                            ParsingKeyValuePair::Value => {}
                        }
                    }
                    ParsingDictionary::NextKeyValuePair(_) => todo!(),
                },
            },
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer() {
        let data = b"i42e";
        let mut parser = BencodeParser::new(&data[..]);
        parser.parse().unwrap();
        assert_eq!(parser.json, "42".to_string());
    }

    mod strings {
        use crate::BencodeParser;

        #[test]
        fn utf8() {
            let data = b"4:spam";

            let mut parser = BencodeParser::new(&data[..]);
            parser.parse().unwrap();

            assert_eq!(parser.json, "\"spam\"".to_string());
        }

        #[test]
        fn non_utf8() {
            let data = b"4:\xFF\xFE\xFD\xFC";

            let mut parser = BencodeParser::new(&data[..]);
            parser.parse().unwrap();

            assert_eq!(parser.json, "\"<hex>fffefdfc</hex>\"".to_string());
        }

        /* todo:
           - String containing special chars: 'i', ':', 'l', 'd', 'e'. The
             bencoded string can contain reserved chars in bencode format.
        */
    }

    mod lists {
        use crate::BencodeParser;

        #[test]
        fn empty_list() {
            let data = b"le";

            let mut parser = BencodeParser::new(&data[..]);
            parser.parse().unwrap();

            assert_eq!(parser.json, "[]".to_string());
        }

        mod with_one_item {
            use crate::BencodeParser;

            #[test]
            fn integer() {
                let data = b"li42ee";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "[42]".to_string());
            }

            #[test]
            fn utf8_string() {
                // List with one UTF8 string: l4:spame
                //   1   2   3   4   5   6   7   8 (pos)
                //   l   4   :   s   p   a   m   e (byte)
                // 108  52  58 115 112  97 109 101 (byte decimal)

                let data = b"l4:spame";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "[\"spam\"]".to_string());
            }

            #[test]
            fn non_utf8_string() {
                // List with one UTF8 string: l4:\xFF\xFE\xFD\xFCe
                //   1   2   3   4   5   6   7   8 (pos)
                //   l   4   : xFF xFE xFD xFC   e (byte)
                // 108  52  58 255 254 253 252 101 (byte decimal)

                let data = b"l4:\xFF\xFE\xFD\xFCe";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "[\"<hex>fffefdfc</hex>\"]".to_string());
            }

            mod of_type_list {
                use crate::BencodeParser;

                #[test]
                fn nested_empty_list() {
                    // List with one empty list: llee
                    //   1   2   3   4 (pos)
                    //   l   l   e   e (byte)
                    // 108 108 101 101 (byte decimal)

                    let data = b"llee";

                    let mut parser = BencodeParser::new(&data[..]);
                    parser.parse().unwrap();

                    assert_eq!(parser.json, "[[]]".to_string());
                }

                #[test]
                fn two_nested_empty_lists() {
                    // List with two nested empty lists: llleee
                    //   1   2   3   4   5   6 (pos)
                    //   l   l   l   e   e   e (byte)
                    // 108 108 108 101 101 101 (byte decimal)

                    let data = b"llleee";

                    let mut parser = BencodeParser::new(&data[..]);
                    parser.parse().unwrap();

                    assert_eq!(parser.json, "[[[]]]".to_string());
                }

                #[test]
                fn nested_list_with_integer() {
                    // List with one empty list: lli42eee
                    //   1   2   3   4   5   6   7   4 (pos)
                    //   l   l   i   4   2   e   e   e (byte)
                    // 108 108 105  52  50 101 101 101 (byte decimal)

                    let data = b"lli42eee";

                    let mut parser = BencodeParser::new(&data[..]);
                    parser.parse().unwrap();

                    assert_eq!(parser.json, "[[42]]".to_string());
                }

                /* todo:
                    - Nested list with UTF-8 string
                    - Nested list with non UTF-8 string

                    - Two nested lists with one integer each
                    - Two nested lists with one UTF-8 string each
                    - Two nested lists with one non UTF-8 string each
                */
            }

            /* todo:
                - With one dictionary
            */
        }

        mod with_two_items_of_the_same_type {
            use crate::BencodeParser;

            #[test]
            fn two_integers() {
                // List with two integers: li42ei43ee
                //   1   2   3   4   5   6   7   8   9  10 (pos)
                //   l   i   4   2   e   i   4   3   e   e (byte)
                // 108 105  52  50 101 105  52  51 101 101 (byte decimal)

                let data = b"li42ei43ee";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "[42,43]".to_string());
            }

            #[test]
            fn two_utf8_strings() {
                // List with two UTF8 strings: l5:alice3:bobe
                //   1   2   3   4   5   6   7   8   9  10  11  12  13  14 (pos)
                //   l   5   :   a   l   i   c   e   3   :   b   o   b   e (byte)
                // 108  53  58  97 108 105  99 101  51  58  98 111  98 101 (byte decimal)

                let data = b"l5:alice3:bobe";
                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "[\"alice\",\"bob\"]".to_string());
            }

            #[test]
            fn two_non_utf8_strings() {
                // List with two UTF8 strings: l2:\xFF\xFE2:\xFD\xFCe
                //   1   2   3   4   5   6   7   8   9  10 (pos)
                //   l   2   : xFF xFE   2   : xFD xFC   e (byte)
                // 108  53  58 255 254 105  99 253 252 101 (byte decimal)

                let data = b"l2:\xFF\xFE2:\xFD\xFCe";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(
                    parser.json,
                    "[\"<hex>fffe</hex>\",\"<hex>fdfc</hex>\"]".to_string()
                );
            }
        }

        mod with_two_items_of_different_types {
            use crate::BencodeParser;

            #[test]
            fn integer_and_utf8_string() {
                // List with an integer and a UTF-8 string: li42e5:alicee
                //   1   2   3   4   5   6   7   8   9  10  11  12  13 (pos)
                //   l   i   4   2   e   5   :   a   l   i   c   e   e (byte)
                // 108 105  52  50 101  53  58  97 108 105  99 101 101 (byte decimal)

                let data = b"li42e5:alicee";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "[42,\"alice\"]".to_string());
            }

            #[test]
            fn integer_and_non_utf8_string() {
                // List with an integer a non UTF-8 string: li42e2:\xFF\xFEe
                //   1   2   3   4   5   6   7   8   9  10 (pos)
                //   l   i   4   2   e   2   : xFF xFE   e (byte)
                // 108 105  52  50 101  50  58 255 254 105 (byte decimal)

                let data = b"li42e2:\xFF\xFEe";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "[42,\"<hex>fffe</hex>\"]".to_string());
            }

            #[test]
            fn utf8_string_and_integer() {
                // List with a UTF-8 string and an integer: l5:alicei42ee
                //   1   2   3   4   5   6   7   8   9  10  11  12  13 (pos)
                //   l   5   :   a   l   i   c   e   i   4   2   e   e (byte)
                // 108  53  58  97 108 105  99 101 105  52  50 101 101 101 (byte decimal)

                let data = b"l5:alicei42ee";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "[\"alice\",42]".to_string());
            }

            #[test]
            fn non_utf8_string_and_an_integer() {
                // List with a non UTF-8 string and an integer: l2:\xFF\xFEi42ee
                //   1   2   3   4   5   6   7   8   9  10 (pos)
                //   l   2   : xFF xFE   i   4   2   e  e (byte)
                // 108  50  58 255 254 105  52  50 101105 (byte decimal)

                let data = b"l2:\xFF\xFEi42ee";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "[\"<hex>fffe</hex>\",42]".to_string());
            }

            /* todo:
                - Integer and list
                - Integer and dictionary

                - UTF-8 string and list
                - UTF-8 string and dictionary

                - Non UTF-8 string and list
                - Non UTF-8 string and dictionary
            */
        }
    }

    mod dictionary {
        use crate::BencodeParser;

        // Note: Keys must be bencoded strings.

        /* todo:

           Valid cases:

           - A key starting with a digit.
           - A key with non UTF-8 value:
                Bencode: d2:\xFF\xFEi42ee
                JSON:    {"<hex>fffe</hex>": 42}

           Error cases:

           - A dictionary key can't be an integer.
           - A dictionary with one key pair, but only the key without value.
        */

        #[test]
        fn empty_dictionary() {
            let data = b"de";

            let mut parser = BencodeParser::new(&data[..]);
            parser.parse().unwrap();

            assert_eq!(parser.json, "{}".to_string());
        }

        mod with_one_key_of_type {
            use crate::BencodeParser;

            #[test]
            fn integer() {
                let data = b"d3:fooi42ee";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "{\"foo\":42}".to_string());
            }

            #[test]
            fn utf8_string() {
                let data = b"d3:bar4:spame";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "{\"bar\":\"spam\"}".to_string());
            }

            #[test]
            fn non_utf8_string() {
                let data = b"d3:bar2:\xFF\xFEe";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "{\"bar\":\"<hex>fffe</hex>\"}".to_string());
            }
        }

        mod with_two_keys_of_the_same_type {
            use crate::BencodeParser;

            #[test]
            #[ignore]
            fn two_integers() {
                let data = b"d3:bari42e3:fooi43ee";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "{\"bar\":42,\"foo\":43}".to_string());
            }
        }
    }
}
