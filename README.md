hlscp
=====

hlscp is a Rust application which copies a HLS rendition from a remote source to a local directory.

hlscp iterates through all the playlists, and downloads all the playlists and segments and puts 
it into a local directory. If the segment names in playlists are absolute URLs, the playlist segment
filenames are changed to relative local paths. 

hlcp should work on WebVTT playlists with VTT segments, i frame only manifests, and other playlist
types that are not as common as the main media playlists.
