use sha1::Digest;
use std::{collections::HashMap, fs::File, io::Read, path::Path};

use crate::bencode::{Bencode, Value};

#[derive(Debug)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}

impl Torrent {
    pub fn open<P: AsRef<Path>>(path: P) -> Self {
        let mut file = File::open(path).expect("Failed to open torrent file");
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf)
            .expect("Failed to read torrent file");

        let decoded = Bencode::new(&buf).decode();
        let decoded_hash_map = match decoded {
            Value::Dictionary(hash_map) => hash_map,
            _ => panic!("Expected torrent file to decode to a dictionary"),
        };

        let announce = match decoded_hash_map.get("announce") {
            Some(Value::String(string)) => string.clone(),
            _ => panic!("Decoded torrent file did not contain an announce string"),
        };

        let info_hash_map = match decoded_hash_map.get("info") {
            Some(Value::Dictionary(hash_map)) => hash_map,
            _ => panic!("Decoded torrent file did not contain an info dictionary"),
        };

        let info: Info = info_hash_map.into();

        Self { announce, info }
    }

    pub fn info_hash(&self) -> String {
        let info_hash_map = (&self.info).into();
        let encoded = Bencode::encode(&Value::Dictionary(info_hash_map));

        let mut hasher = sha1::Sha1::new();
        hasher.update(&encoded);
        let hash = hasher.finalize();
        format!("{:x}", hash)
    }
}

#[derive(Debug)]
pub struct Info {
    pub length: usize,
    pub name: String,
    pub piece_length: usize,
    pub pieces: Vec<[u8; 20]>,
}

impl From<&HashMap<String, Value>> for Info {
    fn from(value: &HashMap<String, Value>) -> Self {
        let length = match value.get("length") {
            Some(Value::Number(number)) => *number as usize,
            _ => panic!("Decoded info dictionary did not contain a length number"),
        };

        let name = match value.get("name") {
            Some(Value::String(string)) => string.clone(),
            _ => panic!("Decoded info dictionary did not contain a name string"),
        };

        let piece_length = match value.get("piece length") {
            Some(Value::Number(number)) => *number as usize,
            _ => panic!("Decoded info dictionary did not contain a piece length number"),
        };

        let all_pieces = match value.get("pieces") {
            Some(Value::Blob(blob)) => blob,
            _ => panic!("Decoded info dictionary did not contain a pieces blob"),
        };

        let pieces = all_pieces
            .chunks_exact(20)
            .map(|chunk| {
                let mut array = [0; 20];
                array.copy_from_slice(chunk);
                array
            })
            .collect();

        Self {
            length,
            name,
            piece_length,
            pieces,
        }
    }
}

impl From<&Info> for HashMap<String, Value> {
    fn from(value: &Info) -> Self {
        let pieces = value
            .pieces
            .iter()
            .flat_map(|array| array.to_vec())
            .collect();

        let mut hash_map = HashMap::new();
        hash_map.insert("length".to_string(), Value::Number(value.length as i64));
        hash_map.insert("name".to_string(), Value::String(value.name.clone()));
        hash_map.insert(
            "piece length".to_string(),
            Value::Number(value.piece_length as i64),
        );
        hash_map.insert("pieces".to_string(), Value::Blob(pieces));

        hash_map
    }
}
