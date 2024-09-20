use std::io::{self, Read};
use std::str;

#[derive(Debug)]
pub struct Stack {
    items: Vec<StackItem>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum StackItem {
    I, // INTEGER
    L, // LIST
    M, // NESTED_LIST
    D, // DICTIONARY
    F, // NESTED_DICTIONARY?
    E, // END OF INTEGER, LIST OR DICTIONARY
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
                b'd' => {
                    self.struct_hlp();
                    self.json.push('{');
                    self.stack.push(StackItem::D);
                }
                b'l' => {
                    self.struct_hlp();
                    self.json.push('[');
                    self.stack.push(StackItem::L);
                }
                b'i' => {
                    self.struct_hlp();
                    self.dump_int().expect("invalid integer");
                }
                b'0'..=b'9' => {
                    self.struct_hlp();
                    self.dump_str(byte).expect("invalid string");
                }
                b'e' => {
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

    mod integers {
        use crate::BencodeParser;

        #[test]
        fn integer() {
            let data = b"i42e";
            let mut parser = BencodeParser::new(&data[..]);
            parser.parse().unwrap();
            assert_eq!(parser.json, "42".to_string());
        }

        // todo: all encodings with a leading zero, such as i03e, are invalid, other
        // than i0e, which of course corresponds to 0.
    }

    mod strings {
        use crate::BencodeParser;

        /* todo:
        - String with size 0 (empty string) are allowed: b"0:"
        - String ending with reserved charts 'i', 'l', 'd', 'l', ':', 'e'
        - String ending with digit
        */

        #[test]
        fn utf8() {
            let data = b"4:spam";

            //  1   2   3   4   5   6 (pos)
            //  4   :   s   p   a   m (byte)
            // 52  58 115 112  97 109 (byte decimal)

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

                /* todo:
                    - Main list empty, nested list two items:
                        - Nested list with non UTF-8 string
                */

                #[test]
                fn two_nested_empty_list() {
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
                fn three_nested_empty_lists() {
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
                fn one_nested_list_which_contains_one_integer() {
                    // List with one empty list: lli42eee
                    //   1   2   3   4   5   6   7   8 (pos)
                    //   l   l   i   4   2   e   e   e (byte)
                    // 108 108 105  52  50 101 101 101 (byte decimal)

                    let data = b"lli42eee";

                    let mut parser = BencodeParser::new(&data[..]);
                    parser.parse().unwrap();

                    assert_eq!(parser.json, "[[42]]".to_string());
                }

                #[test]
                fn one_nested_list_which_contains_two_integers() {
                    // List with one empty list: lli42ei43eee
                    //   1   2   3   4   5   6   7   8   9  10 11  12 (pos)
                    //   l   l   i   4   2   e   i   4   3   e  e   e (byte)
                    // 108 108 105  52  50 101 105  52 105 101 101 101 (byte decimal)

                    let data = b"lli42ei43eee";

                    let mut parser = BencodeParser::new(&data[..]);
                    parser.parse().unwrap();

                    assert_eq!(parser.json, "[[42,43]]".to_string());
                }

                #[test]
                fn one_nested_list_which_contains_one_utf_8_string() {
                    // List with one empty list: ll4:spamee
                    //   1   2   3   4   5   6   7   8   9  10 (pos)
                    //   l   l   4   :   s   p   a   m   e   e (byte)
                    // 108 108  52  58 115 112  97 109 101 101  (byte decimal)

                    let data = b"ll4:spamee";

                    let mut parser = BencodeParser::new(&data[..]);
                    parser.parse().unwrap();

                    assert_eq!(parser.json, "[[\"spam\"]]".to_string());
                }

                #[test]
                fn one_nested_list_which_contains_two_utf_8_strings() {
                    // List with one empty list: ll5:alice3:bobee
                    //   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16 (pos)
                    //   l   l   5   :   a   l   i   c   e   3   :   b   o   b   e   e (byte)
                    // 108 108  53  58  97 108 105  99 101  51  58  98 111  98 101 101 (byte decimal)

                    let data = b"ll5:alice3:bobee";

                    let mut parser = BencodeParser::new(&data[..]);
                    parser.parse().unwrap();

                    assert_eq!(parser.json, "[[\"alice\",\"bob\"]]".to_string());
                }
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

            mod integer_and_list {
                use crate::BencodeParser;

                #[test]
                fn second_item_empty_list() {
                    let data = b"li42elee";

                    let mut parser = BencodeParser::new(&data[..]);
                    parser.parse().unwrap();

                    assert_eq!(parser.json, "[42,[]]".to_string());
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
            fn two_integers() {
                // Dictionary with two integers: d3:bari42e3:fooi43ee
                //   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20 (pos)
                //   d   3   :   b   a   r   i   4   2   e   3   :   f   o   o   i   4   3   e   e (byte)
                // 100  51  58  98  97 114 105  52  50 101  51  58 102 111 111 105  52  51 101 101 (byte decimal)

                let data = b"d3:bari42e3:fooi43ee";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(parser.json, "{\"bar\":42,\"foo\":43}".to_string());
            }

            #[test]
            fn two_utf8_strings() {
                // Dictionary with two UTF-8 strings: d3:bar4:spam3:foo5:alicee
                //   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25 (pos)
                //   d   3   :   b   a   r   4   :   s   p   a   m   3   :   f   o   o   5   :   a   l   i   c   e   e (byte)
                // 100  51  58  98  97 114  52  58 115 112  97 109  51  58 102 111 111  53  58  97 108 105  99 101 101 (byte decimal)

                let data = b"d3:bar4:spam3:foo5:alicee";

                let mut parser = BencodeParser::new(&data[..]);
                parser.parse().unwrap();

                assert_eq!(
                    parser.json,
                    "{\"bar\":\"spam\",\"foo\":\"alice\"}".to_string()
                );
            }
        }
    }
}
