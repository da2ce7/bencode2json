use std::io::{self, Read};
use std::str;

#[derive(Debug)]
pub struct Stack {
    items: Vec<StackItem>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum StackItem {
    I, // Integer (only used to initialize the stack).
    // L -> M (M can repeat n times)
    L, // LIST (swap L -> M).
    M, // Put the delimiter (',') between list items.
    // D -> E -> F (E -> F transition can repeat n times)
    D, // DICTIONARY (swap D -> E).
    E, // Put the delimiter (':') between key and value in key/value paris (swap E -> F).
    F, // Put the delimiter (',') between key/value pairs in dictionaries (swap F -> E).
}

impl Default for Stack {
    fn default() -> Self {
        let items = vec![StackItem::I];
        Self { items }
    }
}

impl Stack {
    fn push(&mut self, item: StackItem) {
        self.items.push(item);
    }

    fn pop(&mut self) {
        self.items.pop();
    }

    fn swap_top(&mut self, new_item: StackItem) {
        self.items.pop();
        self.push(new_item);
    }

    fn top(&self) -> StackItem {
        match self.items.last() {
            Some(top) => top.clone(),
            None => panic!("empty stack!"),
        }
    }
}

pub struct BencodeParser<R: Read> {
    pub debug: bool,
    pub json: String,
    pub iter: u64,
    pub pos: u64,
    reader: R,
    stack: Stack,
    captured_input: Option<Vec<u8>>,
}

// todo: we don't have an integer parser because we simple print all bytes between
// the start (`i`) and end (`e`) delimiters for integer values. However, what
// should happen when the integer contains a byte that is not a digit. For
// example: b"i12G345e"?

#[derive(Default, Debug)]
struct StringParser {
    // String length
    bytes_for_string_length: Vec<u8>,
    string_length: usize,

    // String value bytes
    string_bytes: Vec<u8>,
    string_bytes_counter: usize,
}

impl StringParser {
    fn new_string_starting_with(&mut self, byte: u8) {
        self.new_string();
        self.add_length_byte(byte);
    }

    fn new_string(&mut self) {
        self.bytes_for_string_length = Vec::new();
        self.string_length = 0;
        self.string_bytes = Vec::new();
        self.string_bytes_counter = 0;
    }

    fn add_length_byte(&mut self, byte: u8) {
        // todo: should we fail here is the byte is not a digit (0..9)?
        // or we can wait until we try to convert all bytes in the into a number?
        self.bytes_for_string_length.push(byte);
    }

    fn add_byte(&mut self, byte: u8) {
        // todo: return an error if we try to push a new byte but the end of the
        // string has been reached.
        self.string_bytes.push(byte);
        self.string_bytes_counter += 1;
    }

    /// This function is called when we receive the ':' byte which is the
    /// delimiter for the end of bytes representing the string length.
    fn process_end_of_string_length(&mut self) {
        // todo: maybe we should simply fail when we receive a byte that is not a digit (0..9).
        // This error cannot be understood by users because we first convert into a UTF-8 string
        // and later into a number.
        let length_str = str::from_utf8(&self.bytes_for_string_length)
            .expect("invalid string length, non UTF-8 string length");

        //println!("length_str: {length_str}");

        self.string_length = length_str
            .parse::<usize>()
            .expect("invalid string length, non zero or positive integer");

        //println!("string_length_number: {string_length}");
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

    fn json(&self) -> String {
        format!("\"{}\"", self.utf8())
    }

    fn bytes_to_hex(data: &[u8]) -> String {
        format!("<hex>{}</hex>", hex::encode(data))
    }
}

impl<R: Read> BencodeParser<R> {
    pub fn new(reader: R) -> Self {
        BencodeParser {
            debug: false,
            reader,
            stack: Stack::default(),
            json: String::new(),
            pos: 0,
            iter: 1,
            captured_input: Some(Vec::new()),
        }
    }

    pub fn struct_hlp(&mut self) {
        match self.stack.top() {
            StackItem::D => {
                self.stack.swap_top(StackItem::E);
            }
            StackItem::E => {
                self.json.push(':');
                self.stack.swap_top(StackItem::F);
            }
            StackItem::F => {
                self.json.push(',');
                self.stack.swap_top(StackItem::E);
            }
            StackItem::L => {
                self.stack.swap_top(StackItem::M);
            }
            StackItem::M => self.json.push(','),
            StackItem::I => {}
        }
    }

    fn dump_int(&mut self) -> io::Result<()> {
        let mut st = 0;

        loop {
            let byte = match self.read_byte() {
                Ok(byte) => byte,
                Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                    //println!("Reached the end of file.");
                    panic!("unexpected end of input parsing integer");
                }
                Err(err) => return Err(err),
            };

            let char = byte as char;

            if char.is_ascii_digit() {
                st = 2;
                self.json.push(char);
            } else if char == 'e' && st == 2 {
                return Ok(());
            } else if char == '-' && st == 0 {
                st = 1;
                self.json.push(char);
            } else {
                panic!("invalid integer");
            }
        }
    }

    fn dump_str(&mut self, byte: u8) -> io::Result<()> {
        let mut string_parser = StringParser::default();

        string_parser.new_string_starting_with(byte);

        // Parse length

        loop {
            let byte = match self.read_byte() {
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
            let byte = match self.read_byte() {
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

        self.json.push_str(&string_parser.json());

        //println!("string_parser {string_parser:#?}");

        Ok(())
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
        loop {
            let byte = match self.read_byte() {
                Ok(byte) => byte,
                Err(ref err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                    //println!("Reached the end of file.");
                    break;
                }
                Err(err) => return Err(err),
            };

            if self.debug {
                println!("iter: {}", self.iter);
                println!("pos: {}", self.pos);
                println!("byte: {} ({})", byte, byte as char);
            }

            match byte {
                b'i' => {
                    // Begin of integer
                    self.struct_hlp();
                    self.dump_int().expect("invalid integer");
                }
                b'0'..=b'9' => {
                    // Begin of string
                    self.struct_hlp();
                    self.dump_str(byte).expect("invalid string");
                }
                b'l' => {
                    // Begin of list
                    self.struct_hlp();
                    self.json.push('[');
                    self.stack.push(StackItem::L);
                }
                b'd' => {
                    // Begin of dictionary
                    self.struct_hlp();
                    self.json.push('{');
                    self.stack.push(StackItem::D);
                }
                b'e' => {
                    // End of list or dictionary (not end of integer)
                    match self.stack.top() {
                        StackItem::L | StackItem::M => {
                            self.json.push(']');
                            self.stack.pop();
                        }
                        StackItem::D | StackItem::F => {
                            self.json.push('}');
                        }
                        StackItem::E | StackItem::I => {
                            panic!("error parsing end, unexpected item I on the stack")
                        }
                    }
                    // todo: sp < stack
                }
                _ => {
                    panic!("{}", format!("unexpected byte {} ({})", byte, byte as char));
                }
            }

            if self.debug {
                println!("stack: {:?}", self.stack);
                //println!("string_parser: {:#?}", self.string_parser);
                match &self.captured_input {
                    Some(captured_input) => match str::from_utf8(captured_input) {
                        Ok(string) => println!("input: {string}"),
                        Err(_) => println!("input: {captured_input:#?}"),
                    },
                    None => {}
                }
                println!("output: {}", self.json);
                println!();
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

        let byte = byte[0];

        if let Some(ref mut captured_input) = self.captured_input {
            captured_input.push(byte);
        }

        Ok(byte)
    }
}

#[cfg(test)]
mod tests {

    use crate::BencodeParser;

    fn to_json(bytes: &[u8]) -> String {
        let mut parser = BencodeParser::new(bytes);
        parser.parse().expect("bencoded to JSON conversion failed");
        parser.json
    }

    mod integers {
        use crate::tests::to_json;

        #[test]
        fn zero() {
            assert_eq!(to_json(b"i0e"), "0".to_string());
        }

        #[test]
        fn one_digit_integer() {
            assert_eq!(to_json(b"i0e"), "0".to_string());
        }

        #[test]
        fn two_digits_integer() {
            assert_eq!(to_json(b"i42e"), "42".to_string());
        }

        // todo: all encodings with a leading zero, such as i03e, are invalid, other
        // than i0e, which of course corresponds to 0.
    }

    mod strings {
        use crate::tests::to_json;

        /* todo:
        - String with size 0 (empty string) are allowed: b"0:"
        - String ending with reserved charts 'i', 'l', 'd', 'l', ':', 'e'
        - String ending with digit
        */

        #[test]
        fn utf8() {
            assert_eq!(to_json(b"4:spam"), "\"spam\"".to_string());
        }

        #[test]
        fn non_utf8() {
            assert_eq!(
                to_json(b"4:\xFF\xFE\xFD\xFC"),
                "\"<hex>fffefdfc</hex>\"".to_string()
            );
        }

        /* todo:
           - String containing special chars: 'i', ':', 'l', 'd', 'e'. The
             bencoded string can contain reserved chars in bencode format.
        */
    }

    mod lists {
        use crate::tests::to_json;

        #[test]
        fn empty_list() {
            assert_eq!(to_json(b"le"), "[]".to_string());
        }

        mod with_one_item {
            use crate::tests::to_json;

            #[test]
            fn integer() {
                assert_eq!(to_json(b"li42ee"), "[42]".to_string());
            }

            #[test]
            fn utf8_string() {
                assert_eq!(to_json(b"l4:spame"), "[\"spam\"]".to_string());
            }

            #[test]
            fn non_utf8_string() {
                assert_eq!(
                    to_json(b"l4:\xFF\xFE\xFD\xFCe"),
                    "[\"<hex>fffefdfc</hex>\"]".to_string()
                );
            }

            mod of_type_list {
                use crate::tests::to_json;

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
                    assert_eq!(to_json(b"ll4:spamee"), "[[\"spam\"]]".to_string());
                }

                #[test]
                fn one_nested_list_which_contains_two_utf_8_strings() {
                    assert_eq!(
                        to_json(b"ll5:alice3:bobee"),
                        "[[\"alice\",\"bob\"]]".to_string()
                    );
                }
            }

            /* todo:
                - With one dictionary
            */
        }

        mod with_two_items_of_the_same_type {
            use crate::tests::to_json;

            #[test]
            fn two_integers() {
                assert_eq!(to_json(b"li42ei43ee"), "[42,43]".to_string());
            }

            #[test]
            fn two_utf8_strings() {
                assert_eq!(
                    to_json(b"l5:alice3:bobe"),
                    "[\"alice\",\"bob\"]".to_string()
                );
            }

            #[test]
            fn two_non_utf8_strings() {
                assert_eq!(
                    to_json(b"l2:\xFF\xFE2:\xFD\xFCe"),
                    "[\"<hex>fffe</hex>\",\"<hex>fdfc</hex>\"]".to_string()
                );
            }
        }

        mod with_two_items_of_different_types {
            use crate::tests::to_json;

            #[test]
            fn integer_and_utf8_string() {
                assert_eq!(to_json(b"li42e5:alicee"), "[42,\"alice\"]".to_string());
            }

            #[test]
            fn integer_and_non_utf8_string() {
                assert_eq!(
                    to_json(b"li42e2:\xFF\xFEe"),
                    "[42,\"<hex>fffe</hex>\"]".to_string()
                );
            }

            #[test]
            fn utf8_string_and_integer() {
                assert_eq!(to_json(b"l5:alicei42ee"), "[\"alice\",42]".to_string());
            }

            #[test]
            fn non_utf8_string_and_an_integer() {
                assert_eq!(
                    to_json(b"l2:\xFF\xFEi42ee"),
                    "[\"<hex>fffe</hex>\",42]".to_string()
                );
            }

            mod integer_and_list {
                use crate::tests::to_json;

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
        use crate::tests::to_json;

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
            assert_eq!(to_json(b"de"), "{}".to_string());
        }

        mod with_one_key_of_type {
            use crate::tests::to_json;

            #[test]
            fn integer() {
                assert_eq!(to_json(b"d3:fooi42ee"), "{\"foo\":42}".to_string());
            }

            #[test]
            fn utf8_string() {
                assert_eq!(to_json(b"d3:bar4:spame"), "{\"bar\":\"spam\"}".to_string());
            }

            #[test]
            fn non_utf8_string() {
                assert_eq!(
                    to_json(b"d3:bar2:\xFF\xFEe"),
                    "{\"bar\":\"<hex>fffe</hex>\"}".to_string()
                );
            }
        }

        mod with_two_keys_of_the_same_type {
            use crate::tests::to_json;

            #[test]
            fn two_integers() {
                assert_eq!(
                    to_json(b"d3:bari42e3:fooi43ee"),
                    "{\"bar\":42,\"foo\":43}".to_string()
                );
            }

            #[test]
            fn two_utf8_strings() {
                assert_eq!(
                    to_json(b"d3:bar4:spam3:foo5:alicee"),
                    "{\"bar\":\"spam\",\"foo\":\"alice\"}".to_string()
                );
            }
        }
    }
}
