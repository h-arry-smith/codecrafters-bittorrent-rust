use serde_json::Map;

pub struct Bencode<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Bencode<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub fn decode(&mut self) -> serde_json::Value {
        match self.peek() {
            Some('d') => self.decode_dictionary(),
            Some('l') => self.decode_list(),
            Some('i') => self.decode_integer(),
            Some(c) if c.is_ascii_digit() => self.decode_string(),
            Some(c) => panic!("Unexpected character: {}", c),
            None => panic!("Unexpected end of input"),
        }
    }

    fn decode_string(&mut self) -> serde_json::Value {
        let string_length = self.decode_integer_number();
        self.consume(':').expect("Expected ':' after string length");

        let string_slice = &self.bytes[self.position..self.position + string_length as usize];
        self.position += string_length as usize;

        if let Ok(string) = std::str::from_utf8(string_slice) {
            serde_json::Value::String(string.to_string())
        } else {
            serde_json::Value::Array(
                string_slice
                    .iter()
                    .map(|b| serde_json::Value::Number((*b).into()))
                    .collect(),
            )
        }
    }

    fn decode_integer(&mut self) -> serde_json::Value {
        self.consume('i').expect("Expected 'i' at start of integer");
        let integer = self.decode_integer_number();
        self.consume('e').expect("Expected 'e' at end of integer");

        serde_json::Value::Number(integer.into())
    }

    fn decode_list(&mut self) -> serde_json::Value {
        self.consume('l').expect("Expected 'l' at start of list");

        let mut values = Vec::new();
        while self.peek() != Some('e') {
            values.push(self.decode());
        }

        self.consume('e').expect("Expected 'e' at end of list");

        serde_json::Value::Array(values)
    }

    fn decode_dictionary(&mut self) -> serde_json::Value {
        self.consume('d')
            .expect("Expected 'd' at start of dictionary");

        let mut map = Map::new();
        while self.peek() != Some('e') {
            let key = self.decode_string().to_string();
            let key_stripped = key.strip_prefix('"').unwrap().strip_suffix('"').unwrap();
            let value = self.decode();
            map.insert(key_stripped.to_string(), value);
        }

        self.consume('e')
            .expect("Expected 'e' at end of dictionary");
        serde_json::Value::Object(map)
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
        assert_eq!(
            decoded_value,
            serde_json::Value::String("hello".to_string())
        );
    }

    #[test]
    fn long_string() {
        let mut bencode = super::Bencode::new("11:hello world".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(
            decoded_value,
            serde_json::Value::String("hello world".to_string())
        );
    }

    #[test]
    fn positive_integer() {
        let mut bencode = super::Bencode::new("i123e".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(decoded_value, serde_json::Value::Number(123.into()));
    }

    #[test]
    fn negative_integer() {
        let mut bencode = super::Bencode::new("i-123e".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(decoded_value, serde_json::Value::Number((-123).into()));
    }

    #[test]
    fn simple_list() {
        let mut bencode = super::Bencode::new("l4:spam4:eggse".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(
            decoded_value,
            serde_json::json!(["spam".to_string(), "eggs".to_string()])
        );
    }

    #[test]
    fn multi_type_list() {
        let mut bencode = super::Bencode::new("li123e5:helloe".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(decoded_value, serde_json::json!([123, "hello".to_string()]));
    }

    #[test]
    fn list_inside_a_list() {
        let mut bencode = super::Bencode::new("lli467e9:blueberryee".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(
            decoded_value,
            serde_json::json!([serde_json::json!([467, "blueberry".to_string()])])
        );
    }

    #[test]
    fn dictionary() {
        let mut bencode = super::Bencode::new("d3:foo3:bar5:helloi52ee".as_bytes());
        let decoded_value = bencode.decode();
        assert_eq!(
            decoded_value,
            serde_json::json!({
                "foo".to_string(): "bar".to_string(),
                "hello".to_string(): 52
            })
        );
    }
}
