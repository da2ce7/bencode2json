pub mod integer;
pub mod stack;
pub mod string;

use std::io::{self, Read, Write};

use stack::{Stack, State};
use string::StringParser;

use crate::io::{byte_reader::ByteReader, byte_writer::ByteWriter};

pub struct BencodeParser<R: Read, W: Write> {
    pub debug: bool,
    pub iter: u64,
    byte_reader: ByteReader<R>,
    byte_writer: ByteWriter<W>,
    stack: Stack,
}

impl<R: Read, W: Write> BencodeParser<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        BencodeParser {
            debug: false, // todo: use tracing crate
            iter: 1,
            byte_reader: ByteReader::new(reader),
            byte_writer: ByteWriter::new(writer),
            stack: Stack::default(),
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
    #[allow(clippy::single_match)]
    pub fn parse(&mut self) -> io::Result<()> {
        loop {
            let byte = match self.byte_reader.read_byte() {
                Ok(byte) => byte,
                Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                    //println!("Reached the end of file.");
                    break;
                }
                Err(err) => return Err(err),
            };

            if self.debug {
                println!("iter: {}", self.iter);
                println!("pos: {}", self.byte_reader.input_byte_counter);
                println!("byte: {} ({})", byte, byte as char);
            }

            match byte {
                b'i' => {
                    // Begin of integer
                    self.begin_bencoded_value()?;
                    self.parse_integer().expect("invalid integer");
                }
                b'0'..=b'9' => {
                    // Begin of string
                    self.begin_bencoded_value()?;
                    self.parse_string(byte).expect("invalid string");
                }
                b'l' => {
                    // Begin of list
                    self.begin_bencoded_value()?;
                    self.byte_writer.write_byte(b'[')?;
                    self.stack.push(State::ExpectingFirstItemOrEnd);
                }
                b'd' => {
                    // Begin of dictionary
                    self.begin_bencoded_value()?;
                    self.byte_writer.write_byte(b'{')?;
                    self.stack.push(State::ExpectingFirstFieldOrEnd);
                }
                b'e' => {
                    // End of list or dictionary (not end of integer)
                    self.end_bencoded_value()?;
                }
                _ => {
                    panic!("{}", format!("unexpected byte {} ({})", byte, byte as char));
                }
            }

            if self.debug {
                println!("stack: {}", self.stack);
                //println!("string_parser: {:#?}", self.string_parser);
                self.byte_reader.print_captured_input();
                self.byte_writer.print_captured_output();
                println!();
            }

            self.iter += 1;
        }

        // todo: if we exit the loop with a non default stack, that's an error
        // (incomplete bencode value).

        Ok(())
    }

    /// It updates the stack state and prints the delimiters when needed.
    ///
    /// Called when the first byt of a bencoded value (integer, string, list or dict)
    /// is received.
    ///
    /// # Errors
    ///
    /// Will return an error if the writer can't write to the output.
    pub fn begin_bencoded_value(&mut self) -> io::Result<()> {
        match self.stack.peek() {
            // Initial state
            State::Initial => {}
            // List
            State::ExpectingFirstItemOrEnd => {
                self.stack.swap_top(State::ExpectingNextItem);
            }
            State::ExpectingNextItem => {
                // Items separator
                self.byte_writer.write_byte(b',')?;
            }
            // Dictionary
            State::ExpectingFirstFieldOrEnd => {
                self.stack.swap_top(State::ExpectingFieldValue);
            }
            State::ExpectingFieldValue => {
                // Key/Value separator
                self.byte_writer.write_byte(b':')?;
                self.stack.swap_top(State::ExpectingFieldKey);
            }
            State::ExpectingFieldKey => {
                // Field separator
                self.byte_writer.write_byte(b',')?;
                self.stack.swap_top(State::ExpectingFieldValue);
            }
        }

        Ok(())
    }

    /// It updates the stack state and prints the delimiters when needed.
    ///
    /// Called when the end of list or dictionary byte is received. End of
    /// integers or strings are processed while parsing them.
    ///
    /// # Errors
    ///
    /// Will return an error if the writer can't write to the output.
    ///
    /// # Panics
    ///
    /// Will panic if the end of bencoded value (list or dictionary) was not
    /// expected.
    pub fn end_bencoded_value(&mut self) -> io::Result<()> {
        match self.stack.peek() {
            State::ExpectingFirstItemOrEnd | State::ExpectingNextItem => {
                self.byte_writer.write_byte(b']')?;
                self.stack.pop();
            }
            State::ExpectingFirstFieldOrEnd | State::ExpectingFieldKey => {
                self.byte_writer.write_byte(b'}')?;
            }
            State::ExpectingFieldValue | State::Initial => {
                // todo: pass the type of value (list or dict) to customize the error message
                panic!("error parsing end of list or dictionary, unexpected initial state on the stack")
            }
        }
        // todo: sp < stack. What this conditions does in the C implementation?

        Ok(())
    }

    fn parse_integer(&mut self) -> io::Result<()> {
        integer::parse(&mut self.byte_reader, &mut self.byte_writer)
    }

    fn parse_string(&mut self, byte: u8) -> io::Result<()> {
        let mut string_parser = StringParser::default();

        string_parser.new_string_starting_with(byte);

        // Parse length

        loop {
            let byte = match self.byte_reader.read_byte() {
                Ok(byte) => byte,
                Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                    //println!("Reached the end of file.");
                    panic!("unexpected end of input parsing string length");
                }
                Err(err) => return Err(err),
            };

            match byte {
                b':' => {
                    // End of string length
                    string_parser.process_end_of_string_length();
                    break;
                }
                _ => {
                    string_parser.add_length_byte(byte);
                }
            }
        }

        // Parse value

        for _i in 1..=string_parser.string_length {
            let byte = match self.byte_reader.read_byte() {
                Ok(byte) => byte,
                Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                    //println!("Reached the end of file.");
                    panic!("unexpected end of input parsing string chars");
                }
                Err(err) => return Err(err),
            };

            string_parser.add_byte(byte);

            // todo: escape '"' and '\\' with '\\';
        }

        self.byte_writer.write_str(&string_parser.json())?;

        //println!("string_parser {string_parser:#?}");

        Ok(())
    }

    pub fn opt_captured_output(&self) -> Option<String> {
        self.byte_writer.opt_captured_output.clone()
    }
}

#[cfg(test)]
mod tests {

    use super::BencodeParser;

    fn to_json(input_bytes: &[u8]) -> String {
        let output_bytes = Vec::new();

        let mut parser = BencodeParser::new(input_bytes, output_bytes);

        parser.parse().expect("bencoded to JSON conversion failed");

        match parser.opt_captured_output() {
            Some(captured_output) => captured_output,
            None => panic!(
                "capturing output is not enabled in parser, please enable it to run the tests"
            ),
        }
    }

    mod integers {
        use crate::parsers::tests::to_json;

        #[test]
        fn zero() {
            assert_eq!(to_json(b"i0e"), "0".to_string());
        }

        #[test]
        fn one_digit_integer() {
            assert_eq!(to_json(b"i1e"), "1".to_string());
        }

        #[test]
        fn two_digits_integer() {
            assert_eq!(to_json(b"i42e"), "42".to_string());
        }

        #[test]
        fn negative_integer() {
            assert_eq!(to_json(b"i-1e"), "-1".to_string());
        }

        // todo: all encodings with a leading zero, such as i03e, are invalid, other
        // than i0e, which of course corresponds to 0.
    }

    mod strings {
        use crate::parsers::tests::to_json;

        #[test]
        fn empty_string() {
            assert_eq!(to_json(b"0:"), r#""""#.to_string());
        }

        #[test]
        fn utf8() {
            assert_eq!(to_json(b"4:spam"), r#""spam""#.to_string());
        }

        #[test]
        fn non_utf8() {
            assert_eq!(
                to_json(b"4:\xFF\xFE\xFD\xFC"),
                r#""<hex>fffefdfc</hex>""#.to_string()
            );
        }

        #[test]
        fn ending_with_bencode_end_char() {
            assert_eq!(to_json(b"1:e"), r#""e""#.to_string());
        }

        #[test]
        fn containing_a_reserved_char() {
            assert_eq!(to_json(b"1:i"), r#""i""#.to_string());
            assert_eq!(to_json(b"1:l"), r#""l""#.to_string());
            assert_eq!(to_json(b"1:d"), r#""d""#.to_string());
            assert_eq!(to_json(b"1:l"), r#""l""#.to_string());
            assert_eq!(to_json(b"1:e"), r#""e""#.to_string());
        }

        #[test]
        fn containing_a_digit() {
            assert_eq!(to_json(b"1:0"), r#""0""#.to_string());
            assert_eq!(to_json(b"1:1"), r#""1""#.to_string());
            assert_eq!(to_json(b"1:2"), r#""2""#.to_string());
            assert_eq!(to_json(b"1:3"), r#""3""#.to_string());
            assert_eq!(to_json(b"1:4"), r#""4""#.to_string());
            assert_eq!(to_json(b"1:5"), r#""5""#.to_string());
            assert_eq!(to_json(b"1:6"), r#""6""#.to_string());
            assert_eq!(to_json(b"1:7"), r#""7""#.to_string());
            assert_eq!(to_json(b"1:8"), r#""8""#.to_string());
            assert_eq!(to_json(b"1:9"), r#""9""#.to_string());
        }

        /* todo:
           - String containing special chars like : `"`, `\`, '\\'
           - String containing JSON
        */
    }

    mod lists {
        use crate::parsers::tests::to_json;

        #[test]
        fn empty_list() {
            assert_eq!(to_json(b"le"), "[]".to_string());
        }

        mod with_one_item {
            use crate::parsers::tests::to_json;

            #[test]
            fn integer() {
                assert_eq!(to_json(b"li42ee"), "[42]".to_string());
            }

            #[test]
            fn utf8_string() {
                assert_eq!(to_json(b"l4:spame"), r#"["spam"]"#.to_string());
            }

            #[test]
            fn non_utf8_string() {
                assert_eq!(
                    to_json(b"l4:\xFF\xFE\xFD\xFCe"),
                    r#"["<hex>fffefdfc</hex>"]"#.to_string()
                );
            }

            mod of_type_list {
                use crate::parsers::tests::to_json;

                /* todo:
                    - Main list empty, nested list two items:
                        - Nested list with non UTF-8 string
                */

                #[test]
                fn two_nested_empty_list() {
                    assert_eq!(to_json(b"llee"), "[[]]".to_string());
                }

                #[test]
                fn three_nested_empty_lists() {
                    assert_eq!(to_json(b"llleee"), "[[[]]]".to_string());
                }

                #[test]
                fn one_nested_list_which_contains_one_integer() {
                    assert_eq!(to_json(b"lli42eee"), "[[42]]".to_string());
                }

                #[test]
                fn one_nested_list_which_contains_two_integers() {
                    assert_eq!(to_json(b"lli42ei43eee"), "[[42,43]]".to_string());
                }

                #[test]
                fn one_nested_list_which_contains_one_utf_8_string() {
                    assert_eq!(to_json(b"ll4:spamee"), r#"[["spam"]]"#.to_string());
                }

                #[test]
                fn one_nested_list_which_contains_two_utf_8_strings() {
                    assert_eq!(
                        to_json(b"ll5:alice3:bobee"),
                        r#"[["alice","bob"]]"#.to_string()
                    );
                }
            }

            /* todo:
                - With one dictionary
            */
        }

        mod with_two_items_of_the_same_type {
            use crate::parsers::tests::to_json;

            #[test]
            fn two_integers() {
                assert_eq!(to_json(b"li42ei43ee"), "[42,43]".to_string());
            }

            #[test]
            fn two_utf8_strings() {
                assert_eq!(to_json(b"l5:alice3:bobe"), r#"["alice","bob"]"#.to_string());
            }

            #[test]
            fn two_non_utf8_strings() {
                assert_eq!(
                    to_json(b"l2:\xFF\xFE2:\xFD\xFCe"),
                    r#"["<hex>fffe</hex>","<hex>fdfc</hex>"]"#.to_string()
                );
            }
        }

        mod with_two_items_of_different_types {
            use crate::parsers::tests::to_json;

            #[test]
            fn integer_and_utf8_string() {
                assert_eq!(to_json(b"li42e5:alicee"), r#"[42,"alice"]"#.to_string());
            }

            #[test]
            fn integer_and_non_utf8_string() {
                assert_eq!(
                    to_json(b"li42e2:\xFF\xFEe"),
                    r#"[42,"<hex>fffe</hex>"]"#.to_string()
                );
            }

            #[test]
            fn utf8_string_and_integer() {
                assert_eq!(to_json(b"l5:alicei42ee"), r#"["alice",42]"#.to_string());
            }

            #[test]
            fn non_utf8_string_and_an_integer() {
                assert_eq!(
                    to_json(b"l2:\xFF\xFEi42ee"),
                    r#"["<hex>fffe</hex>",42]"#.to_string()
                );
            }

            mod integer_and_list {
                use crate::parsers::tests::to_json;

                #[test]
                fn second_item_empty_list() {
                    assert_eq!(to_json(b"li42elee"), "[42,[]]".to_string());
                }
            }

            /* todo:
                - Integer and dictionary

                - UTF-8 string and list
                - UTF-8 string and dictionary

                - Non UTF-8 string and list
                - Non UTF-8 string and dictionary
            */
        }
    }

    mod dictionary {
        use crate::parsers::tests::to_json;

        // Note: Keys must be bencoded strings.

        /* todo:

           Error cases:

           - A dictionary key can't be an integer.
           - A dictionary with one key pair, but only the key without value.
        */

        #[test]
        fn empty_dictionary() {
            assert_eq!(to_json(b"de"), "{}".to_string());
        }

        mod with_one_key {
            use crate::parsers::tests::to_json;

            #[test]
            fn with_a_key_starting_with_a_digit() {
                assert_eq!(to_json(b"d4:1fooi42ee"), r#"{"1foo":42}"#.to_string());
            }

            #[test]
            fn with_a_key_with_a_non_urf_string() {
                assert_eq!(
                    to_json(b"d2:\xFF\xFEi42ee"),
                    r#"{"<hex>fffe</hex>":42}"#.to_string()
                );
            }
        }

        mod with_one_key_of_type {
            use crate::parsers::tests::to_json;

            #[test]
            fn integer() {
                assert_eq!(to_json(b"d3:fooi42ee"), r#"{"foo":42}"#.to_string());
            }

            #[test]
            fn utf8_string() {
                assert_eq!(to_json(b"d3:bar4:spame"), r#"{"bar":"spam"}"#.to_string());
            }

            #[test]
            fn non_utf8_string() {
                assert_eq!(
                    to_json(b"d3:bar2:\xFF\xFEe"),
                    r#"{"bar":"<hex>fffe</hex>"}"#.to_string()
                );
            }
        }

        mod with_two_keys_of_the_same_type {
            use crate::parsers::tests::to_json;

            #[test]
            fn two_integers() {
                assert_eq!(
                    to_json(b"d3:bari42e3:fooi43ee"),
                    r#"{"bar":42,"foo":43}"#.to_string()
                );
            }

            #[test]
            fn two_utf8_strings() {
                assert_eq!(
                    to_json(b"d3:bar4:spam3:foo5:alicee"),
                    r#"{"bar":"spam","foo":"alice"}"#.to_string()
                );
            }
        }
    }
}
