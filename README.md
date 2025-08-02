# hlscp

A fast, concurrent Rust application for downloading HLS (HTTP Live Streaming) content from remote sources to local directories.

## Features

- **Complete HLS Support**: Downloads master playlists, media playlists, and all associated segments
- **Multi-format Compatibility**: Works with WebVTT playlists, I-frame only manifests, and other HLS playlist types
- **Concurrent Downloads**: Parallel segment downloading for improved performance
- **Progress Tracking**: Real-time progress bars showing download status
- **URL Rewriting**: Automatically converts absolute URLs in playlists to relative local paths
- **Directory Structure**: Maintains proper file organization in the destination directory

## Installation

### Prerequisites

- Rust 1.70 or later

### From Source

```bash
git clone https://github.com/benburkhart1/hlscp.git
cd hlscp
cargo build --release
```

The binary will be available at `target/release/hlscp`.

## Usage

```bash
hlscp <source_url> <destination_directory>
```

### Arguments

- `source_url`: The URL of the HLS playlist (master or media playlist)
- `destination_directory`: Local directory where files will be saved

### Examples

Download a complete HLS stream:
```bash
hlscp https://example.com/playlist.m3u8 ./downloads/stream1
```

Download to current directory:
```bash
hlscp https://example.com/master.m3u8 .
```

## How It Works

1. **Playlist Analysis**: Determines if the source is a master playlist or media playlist
2. **Master Playlist Processing**: If master playlist, extracts all referenced media playlists
3. **Media Playlist Processing**: Downloads all segments referenced in each media playlist
4. **URL Rewriting**: Converts absolute URLs to relative paths in downloaded playlists
5. **File Organization**: Maintains directory structure and saves all files locally

## Supported HLS Features

- Master playlists (`#EXT-X-STREAM-INF`)
- Media playlists with segments
- Alternative audio/video tracks (`#EXT-X-MEDIA`)
- I-frame only streams (`#EXT-X-I-FRAME-STREAM-INF`)
- WebVTT subtitle tracks
- Segment maps (`#EXT-X-MAP`)

## Dependencies

- **tokio**: Async runtime for concurrent operations
- **reqwest**: HTTP client for downloading content
- **clap**: Command-line argument parsing
- **indicatif**: Progress bar implementation
- **url**: URL parsing and manipulation
- **regex**: Pattern matching for playlist parsing
- **anyhow**: Error handling
- **futures**: Async utilities

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.