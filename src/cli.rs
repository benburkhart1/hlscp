use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hlscp")]
#[command(about = "Copy HLS rendition from remote source to local directory")]
pub struct Args {
    #[arg(help = "Source HLS playlist URL")]
    pub source: String,
    #[arg(help = "Destination directory")]
    pub destination: PathBuf,
}