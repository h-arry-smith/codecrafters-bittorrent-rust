use crate::bencode::Bencode;
use clap::{Parser, Subcommand};
use torrent::Torrent;
use tracker::Tracker;

mod bencode;
mod torrent;
mod tracker;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[clap(rename_all = "snake_case")]
enum Commands {
    Decode {
        encoded_value: String,
    },
    Info {
        torrent_file: String,
    },
    Peers {
        torrent_file: String,
    },
    Handshake {
        torrent_file: String,
        addr: String,
    },
    DownloadPiece {
        #[clap(short)]
        #[clap(short = 'o')]
        path: String,
        torrent_file: String,
        piece_index: usize,
    },
    Download {
        #[clap(short)]
        out: String,
        torrent_file: String,
    },
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
            let mut tracker = Tracker::new(Torrent::open(torrent_file), Some(addr));
            let handshake = tracker.handshake();
            println!("Peer ID: {}", hex::encode(handshake.peer_id));
        }
        Commands::DownloadPiece {
            path,
            torrent_file,
            piece_index,
        } => {
            let mut tracker = Tracker::new(Torrent::open(torrent_file), None);
            tracker.handshake();

            // create a file at the path
            let mut file = std::fs::File::create(path.clone()).expect("Failed to create file");
            tracker.download_piece(piece_index, &mut file);
            println!("Piece {} downloaded to {}.", piece_index, path);
        }
        Commands::Download { out, torrent_file } => {
            let mut tracker = Tracker::new(Torrent::open(torrent_file.clone()), None);
            tracker.handshake();

            // create a file at the path
            let mut file = std::fs::File::create(out.clone()).expect("Failed to create file");
            tracker.download_all_pieces(&mut file);
            println!("Downloaded {} to {}.", torrent_file, out);
        }
    }
}
