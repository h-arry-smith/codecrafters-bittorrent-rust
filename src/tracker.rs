use std::{
    fs::File,
    io::{Read, Write},
    net::{SocketAddrV4, TcpStream},
};

use crate::torrent::Torrent;

pub struct Tracker {
    torrent: Torrent,
    socket: TcpStream,
    // TODO: Could use struct states for this
    state: State,
}

impl Tracker {
    pub fn new(torrent: Torrent, addr: Option<String>) -> Self {
        let addr: SocketAddrV4 = match addr {
            Some(addr) => (*addr).parse::<SocketAddrV4>().unwrap(),
            None => *torrent.get_peers().first().unwrap(),
        };

        let socket = TcpStream::connect(addr).expect("Failed to connect to peer");

        Self {
            torrent,
            socket,
            state: State::Connected,
        }
    }

    pub fn handshake(&mut self) -> Handshake {
        if self.state != State::Connected {
            panic!("Cannot handshake in state {:?}", self.state);
        }

        let handshake = Handshake::new(
            "BitTorrent protocol".to_string(),
            self.torrent.info_hash(),
            [0; 20],
        );

        self.socket
            .write_all(&handshake.as_bytes())
            .expect("Failed to write handshake");

        let mut bytes = [0; 68];
        self.socket
            .read_exact(&mut bytes)
            .expect("Failed to read handshake");

        self.state = State::Handshake;
        Handshake::from_bytes(bytes)
    }

    pub fn download_all_pieces(&mut self, file: &mut File) {
        if self.state != State::Handshake {
            panic!("Cannot download pieces in state {:?}", self.state);
        }

        for piece_index in 0..self.torrent.info.pieces.len() {
            eprintln!("starting {}", piece_index);
            self.download_piece(piece_index, file);
        }
    }

    pub fn download_piece(&mut self, piece_index: usize, file: &mut File) {
        let _piece_hash = self.torrent.info.pieces[piece_index];
        if self.state == State::Handshake {
            self.state = State::WaitingForBitField;
        }

        eprintln!("Downloading piece {}", piece_index);

        loop {
            #[allow(clippy::single_match)]
            match self.state {
                State::WaitingForBitField => {
                    let message = Message::read_from_socket(&mut self.socket);
                    if message.id == MessageId::Bitfield {
                        self.state = State::SendInterested;
                    }
                }
                State::SendInterested => {
                    let message = Message::interested();
                    self.socket
                        .write_all(&message.as_bytes())
                        .expect("Failed to write interested");
                    self.state = State::WaitingForUnchoke;
                }
                State::WaitingForUnchoke => {
                    let message = Message::read_from_socket(&mut self.socket);
                    if message.id == MessageId::Unchoke {
                        self.state = State::Download;
                    }
                }
                State::Download => {
                    let piece_length = usize::min(
                        self.torrent.info.length - (piece_index * self.torrent.info.piece_length),
                        self.torrent.info.piece_length,
                    );
                    let blocks_to_download = (piece_length as f64 / 16384.0).ceil() as usize;
                    let mut block_index = 0;

                    while block_index < blocks_to_download {
                        eprintln!("downloading block {}", block_index);
                        let payload: Vec<u8> = {
                            let mut payload: Vec<u8> = Vec::new();
                            let piece_index = piece_index as u32;
                            let block_index_start = block_index as u32 * 16384;
                            let block_length =
                                u32::min(piece_length as u32 - (block_index * 16384) as u32, 16384);
                            payload.extend(&piece_index.to_be_bytes());
                            payload.extend(&block_index_start.to_be_bytes());
                            payload.extend(&block_length.to_be_bytes());
                            payload
                        };

                        let request_message = Message::new(MessageId::Request, payload);
                        self.socket
                            .write_all(&request_message.as_bytes())
                            .expect("Failed to write request");

                        let response_message = Message::read_from_socket(&mut self.socket);
                        assert!(response_message.id == MessageId::Piece);
                        let piece = response_message.payload[8..].to_vec();
                        file.write_all(&piece).expect("Failed to write piece");
                        block_index += 1
                    }

                    // TODO: Verify piece hash
                    self.state = State::Finish
                }
                State::Finish => {
                    eprintln!("finish {}", piece_index);
                    self.state = State::Download;
                    break;
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Connected,
    Handshake,
    WaitingForBitField,
    SendInterested,
    WaitingForUnchoke,
    Download,
    Finish,
}

#[derive(Debug)]
struct Message {
    length: u32,
    id: MessageId,
    payload: Vec<u8>,
}

impl Message {
    fn new(id: MessageId, payload: Vec<u8>) -> Self {
        let length = payload.len() as u32 + 1;
        Self {
            length,
            id,
            payload,
        }
    }

    fn interested() -> Self {
        Self::new(MessageId::Interested, vec![])
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&self.length.to_be_bytes());
        bytes.push(self.id.clone().into());
        bytes.extend(&self.payload);
        bytes
    }

    fn read_from_socket(socket: &mut TcpStream) -> Self {
        let mut buf = [0; 4];
        socket.read_exact(&mut buf).unwrap();
        let length = u32::from_be_bytes(buf);

        let mut buf = vec![0; length as usize];
        socket.read_exact(&mut buf).unwrap();
        let (tag, payload) = buf.split_first().unwrap();

        Self {
            length,
            id: (*tag).into(),
            payload: payload.to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MessageId {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have,
    Bitfield,
    Request,
    Piece,
    Cancel,
}

impl From<u8> for MessageId {
    fn from(id: u8) -> Self {
        match id {
            0 => Self::Choke,
            1 => Self::Unchoke,
            2 => Self::Interested,
            3 => Self::NotInterested,
            4 => Self::Have,
            5 => Self::Bitfield,
            6 => Self::Request,
            7 => Self::Piece,
            8 => Self::Cancel,
            _ => panic!("Invalid message id"),
        }
    }
}

impl From<MessageId> for u8 {
    fn from(id: MessageId) -> Self {
        match id {
            MessageId::Choke => 0,
            MessageId::Unchoke => 1,
            MessageId::Interested => 2,
            MessageId::NotInterested => 3,
            MessageId::Have => 4,
            MessageId::Bitfield => 5,
            MessageId::Request => 6,
            MessageId::Piece => 7,
            MessageId::Cancel => 8,
        }
    }
}

pub struct Handshake {
    pub pstr: String,
    pub reserved: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
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
