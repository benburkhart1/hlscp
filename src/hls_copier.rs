use anyhow::{Context, Result};
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use reqwest::Client;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::playlist::Playlist;

pub struct HlsCopier {
    client: Client,
    base_url: Url,
    dest_dir: PathBuf,
    multi_progress: Arc<MultiProgress>,
}

impl HlsCopier {
    pub fn new(source_url: &str, dest_dir: PathBuf) -> Result<Self> {
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

    async fn process_playlist(&self, playlist_url: &Url, local_filename: &str) -> Result<()> {
        let content = self.fetch_playlist(playlist_url).await?;
        let playlist = Playlist::parse(&content, playlist_url)?;
        
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
        
        let rewritten_content = playlist.rewrite_content();
        let playlist_path = self.dest_dir.join(local_filename);
        
        fs::write(&playlist_path, rewritten_content)
            .with_context(|| format!("Failed to write playlist: {}", playlist_path.display()))?;
        
        Ok(())
    }

    pub async fn copy_hls(&self) -> Result<()> {
        fs::create_dir_all(&self.dest_dir).context("Failed to create destination directory")?;
        
        let master_filename = self.base_url.path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("playlist.m3u8");
        
        let master_content = self.fetch_playlist(&self.base_url).await?;
        let master_path = self.dest_dir.join(master_filename);
        fs::write(&master_path, &master_content).context("Failed to write master playlist")?;
        
        if Playlist::is_master_playlist(&master_content) {
            let stream_playlists = Playlist::extract_all_playlists(&master_content);
            
            for stream_playlist_url_str in stream_playlists {
                let stream_playlist_url = self.resolve_url(&stream_playlist_url_str, &self.base_url)?;
                let stream_filename = stream_playlist_url.path_segments()
                    .and_then(|segments| segments.last())
                    .unwrap_or(&stream_playlist_url_str);
                
                self.process_playlist(&stream_playlist_url, stream_filename).await?;
            }
        } else {
            self.process_playlist(&self.base_url, master_filename).await?;
        }
        
        Ok(())
    }
}