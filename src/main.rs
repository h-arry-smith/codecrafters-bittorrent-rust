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
        }
    }
}
