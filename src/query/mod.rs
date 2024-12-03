use camino::{Utf8Ancestors, Utf8Component};
use id3::Tag;

// search in
// [ ] directory names
// [ ] file name
// [ ] metadata

// not ideal, means you can't eg filter by artist that way, TODO
enum TagGroup {
    DirComponents(Vec<String>),
    FileExploded(Vec<String>),
    MP3(Tag),
    Vorbis,
}
