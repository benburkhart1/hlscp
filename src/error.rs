use thiserror::Error;

#[derive(Error, Debug)]
pub enum HlsError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    
    #[error("Playlist parsing error: {0}")]
    PlaylistParseError(String),
    
    #[error("Download error: {0}")]
    DownloadError(String),
}