use std::{sync::Arc, time::Duration};

use axum::{
    body::Body,
    extract::{Query, State},
    http::{header::CONTENT_TYPE, Method, Request, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
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
use tracing::{debug, error, info, trace, warn, Span};

use crate::{
    config::Config,
    entity::song,
    indexer::{db::DB, types::QueryResult},
    util::Pwn,
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
    base_url: String,
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

// enum Either {
//     First(Box<dyn IntoResponse>),
//     Second(Box<dyn IntoResponse>),
// }

// impl IntoResponse for Either {
//     fn into_response(self) -> Response {
//         match self {
//             Either::First(r) => r.into_response(),
//             Either::Second(r) => r.into_response(),
//         }
//     }
// }

// frontend is mostly a directory tree served 1:1, but for dioxus router we need
// to redirect to `{base_url}/` in a 404 scenario
async fn serve_frontend(state: State<AppState>, uri: Uri) -> Response {
    let base_url = &state.base_url;
    let path = uri.path();

    let components = match path
        .strip_prefix(base_url)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
    {
        Ok(path) => path.split("/"),
        Err(e) => {
            error!("request outside of base URL ({base_url}): {path}");
            return e.into_response();
        }
    };

    let path = components.fold(state.file_root.clone(), |acc, e| acc.join(e));
    let mime_type = MimeGuess::from_path(path.as_std_path())
        .first_or_octet_stream()
        .to_string();

    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(err) => {
            debug!("File not found: {}", err);

            // prevent redirect loop
            let mut index_path = state.file_root.clone();
            index_path.push("index.html");
            if path == index_path {
                error!("index.html not found at public root: {index_path}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            return Redirect::to((base_url.to_string() + "/").as_str()).into_response();
        }
    };

    let headers = [(CONTENT_TYPE, mime_type)];
    let body = AsyncReadBody::new(file);
    (headers, body).into_response()
}

pub async fn serve(db: Arc<DB>, config: &Config) {
    let base_url = config
        .system
        .base_url
        .as_deref()
        .to_pwned()
        .unwrap_or_default();
    let state = AppState {
        db,
        file_root: Utf8Path::new(&config.system.data_path).join("public"),
        base_url: base_url.clone(),
    };

    let api = Router::new()
        .route(
            "/scrobble.view",
            get(|query: Query<Scrobble>| async move {
                debug!("TODO scrobble {query:?}");
                SR(SubsonicResponse::ok(Version::LATEST, ResponseBody::Empty))
            }),
        )
        .route(
            "/getCoverArt.view",
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
            "/stream.view",
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
            "/getSong.view",
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
            "/search3.view",
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
            "/ping.view",
            get(|| async { SR(SubsonicResponse::ok(Version::V1_13_0, ResponseBody::Empty)) }),
        )
        .route(
            "/getPlaylists.view",
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
            "/getMusicFolders.view",
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
            "/getArtists.view",
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
            "/getAlbumList2.view",
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
        .with_state(state.clone());
    let index_url = base_url.clone() + "/index.html";
    let mut app = Router::new()
        .fallback(serve_frontend)
        .route(
            (base_url.clone() + "/").as_str(),
            get(|state| async move {
                serve_frontend(
                    state,
                    Uri::builder().path_and_query(index_url).build().unwrap(),
                )
                .await
            }),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn(uri_middleware))
        .layer(
            TraceLayer::new_for_http()
                .on_request(|req: &Request<Body>, _span: &Span| {
                    debug!("{} {}", req.method(), req.uri());
                })
                .on_response(|response: &Response, _latency: Duration, _span: &Span| {
                    trace!("{response:?}");
                    trace!(
                        "{:?}",
                        response.extensions().get::<RequestUri>().map(|r| &r.0)
                    )
                })
                .on_failure(
                    |error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        error!("{error:?}");
                    },
                ),
        )
        .nest(&(base_url.clone() + "/rest"), api);

    if config.system.dev {
        warn!("CORS: allowing any request");
        let cors = CorsLayer::new()
            // allow `GET` and `POST` when accessing the resource
            .allow_methods([Method::GET, Method::POST])
            // allow requests from any origin
            .allow_origin(cors::Any);
        app = app.layer(cors);
    }

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

// async fn shutdown_signal() {
//     info!("shutting down");
// }

#[derive(Clone)]
struct RequestUri(Uri);

async fn uri_middleware(request: Request<Body>, next: Next) -> Response {
    let uri = request.uri().clone();

    let mut response = next.run(request).await;

    response.extensions_mut().insert(RequestUri(uri));

    response
}
