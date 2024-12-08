use sea_orm::FromQueryResult;
use subsonic_types::{
    common::DateTime,
    response::{AlbumID3, ArtistID3},
};

use crate::entity::song;

#[derive(Debug)]
pub(crate) struct QueryResult {
    pub(crate) artists: Vec<Artist>,
    pub(crate) albums: Vec<Album>,
    pub(crate) songs: Vec<song::Model>,
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct Artist {
    name: String,
}

impl Artist {
    pub(crate) fn id(&self) -> String {
        self.name.clone()
    }

    pub(crate) fn album_count(&self) -> u32 {
        1
    }

    pub(crate) fn cover_art(&self) -> Option<String> {
        None
    }

    pub(crate) fn artist_image_url(&self) -> Option<String> {
        None
    }

    pub(crate) fn starred(&self) -> Option<DateTime> {
        None
    }

    pub(crate) fn name(&self) -> String {
        self.name.clone()
    }
}

impl From<Artist> for ArtistID3 {
    fn from(artist: Artist) -> Self {
        ArtistID3 {
            id: artist.id(),
            name: artist.name(),
            cover_art: artist.cover_art(),
            artist_image_url: artist.artist_image_url(),
            album_count: artist.album_count(),
            starred: artist.starred(),
        }
    }
}

impl From<song::Model> for Artist {
    fn from(song: song::Model) -> Self {
        Artist {
            // TODO h4x
            name: song.artist.unwrap_or_default(),
        }
    }
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct Album {
    title: String,
    artist: String,
}

// TODO h4x
impl From<Album> for AlbumID3 {
    fn from(album: Album) -> Self {
        // TODO h4x
        let artist = Artist { name: album.artist };
        AlbumID3 {
            id: album.title.clone(),
            name: album.title.clone(),
            // TODO h4x
            artist: Some(artist.name()),
            // TODO h4x
            artist_id: Some(artist.id()),
            cover_art: None,
            song_count: 1,
            duration: 1,
            play_count: None,
            created: None,
            starred: None,
            year: None,
            genre: None,
        }
    }
}
