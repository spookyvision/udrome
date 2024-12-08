use std::{sync::Arc, time::Duration};

use axum::{
    body::Body,
    extract::{Query, State},
    http::{header::CONTENT_TYPE, Method, Request, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_extra::{body::AsyncReadBody, headers::Range, TypedHeader};
use axum_range::{KnownSize, Ranged};
use camino::{Utf8Path, Utf8PathBuf};
use mime_guess::MimeGuess;
use serde::{Deserialize, Serialize};
use subsonic_types::{
    common::{Milliseconds, Seconds, Version},
    request::{
        browsing::GetSong,
        retrieval::{GetCoverArt, Stream},
        search::Search3,
    },
    response::{
        AlbumID3, AlbumList2, ArtistID3, ArtistsID3, Child, IndexID3, MusicFolder, MusicFolders,
        Playlist, Playlists, Response as SubsonicResponse, ResponseBody, SearchResult3,
    },
};
use tower_http::{
    classify::ServerErrorsFailureClass,
    cors::{self, CorsLayer},
    trace::TraceLayer,
};
use tracing::{debug, error, info, warn, Span};

use crate::{
    config::Config,
    entity::song,
    indexer::{
        db::DB,
        types::{Artist, QueryResult},
    },
};

// wrapper to get around orphan rule, so we can impl IntoResponse
struct SR(SubsonicResponse);

impl IntoResponse for SR {
    fn into_response(self) -> Response {
        self.0.to_json().expect("bug").into_response()
    }
}

#[derive(Debug, Clone)]

struct AppState {
    db: Arc<DB>,
    file_root: Utf8PathBuf,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Scrobble {
    /// A string which uniquely identifies the file to scrobble.
    #[serde(default)]
    pub id: String, // fixed: this was Vec<String>
    /// The time at which the song was listened to.
    #[serde(default)]
    pub time: Milliseconds, // fixed: this was Vec<Milliseconds>
    /// Whether this is a "submission" or a "now playing" notification.
    pub submission: Option<bool>,
}

impl From<song::Model> for Child {
    fn from(song: song::Model) -> Self {
        let mut child = Child::default();
        child.id = format!("{}", song.id);
        child.path = Some(song.path);
        child.parent = song.parent;
        child.title = song.title;
        child.album_id = song.album.clone();
        child.album = song.album;
        child.artist = song.artist.clone();
        child.artist_id = song.artist;
        child.track = song.track;
        child.duration = song.duration.map(|d| Seconds::new(d as _));
        child.year = song.year;
        child.genre = song.genre;
        child.cover_art = song.cover_art;
        child.size = song.size.map(|sz| sz.into());
        child.content_type = song.content_type;
        child
    }
}

async fn serve_frontend(state: State<AppState>, uri: Uri) -> impl IntoResponse {
    let components = uri.path().split("/");
    let path = components.fold(state.file_root.clone(), |acc, e| acc.join(e));
    let mime_type = MimeGuess::from_path(path.as_std_path())
        .first_or_octet_stream()
        .to_string();

    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(err) => {
            return Err((StatusCode::NOT_FOUND, format!("File not found: {}", err)));
        }
    };

    let headers = [(CONTENT_TYPE, mime_type)];
    let body = AsyncReadBody::new(file);
    Ok((headers, body))
}

pub async fn serve(db: Arc<DB>, config: &Config) {
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(cors::Any);

    let app = Router::new()
        .fallback(serve_frontend)
        .route(
            "/",
            get(|state| async {
                serve_frontend(
                    state,
                    Uri::builder()
                        .path_and_query("/index.html")
                        .build()
                        .unwrap(),
                )
                .await
            }),
        )
        .route(
            "/rest/scrobble.view",
            get(|query: Query<Scrobble>| async move {
                debug!("TODO scrobble {query:?}");
                SR(SubsonicResponse::ok(Version::LATEST, ResponseBody::Empty))
            }),
        )
        .route(
            "/rest/getCoverArt.view",
            get(
                |State(state): State<AppState>, query: Query<GetCoverArt>| async move {
                    let Some(cover_art) = state.db.get_cover_art(&query.id).await else {
                        error!("cannot find {}", query.id);
                        return Err((StatusCode::NOT_FOUND, "404".to_string()));
                    };

                    let file = match tokio::fs::File::open(cover_art.path(state.db.data_path()))
                        .await
                    {
                        Ok(file) => file,
                        Err(err) => {
                            return Err((StatusCode::NOT_FOUND, format!("File not found: {}", err)))
                        }
                    };
                    let headers = [(CONTENT_TYPE, cover_art.mime_type)];
                    let body = AsyncReadBody::new(file);
                    Ok((headers, body))
                },
            ),
        )
        .route(
            "/rest/stream.view",
            get(
                |State(state): State<AppState>,
                 range: Option<TypedHeader<Range>>,
                 query: Query<Stream>| async move {
                    let Some(song) = state.db.get_song(&query.id).await else {
                        error!("cannot find {}", query.id);
                        return Err((StatusCode::NOT_FOUND, "404".to_string()));
                    };
                    debug!("streaming {song:?}");
                    let file = match tokio::fs::File::open(song.path).await {
                        Ok(file) => file,
                        Err(err) => {
                            return Err((StatusCode::NOT_FOUND, format!("File not found: {}", err)))
                        }
                    };
                    let body = KnownSize::file(file).await.unwrap();
                    let range = range.map(|TypedHeader(range)| range);
                    let ranged = Ranged::new(range, body);
                    let headers = [(CONTENT_TYPE, "audio/mpeg")];
                    Ok((headers, ranged))
                },
            ),
        )
        .route(
            "/rest/getSong.view",
            get(
                |State(state): State<AppState>, query: Query<GetSong>| async move {
                    let Some(song) = state.db.get_song(&query.id).await else {
                        error!("cannot find {}", query.id);
                        return Err((StatusCode::NOT_FOUND, "404".to_string()));
                    };

                    Ok(SR(SubsonicResponse::ok(
                        Version::V1_13_0,
                        ResponseBody::Song(song.into()),
                    )))
                },
            ),
        )
        .route(
            "/rest/search3.view",
            get(
                |State(state): State<AppState>, query: Query<Search3>| async move {
                    let QueryResult {
                        albums,
                        artists,
                        songs,
                    } = state.db.query(&query).await;

                    let albums = albums.into_iter().map(|m| m.into()).collect();
                    let artists = artists.into_iter().map(|m| m.into()).collect();
                    let songs = songs.into_iter().map(|m| m.into()).collect();
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
            get(|State(state): State<AppState>| async move {
                // TODO (everywhere): do we gain anything from using Option<String> for user_query instead?
                let ars = state
                    .db
                    .get_artists("", None, None)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(ArtistID3::from)
                    .inspect(|a| {
                        debug!("{}", a.name);
                    })
                    .collect();
                let artists = ArtistsID3 {
                    index: vec![IndexID3 {
                        name: "idx".into(),
                        artist: ars,
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
            get(|State(state): State<AppState>| async move {
                let albums = state
                    .db
                    .get_albums("", None, None)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(AlbumID3::from)
                    .inspect(|a| {
                        debug!("{}", a.name);
                    })
                    .collect();

                let albums = AlbumList2 { album: albums };
                SR(SubsonicResponse::ok(
                    Version::V1_13_0,
                    ResponseBody::AlbumList2(albums),
                ))
            }),
        )
        .with_state(AppState {
            db,
            file_root: Utf8Path::new(&config.system.data_path).join("public"),
        })
        .layer(
            TraceLayer::new_for_http()
                .on_request(|req: &Request<Body>, _span: &Span| {
                    debug!("{} {}", req.method(), req.uri());
                })
                .on_response(|response: &Response, _latency: Duration, _span: &Span| {
                    // debug!("{response:?}");
                    // debug!(
                    //     "{:?}",
                    //     response.extensions().get::<RequestUri>().map(|r| &r.0)
                    // )
                })
                .on_failure(
                    |error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        error!("{error:?}");
                    },
                ),
        )
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(&config.system.bind_addr)
        .await
        .unwrap();
    info!("Running on {}", listener.local_addr().unwrap());

    // TODO figure out how exactly the shutdown handling is supposed to work,
    // as it stands adding it leads to immediate shutdown
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
