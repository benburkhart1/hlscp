use anyhow::Result;
use clap::Parser;

use hlscp::{Args, HlsCopier};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    let copier = HlsCopier::new(&args.source, args.destination)?;
    copier.copy_hls().await?;
    
    println!("✓ HLS copy completed successfully!");
    Ok(())
}