use id3::Tag;

// TODO below are just some random ideas

// search in
// [ ] directory names
// [ ] file name
// [ ] metadata

// not ideal, means you can't eg filter by artist that way, TODO
#[allow(unused)]
enum QueryGroup {
    DirComponents(Vec<String>),
    FileExploded(Vec<String>),
    MP3(Tag),
    Vorbis,
}
