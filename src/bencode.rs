pub struct Bencode<'a> {
    string: &'a str,
    position: usize,
}

impl<'a> Bencode<'a> {
    pub fn new(string: &'a str) -> Self {
        Self {
            string,
            position: 0,
        }
    }

    pub fn decode(&mut self) -> serde_json::Value {
        match self.peek() {
            Some(_) => self.decode_string(),
            None => panic!("Unexpected end of input"),
        }
    }

    fn decode_string(&mut self) -> serde_json::Value {
        let string_length = self.decode_number();
        self.consume(':').expect("Expected ':' after string length");

        let string_slice = &self.string[self.position..self.position + string_length as usize];
        self.position += string_length as usize;
        serde_json::Value::String(string_slice.to_string())
    }

    fn decode_number(&mut self) -> i64 {
        let mut number_string = String::new();
        loop {
            match self.peek() {
                Some(c) if c.is_ascii_digit() => {
                    number_string.push(c);
                    self.next();
                }
                _ => break,
            }
        }
        number_string.parse::<i64>().expect("Invalid number")
    }

    fn next(&mut self) -> Option<char> {
        let next_char = self.string.chars().nth(self.position);
        self.position += 1;
        next_char
    }

    fn peek(&self) -> Option<char> {
        self.string.chars().nth(self.position)
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
        let mut bencode = super::Bencode::new("5:hello");
        let decoded_value = bencode.decode();
        assert_eq!(
            decoded_value,
            serde_json::Value::String("hello".to_string())
        );
    }

    #[test]
    fn long_string() {
        let mut bencode = super::Bencode::new("11:hello world");
        let decoded_value = bencode.decode();
        assert_eq!(
            decoded_value,
            serde_json::Value::String("hello world".to_string())
        );
    }
}