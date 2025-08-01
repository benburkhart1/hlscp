use anyhow::{Context, Result};
use clap::Parser;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use regex::Regex;
use reqwest::Client;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use url::Url;

#[derive(Parser)]
#[command(name = "hlscp")]
#[command(about = "Copy HLS rendition from remote source to local directory")]
struct Args {
    #[arg(help = "Source HLS playlist URL")]
    source: String,
    #[arg(help = "Destination directory")]
    destination: PathBuf,
}

#[derive(Debug)]
struct Playlist {
    content: String,
    segments: Vec<String>,
    url: Url,
}

struct HlsCopier {
    client: Client,
    base_url: Url,
    dest_dir: PathBuf,
    multi_progress: Arc<MultiProgress>,
}

impl HlsCopier {
    fn new(source_url: &str, dest_dir: PathBuf) -> Result<Self> {
        let base_url = Url::parse(source_url).context("Invalid source URL")?;
        let client = Client::new();
        let multi_progress = Arc::new(MultiProgress::new());
        
        Ok(HlsCopier {
            client,
            base_url,
            dest_dir,
            multi_progress,
        })
    }

    async fn fetch_playlist(&self, url: &Url) -> Result<String> {
        let pb = self.multi_progress.add(ProgressBar::new_spinner());
        pb.set_style(ProgressStyle::default_spinner()
            .template("{spinner:.green} Fetching playlist: {msg}")
            .unwrap());
        pb.set_message(url.as_str().to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        
        let response = self.client
            .get(url.as_str())
            .send()
            .await
            .context("Failed to fetch playlist")?;
        
        let content = response
            .text()
            .await
            .context("Failed to read playlist content")?;
        
        pb.finish_with_message(format!("✓ Fetched playlist: {}", url.as_str()));
        Ok(content)
    }

    fn parse_playlist(&self, content: &str, base_url: &Url) -> Result<Playlist> {
        let mut segments = Vec::new();
        let uri_regex = Regex::new(r#"URI="([^"]+)""#)?;
        
        for line in content.lines() {
            let line = line.trim();
            
            if line.starts_with("#EXT-X-MAP:") {
                if let Some(caps) = uri_regex.captures(line) {
                    if let Some(uri_match) = caps.get(1) {
                        segments.push(uri_match.as_str().to_string());
                    }
                }
            } else if !line.starts_with('#') && !line.is_empty() {
                segments.push(line.to_string());
            }
        }

        Ok(Playlist {
            content: content.to_string(),
            segments,
            url: base_url.clone(),
        })
    }

    fn is_master_playlist(&self, content: &str) -> bool {
        content.contains("#EXT-X-STREAM-INF") || 
        content.contains("#EXT-X-MEDIA") || 
        content.contains("#EXT-X-I-FRAME-STREAM-INF")
    }

    fn extract_all_playlists(&self, content: &str) -> Vec<String> {
        let mut playlists = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let uri_regex = Regex::new(r#"URI="([^"]+)""#).unwrap();
        
        for (i, line) in lines.iter().enumerate() {
            let line = line.trim();
            
            // Handle #EXT-X-STREAM-INF (main stream playlists)
            if line.starts_with("#EXT-X-STREAM-INF") {
                if let Some(next_line) = lines.get(i + 1) {
                    let next_line = next_line.trim();
                    if !next_line.starts_with('#') && !next_line.is_empty() {
                        playlists.push(next_line.to_string());
                    }
                }
            }
            // Handle #EXT-X-MEDIA (audio, video, subtitles, closed-captions)
            else if line.starts_with("#EXT-X-MEDIA") {
                if let Some(caps) = uri_regex.captures(line) {
                    if let Some(uri_match) = caps.get(1) {
                        playlists.push(uri_match.as_str().to_string());
                    }
                }
            }
            // Handle #EXT-X-I-FRAME-STREAM-INF (I-frame only playlists)
            else if line.starts_with("#EXT-X-I-FRAME-STREAM-INF") {
                if let Some(caps) = uri_regex.captures(line) {
                    if let Some(uri_match) = caps.get(1) {
                        playlists.push(uri_match.as_str().to_string());
                    }
                }
            }
        }
        
        playlists
    }

    fn resolve_url(&self, url_str: &str, base_url: &Url) -> Result<Url> {
        if let Ok(absolute_url) = Url::parse(url_str) {
            Ok(absolute_url)
        } else {
            base_url.join(url_str).context("Failed to resolve relative URL")
        }
    }

    async fn download_segment(&self, url: &Url, filename: &str, pb: &ProgressBar) -> Result<()> {
        pb.set_message(format!("Downloading: {}", filename));
        
        let response = self.client
            .get(url.as_str())
            .send()
            .await
            .context("Failed to fetch segment")?;
        
        let bytes = response
            .bytes()
            .await
            .context("Failed to read segment bytes")?;
        
        let file_path = self.dest_dir.join(filename);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).context("Failed to create directory")?;
        }
        
        let mut file = File::create(&file_path)
            .await
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;
        
        file.write_all(&bytes)
            .await
            .context("Failed to write segment to file")?;
        
        pb.inc(1);
        Ok(())
    }

    fn rewrite_playlist(&self, playlist: &Playlist) -> String {
        let mut content = playlist.content.clone();
        let uri_regex = Regex::new(r#"URI="([^"]+)""#).unwrap();
        
        content = uri_regex.replace_all(&content, |caps: &regex::Captures| {
            let original_uri = &caps[1];
            if let Ok(url) = Url::parse(original_uri) {
                let filename = url.path_segments()
                    .and_then(|segments| segments.last())
                    .unwrap_or(original_uri);
                format!(r#"URI="{}""#, filename)
            } else {
                caps[0].to_string()
            }
        }).to_string();
        
        let mut lines: Vec<String> = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if !line.starts_with('#') && !line.is_empty() {
                if let Ok(_) = Url::parse(line) {
                    if let Ok(url) = Url::parse(line) {
                        let filename = url.path_segments()
                            .and_then(|segments| segments.last())
                            .unwrap_or(line);
                        lines.push(filename.to_string());
                    } else {
                        lines.push(line.to_string());
                    }
                } else {
                    lines.push(line.to_string());
                }
            } else {
                lines.push(line.to_string());
            }
        }
        
        lines.join("\n")
    }

    async fn process_playlist(&self, playlist_url: &Url, local_filename: &str) -> Result<()> {
        let content = self.fetch_playlist(playlist_url).await?;
        let playlist = self.parse_playlist(&content, playlist_url)?;
        
        // Only download segments if this playlist has any (media playlists have segments, master playlists don't)
        if !playlist.segments.is_empty() {
            let mut segment_data = Vec::new();
            
            for segment_url_str in &playlist.segments {
                let segment_url = self.resolve_url(segment_url_str, &playlist.url)?;
                let filename = segment_url.path_segments()
                    .and_then(|segments| segments.last())
                    .unwrap_or(segment_url_str)
                    .to_string();
                
                segment_data.push((segment_url, filename));
            }
            
            let pb = self.multi_progress.add(ProgressBar::new(segment_data.len() as u64));
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} segments ({msg})")
                .unwrap()
                .progress_chars("#>-"));
            pb.set_message(format!("Downloading segments for {}", local_filename));
            
            let download_tasks = segment_data.iter()
                .map(|(url, filename)| self.download_segment(url, filename, &pb));
            
            join_all(download_tasks).await.into_iter().collect::<Result<Vec<_>>>()?;
            pb.finish_with_message(format!("✓ Downloaded {} segments for {}", segment_data.len(), local_filename));
        }
        
        let rewritten_content = self.rewrite_playlist(&playlist);
        let playlist_path = self.dest_dir.join(local_filename);
        
        fs::write(&playlist_path, rewritten_content)
            .with_context(|| format!("Failed to write playlist: {}", playlist_path.display()))?;
        
        Ok(())
    }

    async fn copy_hls(&self) -> Result<()> {
        fs::create_dir_all(&self.dest_dir).context("Failed to create destination directory")?;
        
        let master_filename = self.base_url.path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("playlist.m3u8");
        
        // First, fetch and save the master playlist
        let master_content = self.fetch_playlist(&self.base_url).await?;
        let master_path = self.dest_dir.join(master_filename);
        fs::write(&master_path, &master_content).context("Failed to write master playlist")?;
        
        // Check if this is a master playlist or a media playlist
        if self.is_master_playlist(&master_content) {
            // Extract all referenced playlists from the master playlist
            let stream_playlists = self.extract_all_playlists(&master_content);
            
            // Process each stream playlist
            for stream_playlist_url_str in stream_playlists {
                let stream_playlist_url = self.resolve_url(&stream_playlist_url_str, &self.base_url)?;
                let stream_filename = stream_playlist_url.path_segments()
                    .and_then(|segments| segments.last())
                    .unwrap_or(&stream_playlist_url_str);
                
                self.process_playlist(&stream_playlist_url, stream_filename).await?;
            }
        } else {
            // This is already a media playlist, process it directly
            self.process_playlist(&self.base_url, master_filename).await?;
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    let copier = HlsCopier::new(&args.source, args.destination)?;
    copier.copy_hls().await?;
    
    println!("✓ HLS copy completed successfully!");
    Ok(())
}