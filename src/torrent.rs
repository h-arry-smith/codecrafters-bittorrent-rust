use std::{fs::File, io::Read, path::Path};

use serde::{
    de::{SeqAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::bencode::Bencode;

#[derive(Debug, Deserialize)]
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

        serde_json::from_value(decoded).expect("Failed to parse torrent file")
    }
}

#[derive(Debug, Deserialize)]
pub struct Info {
    pub length: usize,
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: usize,
    pub pieces: HashList,
}

#[derive(Debug)]
pub struct HashList {
    pub hashes: Vec<[u8; 20]>,
}

impl<'de> Deserialize<'de> for HashList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(HashListVisitor)
    }
}

struct HashListVisitor;

impl<'de> Visitor<'de> for HashListVisitor {
    type Value = HashList;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a sequence of hashes")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        // if sequence isn't divisible by twenty then error out
        let mut hashes = Vec::new();
        let mut current_hash = Vec::new();

        while let Some(value) = seq.next_element()? {
            current_hash.push(value);

            if current_hash.len() == 20 {
                let mut hash = [0; 20];
                hash.copy_from_slice(&current_hash);
                hashes.push(hash);
                current_hash.clear();
            }
        }

        Ok(HashList { hashes })
    }
}
