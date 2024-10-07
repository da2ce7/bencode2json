pub mod integer;
pub mod stack;
pub mod string;

use std::{
    fmt::Write as FmtWrite,
    io::{self, Read, Write as IoWrite},
};

use stack::{Stack, State};

use crate::io::{
    byte_reader::ByteReader, byte_writer::ByteWriter, char_writer::CharWriter, writer::Writer,
};

pub struct BencodeParser<R: Read> {
    pub debug: bool,
    pub iter: u64,
    byte_reader: ByteReader<R>,
    stack: Stack,
}

impl<R: Read> BencodeParser<R> {
    const JSON_ARRAY_BEGIN: u8 = b'[';
    const JSON_ARRAY_ITEMS_SEPARATOR: u8 = b',';
    const JSON_ARRAY_END: u8 = b']';

    const JSON_OBJ_BEGIN: u8 = b'{';
    const JSON_OBJ_FIELDS_SEPARATOR: u8 = b',';
    const JSON_OBJ_FIELD_KEY_VALUE_SEPARATOR: u8 = b':';
    const JSON_OBJ_END: u8 = b'}';

    pub fn new(reader: R) -> Self {
        BencodeParser {
            debug: false, // todo: use tracing crate
            iter: 1,
            byte_reader: ByteReader::new(reader),
            stack: Stack::default(),
        }
    }

    /// It parses a bencoded value read from input and writes the corresponding
    /// JSON value to the output.
    ///
    /// # Errors
    ///
    /// Will return an error if it can't read from the input or write to the
    /// output.
    ///
    /// # Panics
    ///
    /// Will panic if receives a byte that isn't a valid begin or end of a
    /// bencoded type: integer, string, list or dictionary.
    pub fn write_bytes<W: IoWrite>(&mut self, writer: W) -> io::Result<()> {
        let mut writer = ByteWriter::new(writer);
        self.parse(&mut writer)
    }

    /// It parses a bencoded value read from input and writes the corresponding
    /// JSON value to the output.
    ///
    /// # Errors
    ///
    /// Will return an error if it can't read from the input or write to the
    /// output.
    ///
    /// # Panics
    ///
    /// Will panic if receives a byte that isn't a valid begin or end of a
    /// bencoded type: integer, string, list or dictionary.
    pub fn write_str<W: FmtWrite>(&mut self, writer: W) -> io::Result<()> {
        let mut writer = CharWriter::new(writer);
        self.parse(&mut writer)
    }

    /// It parses a bencoded value read from input and writes the corresponding
    /// JSON value to the output.
    ///
    /// # Errors
    ///
    /// Will return an error if it can't read from the input or write to the
    /// output.
    ///
    /// # Panics
    ///
    /// Will panic if receives a byte that isn't a valid begin or end of a
    /// bencoded type: integer, string, list or dictionary.
    fn parse<W: Writer>(&mut self, writer: &mut W) -> io::Result<()> {
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
                    self.begin_bencoded_value(writer)?;
                    integer::parse(&mut self.byte_reader, writer, byte)?;
                }
                b'0'..=b'9' => {
                    // Begin of string
                    self.begin_bencoded_value(writer)?;
                    string::parse(&mut self.byte_reader, writer, byte)?;
                }
                b'l' => {
                    // Begin of list
                    self.begin_bencoded_value(writer)?;
                    writer.write_byte(Self::JSON_ARRAY_BEGIN)?;
                    self.stack.push(State::ExpectingFirstListItemOrEnd);
                }
                b'd' => {
                    // Begin of dictionary
                    self.begin_bencoded_value(writer)?;
                    writer.write_byte(Self::JSON_OBJ_BEGIN)?;
                    self.stack.push(State::ExpectingFirstDictFieldOrEnd);
                }
                b'e' => {
                    // End of list or dictionary (not end of integer)
                    self.end_bencoded_value(writer)?;
                }
                _ => {
                    panic!("{}", format!("unexpected byte {} ({})", byte, byte as char));
                }
            }

            if self.debug {
                println!("stack: {}", self.stack);
                self.byte_reader.print_captured_input();
                writer.print_captured_output();
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
    pub fn begin_bencoded_value<W: Writer>(&mut self, writer: &mut W) -> io::Result<()> {
        match self.stack.peek() {
            State::Initial => {}
            State::ExpectingFirstListItemOrEnd => {
                self.stack.swap_top(State::ExpectingNextListItem);
            }
            State::ExpectingNextListItem => {
                writer.write_byte(Self::JSON_ARRAY_ITEMS_SEPARATOR)?;
            }
            State::ExpectingFirstDictFieldOrEnd => {
                self.stack.swap_top(State::ExpectingDictFieldValue);
            }
            State::ExpectingDictFieldValue => {
                writer.write_byte(Self::JSON_OBJ_FIELD_KEY_VALUE_SEPARATOR)?;
                self.stack.swap_top(State::ExpectingDictFieldKey);
            }
            State::ExpectingDictFieldKey => {
                writer.write_byte(Self::JSON_OBJ_FIELDS_SEPARATOR)?;
                self.stack.swap_top(State::ExpectingDictFieldValue);
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
    pub fn end_bencoded_value<W: Writer>(&mut self, writer: &mut W) -> io::Result<()> {
        match self.stack.peek() {
            State::ExpectingFirstListItemOrEnd | State::ExpectingNextListItem => {
                writer.write_byte(Self::JSON_ARRAY_END)?;
                self.stack.pop();
            }
            State::ExpectingFirstDictFieldOrEnd | State::ExpectingDictFieldKey => {
                writer.write_byte(Self::JSON_OBJ_END)?;
            }
            State::ExpectingDictFieldValue | State::Initial => {
                // todo: pass the type of value (list or dict) to customize the error message
                panic!("error parsing end of list or dictionary, unexpected initial state on the stack")
            }
        }
        // todo: sp < stack. What this conditions does in the C implementation?

        Ok(())
    }

    pub fn opt_captured_output<W: Writer>(&self, writer: &mut W) -> Option<String> {
        writer.get_captured_output()
    }
}

#[cfg(test)]
mod tests {

    use super::BencodeParser;

    #[test]
    fn it_should_allow_writing_to_a_byte_vector() {
        let mut output = Vec::new();

        let mut parser = BencodeParser::new(&b"i0e"[..]);

        parser
            .write_bytes(&mut output)
            .expect("Bencode to JSON conversion failed");

        assert_eq!(output, vec!(48));
    }

    #[test]
    fn it_should_allow_writing_to_a_string() {
        let mut output = String::new();

        let mut parser = BencodeParser::new(&b"i0e"[..]);

        parser
            .write_str(&mut output)
            .expect("Bencode to JSON conversion failed");

        assert_eq!(output, "0".to_string());
    }

    fn to_json(input_buffer: &[u8]) -> String {
        let mut output = String::new();

        let mut parser = BencodeParser::new(input_buffer);

        parser
            .write_str(&mut output)
            .expect("Bencode to JSON conversion failed");

        output
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
