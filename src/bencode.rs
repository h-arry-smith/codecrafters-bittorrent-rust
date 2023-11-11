use std::{collections::HashMap, fmt::Display};

#[derive(Debug, PartialEq)]
pub enum Value {
    String(String),
    Blob(Vec<u8>),
    Number(i64),
    List(Vec<Value>),
    Dictionary(HashMap<String, Value>),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(string) => write!(f, "\"{}\"", string),
            Value::Blob(blob) => write!(f, "{:?}", blob),
            Value::Number(number) => write!(f, "{}", number),
            Value::List(list) => {
                write!(f, "[")?;
                for (index, value) in list.iter().enumerate() {
                    write!(f, "{}", value)?;
                    if index < list.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "]")
            }
            Value::Dictionary(map) => {
                write!(f, "{{")?;
                let mut key_value_strings = Vec::new();
                let mut sorted_keys = map.keys().collect::<Vec<&String>>();
                sorted_keys.sort();

                for key in sorted_keys.iter() {
                    let value = map.get(*key).unwrap();

                    let string = match value {
                        // Note: Special casing the list formatting while in dicts to match codecrafter tests.
                        Value::List(list) => {
                            let mut list_strings = Vec::new();
                            for value in list.iter() {
                                list_strings.push(format!("{}", value));
                            }
                            format!("\"{}\":[{}]", key, list_strings.join(","))
                        }
                        _ => format!("\"{}\":{}", key, value),
                    };

                    key_value_strings.push(string);
                }

                let joined_key_value_strings = key_value_strings.join(",");
                write!(f, "{}", joined_key_value_strings)?;
                write!(f, "}}")
            }
        }
    }
}

pub struct Bencode<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Bencode<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub fn decode(&mut self) -> Value {
        match self.peek() {
            Some('d') => self.decode_dictionary(),
            Some('l') => self.decode_list(),
            Some('i') => self.decode_integer(),
            Some(c) if c.is_ascii_digit() => self.decode_string(),
            Some(c) => panic!("Unexpected character: {}", c),
            None => panic!("Unexpected end of input"),
        }
    }

    // TODO: encode needs to take a custom value structure to differentiate between blobs and arrays of numbers
    #[allow(dead_code)]
    pub fn encode(value: &Value) -> Vec<u8> {
        match value {
            Value::String(string) => {
                let mut encoded = string.len().to_string().into_bytes();
                encoded.push(b':');
                encoded.extend_from_slice(string.as_bytes());
                encoded
            }
            Value::Blob(bytes) => {
                let mut encoded = bytes.len().to_string().into_bytes();
                encoded.push(b':');
                encoded.extend_from_slice(bytes);
                encoded
            }
            Value::Number(number) => {
                let mut encoded = b"i".to_vec();
                encoded.extend_from_slice(number.to_string().as_bytes());
                encoded.push(b'e');
                encoded
            }
            Value::List(array) => {
                let mut encoded = b"l".to_vec();
                for value in array.iter() {
                    encoded.extend_from_slice(&Self::encode(value));
                }
                encoded.push(b'e');
                encoded
            }
            Value::Dictionary(map) => {
                let mut encoded = b"d".to_vec();
                let mut sorted_keys = map.keys().collect::<Vec<&String>>();
                sorted_keys.sort();

                for key in sorted_keys.iter() {
                    let value = map.get(*key).unwrap();
                    encoded.extend_from_slice(&Self::encode(&Value::String(key.to_string())));
                    encoded.extend_from_slice(&Self::encode(value));
                }

                encoded.push(b'e');
                encoded
            }
        }
    }

    fn decode_string(&mut self) -> Value {
        let string_length = self.decode_integer_number();
        self.consume(':').expect("Expected ':' after string length");

        let string_slice = &self.bytes[self.position..self.position + string_length as usize];
        self.position += string_length as usize;

        if let Ok(string) = std::str::from_utf8(string_slice) {
            Value::String(string.to_string())
        } else {
            Value::Blob(string_slice.to_vec())
        }
    }

    fn decode_integer(&mut self) -> Value {
        self.consume('i').expect("Expected 'i' at start of integer");
        let integer = self.decode_integer_number();
        self.consume('e').expect("Expected 'e' at end of integer");

        Value::Number(integer)
    }

    fn decode_list(&mut self) -> Value {
        self.consume('l').expect("Expected 'l' at start of list");

        let mut values = Vec::new();
        while self.peek() != Some('e') {
            values.push(self.decode());
        }

        self.consume('e').expect("Expected 'e' at end of list");

        Value::List(values)
    }

    fn decode_dictionary(&mut self) -> Value {
        self.consume('d')
            .expect("Expected 'd' at start of dictionary");

        let mut map = HashMap::new();
        while self.peek() != Some('e') {
            let key = self
                .decode_string()
                .to_string()
                .strip_prefix('\"')
                .unwrap()
                .strip_suffix('\"')
                .unwrap()
                .to_string();
            let value = self.decode();
            map.insert(key, value);
        }

        self.consume('e')
            .expect("Expected 'e' at end of dictionary");
        Value::Dictionary(map)
    }

    fn decode_integer_number(&mut self) -> i64 {
        let mut number_string = String::new();
        loop {
            match self.peek() {
                Some(c) if c.is_ascii_digit() || c == '-' => {
                    number_string.push(c);
                    self.next();
                }
                _ => break,
            }
        }
        number_string.parse::<i64>().expect("Invalid number")
    }

    fn next(&mut self) -> Option<char> {
        let next_char = self.bytes.get(self.position);
        self.position += 1;
        next_char.map(|b| *b as char)
    }

    fn peek(&self) -> Option<char> {
        self.bytes.get(self.position).map(|b| *b as char)
    }

    fn consume(&mut self, expected: char) -> Result<char, String> {
        match self.next() {
            Some(c) if c == expected => Ok(c),
            Some(c) => Err(format!("Was expecting {}, but got {}.", expected, c)),
            None => Err(format!(
                "Was expecting {}, but reached end of input",
                expected
            )),
        }
    }
}

mod tests {
    #[test]
    fn hello_string() {
        let mut bencode = super::Bencode::new("5:hello".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(decoded_value, super::Value::String("hello".to_string()));
    }

    #[test]
    fn long_string() {
        let mut bencode = super::Bencode::new("11:hello world".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(
            decoded_value,
            super::Value::String("hello world".to_string())
        );
    }

    #[test]
    fn positive_integer() {
        let mut bencode = super::Bencode::new("i123e".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(decoded_value, super::Value::Number(123.into()));
    }

    #[test]
    fn negative_integer() {
        let mut bencode = super::Bencode::new("i-123e".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(decoded_value, super::Value::Number((-123).into()));
    }

    #[test]
    fn simple_list() {
        let mut bencode = super::Bencode::new("l4:spam4:eggse".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(
            decoded_value,
            super::Value::List(vec![
                super::Value::String("spam".to_string()),
                super::Value::String("eggs".to_string())
            ])
        );
    }

    #[test]
    fn multi_type_list() {
        let mut bencode = super::Bencode::new("li123e5:helloe".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(
            decoded_value,
            super::Value::List(vec![
                super::Value::Number(123.into()),
                super::Value::String("hello".to_string())
            ])
        );
    }

    #[test]
    fn list_inside_a_list() {
        let mut bencode = super::Bencode::new("lli467e9:blueberryee".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(
            decoded_value,
            super::Value::List(vec![super::Value::List(vec![
                super::Value::Number(467.into()),
                super::Value::String("blueberry".to_string())
            ])])
        );
    }

    #[test]
    fn dictionary() {
        let mut bencode = super::Bencode::new("d3:foo3:bar5:helloi52ee".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(
            decoded_value,
            super::Value::Dictionary(
                vec![
                    (
                        "\"foo\"".to_string(),
                        super::Value::String("bar".to_string())
                    ),
                    ("\"hello\"".to_string(), super::Value::Number(52.into()))
                ]
                .into_iter()
                .collect()
            )
        );
    }
}
