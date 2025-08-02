use anyhow::Result;
use regex::Regex;
use url::Url;

#[derive(Debug)]
pub struct Playlist {
    pub content: String,
    pub segments: Vec<String>,
    pub url: Url,
}

impl Playlist {
    pub fn parse(content: &str, base_url: &Url) -> Result<Self> {
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

    pub fn is_master_playlist(content: &str) -> bool {
        content.contains("#EXT-X-STREAM-INF") || 
        content.contains("#EXT-X-MEDIA") || 
        content.contains("#EXT-X-I-FRAME-STREAM-INF")
    }

    pub fn extract_all_playlists(content: &str) -> Vec<String> {
        let mut playlists = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let uri_regex = Regex::new(r#"URI="([^"]+)""#).unwrap();
        
        for (i, line) in lines.iter().enumerate() {
            let line = line.trim();
            
            if line.starts_with("#EXT-X-STREAM-INF") {
                if let Some(next_line) = lines.get(i + 1) {
                    let next_line = next_line.trim();
                    if !next_line.starts_with('#') && !next_line.is_empty() {
                        playlists.push(next_line.to_string());
                    }
                }
            }
            else if line.starts_with("#EXT-X-MEDIA") {
                if let Some(caps) = uri_regex.captures(line) {
                    if let Some(uri_match) = caps.get(1) {
                        playlists.push(uri_match.as_str().to_string());
                    }
                }
            }
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

    pub fn rewrite_content(&self) -> String {
        let mut content = self.content.clone();
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
}