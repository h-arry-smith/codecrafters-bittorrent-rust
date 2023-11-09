use crate::bencode::Bencode;
use clap::{Parser, Subcommand};

mod bencode;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Decode { encoded_value: String },
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Decode { encoded_value } => {
            let decoded_value = Bencode::new(&encoded_value).decode();
            println!("{}", serde_json::to_string_pretty(&decoded_value).unwrap());
        }
    }
}
