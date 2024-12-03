use std::{
    sync::{atomic::AtomicU32, Arc},
    time::Duration,
};

use axum::{
    body::Body,
    extract::{Query, State},
    http::{header::CONTENT_TYPE, Request, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use axum_extra::body::AsyncReadBody;
use id3::TagLike;
use subsonic_types::{
    common::Version,
    request::{browsing::GetSong, retrieval::Stream, search::Search3},
    response::{
        AlbumID3, AlbumList2, ArtistID3, ArtistsID3, Child, IndexID3, MusicFolder, MusicFolders,
        Playlist, Playlists, Response as SubsonicResponse, ResponseBody, SearchResult3,
    },
};
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::{debug, error, info, Span};

use crate::indexer::DB;

// wrapper to get around orphan rule, so we can impl IntoResponse
struct SR(SubsonicResponse);

impl IntoResponse for SR {
    fn into_response(self) -> Response {
        self.0.to_json().expect("bug").into_response()
    }
}

/*


let Some(song_path) = state_db.song(&query.id) else {
                        error!("cannot find {}", query.id);
                        return Err((StatusCode::NOT_FOUND, "404".to_string()));
                    };

                    debug!("try {song_path}");
                    let file = match tokio::fs::File::open(song_path).await {
                        Ok(file) => file,
                        Err(err) => {
                            return Err((StatusCode::NOT_FOUND, format!("File not found: {}", err)))
                        }
                    };
                    let headers = [(CONTENT_TYPE, "audio/mpeg")];
                    let body = AsyncReadBody::new(file);
                    Ok((headers, body))
*/
pub async fn serve(db: Arc<DB>) {
    // build our application with a route
    let app = Router::new()
        .route(
            "/",
            get(|| async { "when I grow up I'll be a landing page" }),
        )
        .route(
            "/rest/stream.view",
            get(
                |State(state_db): State<Arc<DB>>, query: Query<Stream>| async move {
                    let Some(song_path) = state_db.song(&query.id) else {
                        error!("cannot find {}", query.id);
                        return Err((StatusCode::NOT_FOUND, "404".to_string()));
                    };

                    debug!("try {song_path}");
                    let file = match tokio::fs::File::open(song_path).await {
                        Ok(file) => file,
                        Err(err) => {
                            return Err((StatusCode::NOT_FOUND, format!("File not found: {}", err)))
                        }
                    };
                    let headers = [(CONTENT_TYPE, "audio/mpeg")];
                    let body = AsyncReadBody::new(file);
                    Ok((headers, body))
                },
            ),
        )
        .route(
            "/rest/getSong.view",
            get(
                |State(state_db): State<Arc<DB>>, query: Query<GetSong>| async move {
                    let Some(path) = state_db.song(&query.id) else {
                        error!("cannot find {}", query.id);
                        return Err((StatusCode::NOT_FOUND, "404".to_string()));
                    };

                    debug!("try {path}");

                    let mut song = Child::default();

                    song.id = query.id.clone();

                    // TODO
                    song.suffix = Some("mp3".to_string());
                    song.path = Some(path.to_string());
                    let fallback_title = "FIXME".to_string();
                    if let Some(meta) = state_db.meta(&path) {
                        match &meta.tag {
                            Some(tag) => {
                                song.artist = tag.artist().map(str::to_string);
                                song.title =
                                    tag.title().map(str::to_string).unwrap_or(fallback_title);
                            }

                            None => {
                                song.title = fallback_title;
                            }
                        }
                    }

                    Ok(SR(SubsonicResponse::ok(
                        Version::V1_13_0,
                        ResponseBody::Song(song),
                    )))
                },
            ),
        )
        .route(
            "/rest/search3.view",
            get(
                |State(state_db): State<Arc<DB>>, query: Query<Search3>| async move {
                    // debug!("{query:?}");
                    let mut artists = vec![];
                    let mut albums = vec![];
                    let mut songs = vec![];
                    let mut album_id = 1;
                    let mut artist_id = 1;
                    let mut song_id = 1;
                    state_db.for_each(|path, meta| {
                        let mut artist = ArtistID3::default();
                        artist.id = format!("ar-{artist_id}");
                        let mut album = AlbumID3::default();
                        album.id = format!("al-{album_id}");
                        let mut song = Child::default();

                        song.id = format!("s-{song_id}");

                        // TODO
                        song.suffix = Some("mp3".to_string());
                        song.path = Some(path.to_string());
                        let fallback_title = path
                            .components()
                            .last()
                            .map(|c| c.to_string())
                            .unwrap_or("BROKEN".to_string());
                        match &meta.tag {
                            Some(tag) => {
                                song.artist = tag.artist().map(str::to_string);
                                song.title =
                                    tag.title().map(str::to_string).unwrap_or(fallback_title);
                            }

                            None => {
                                song.title = fallback_title;
                            }
                        }

                        songs.push(song);
                        debug!("songmax {:?}", query.song_count);

                        album_id += 1;
                        artist_id += 1;
                        song_id += 1;
                    });

                    SR(SubsonicResponse::ok(
                        Version::V1_13_0,
                        ResponseBody::SearchResult3(SearchResult3 {
                            artist: artists,
                            album: albums,
                            song: songs,
                        }),
                    ))
                },
            ),
        )
        .route(
            "/rest/ping.view",
            get(|| async { SR(SubsonicResponse::ok(Version::V1_13_0, ResponseBody::Empty)) }),
        )
        .route(
            "/rest/getPlaylists.view",
            get(|| async {
                let mut pl = Playlist::default();
                pl.name = "EGG!!".into();
                pl.id = "1".into();
                pl.song_count = 1;
                let pls = Playlists { playlist: vec![pl] };
                SR(SubsonicResponse::ok(
                    Version::V1_13_0,
                    ResponseBody::Playlists(pls),
                ))
            }),
        )
        .route(
            "/rest/getMusicFolders.view",
            get(|| async {
                let folders = MusicFolders {
                    music_folder: vec![MusicFolder {
                        id: 1,
                        name: Some("music".into()),
                    }],
                };
                SR(SubsonicResponse::ok(
                    Version::V1_13_0,
                    ResponseBody::MusicFolders(folders),
                ))
            }),
        )
        .route(
            "/rest/getArtists.view",
            get(|| async {
                let mut artist = ArtistID3::default();
                artist.name = "ART!!!".into();
                artist.id = "1".into();
                let artists = ArtistsID3 {
                    index: vec![IndexID3 {
                        name: "idx".into(),
                        artist: vec![artist],
                    }],
                    ignored_articles: "".into(),
                };
                SR(SubsonicResponse::ok(
                    Version::V1_13_0,
                    ResponseBody::Artists(artists),
                ))
            }),
        )
        .route(
            "/rest/getAlbumList2.view",
            get(|| async {
                let mut album = AlbumID3::default();
                album.id = "1".into();
                album.name = "MY FIRST ALBUM".into();
                let albums = AlbumList2 { album: vec![album] };
                SR(SubsonicResponse::ok(
                    Version::V1_13_0,
                    ResponseBody::AlbumList2(albums),
                ))
            }),
        )
        .with_state(db)
        .layer(
            TraceLayer::new_for_http()
                .on_request(|req: &Request<Body>, _span: &Span| {
                    debug!("{} {}", req.method(), req.uri());
                })
                .on_response(|response: &Response, _latency: Duration, _span: &Span| {
                    debug!("{response:?}");
                    debug!(
                        "{:?}",
                        response.extensions().get::<RequestUri>().map(|r| &r.0)
                    )
                })
                .on_failure(
                    |error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        error!("{error:?}");
                    },
                ),
        );

    let addr = "localhost:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("Running on {}", listener.local_addr().unwrap());

    // TODO figure out how exactly the shutdown handling is supposed to work,
    // as it stands it leads to immediate shutdown
    axum::serve(listener, app)
        // .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("could not axum::serve()");
}

async fn shutdown_signal() {
    info!("shutting down");
}

#[derive(Clone)]
struct RequestUri(Uri);

async fn uri_middleware(request: Request<Body>, next: Next) -> Response {
    let uri = request.uri().clone();

    let mut response = next.run(request).await;

    response.extensions_mut().insert(RequestUri(uri));

    response
}
