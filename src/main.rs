use std::{
    io::{Read, Write},
    net::{SocketAddrV4, TcpStream},
};

use crate::bencode::Bencode;
use clap::{Parser, Subcommand};
use torrent::Torrent;

mod bencode;
mod torrent;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Decode { encoded_value: String },
    Info { torrent_file: String },
    Peers { torrent_file: String },
    Handshake { torrent_file: String, addr: String },
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Decode { encoded_value } => {
            let decoded_value = Bencode::new(encoded_value.as_bytes()).decode();
            println!("{}", decoded_value)
        }
        Commands::Info { torrent_file } => {
            let torrent = Torrent::open(torrent_file);
            println!("Tracker URL: {}", torrent.announce);
            println!("Length: {}", torrent.info.length);
            println!("Info Hash: {}", torrent.info_hash());
            println!("Piece Length: {}", torrent.info.piece_length);
            println!("Piece Hashes:");
            for hash in torrent.info.pieces {
                println!("{}", hex::encode(hash));
            }
        }
        Commands::Peers { torrent_file } => {
            let torrent = Torrent::open(torrent_file);
            let peers = torrent.get_peers();
            for peer in peers {
                println!("{}", peer);
            }
        }
        Commands::Handshake { torrent_file, addr } => {
            let addr: SocketAddrV4 = addr.parse().expect("Failed to parse address");
            let mut socket = TcpStream::connect(addr).expect("Failed to connect to peer");
            let torrent = Torrent::open(torrent_file);

            let handshake = Handshake::new(
                "BitTorrent protocol".to_string(),
                torrent.info_hash(),
                [0; 20],
            );

            socket
                .write_all(&handshake.as_bytes())
                .expect("Failed to write handshake");

            let mut bytes = [0; 68];
            socket
                .read_exact(&mut bytes)
                .expect("Failed to read handshake");

            let peer_handshake = Handshake::from_bytes(bytes);
            println!("Peer ID: {}", hex::encode(peer_handshake.peer_id));
        }
    }
}

struct Handshake {
    pstr: String,
    reserved: [u8; 8],
    info_hash: [u8; 20],
    peer_id: [u8; 20],
}

impl Handshake {
    fn new(pstr: String, info_hash: String, peer_id: [u8; 20]) -> Self {
        let info_hash = hex::decode(info_hash).expect("Failed to decode info hash");

        Self {
            pstr,
            reserved: [0; 8],
            info_hash: info_hash.try_into().expect("Failed to convert info hash"),
            peer_id,
        }
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.pstr.len() as u8);
        bytes.extend(self.pstr.as_bytes());
        bytes.extend(&self.reserved);
        bytes.extend(&self.info_hash);
        bytes.extend(&self.peer_id);
        bytes
    }

    fn from_bytes(bytes: [u8; 68]) -> Self {
        let pstr_len = bytes[0] as usize;
        let pstr =
            String::from_utf8(bytes[1..pstr_len + 1].to_vec()).expect("Failed to parse pstr");
        let reserved = bytes[pstr_len + 1..pstr_len + 9]
            .try_into()
            .expect("Failed to parse reserved");
        let info_hash = bytes[pstr_len + 9..pstr_len + 29]
            .try_into()
            .expect("Failed to parse info hash");
        let peer_id = bytes[pstr_len + 29..pstr_len + 49]
            .try_into()
            .expect("Failed to parse peer id");

        Self {
            pstr,
            reserved,
            info_hash,
            peer_id,
        }
    }
}
