use std::{
    env,
    fs::File,
    io::BufReader,
    ops::Add,
    sync::{
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    thread::{sleep, spawn},
    time::Duration,
};

use app::{App, Command, State};
use playlist::Player;
use rodio::{source::EmptyCallback, Decoder, OutputStream, Sink};
use tracing::{debug, error, info};
use tracing_subscriber::{prelude::*, EnvFilter};
use tui_logger::init_logger;

use crate::http::SeekRequest;

mod app;
mod http;
pub mod playlist;
fn main() -> anyhow::Result<()> {
    // tracing_subscriber::fmt::init();
    init_logger(tui_logger::LevelFilter::Debug)?;
    tracing_subscriber::registry()
        .with(tui_logger::tracing_subscriber_layer())
        .with(EnvFilter::from_default_env())
        .init();

    info!("ohaye");
    let (cmd_tx, state_rx) = audio_task();
    for url in env::args().skip(1) {
        cmd_tx.send(Command::EnqueueUrl(url))?;
    }
    cmd_tx.send(Command::Play)?;

    let terminal = ratatui::init();
    let terminal = Arc::new(Mutex::new(terminal));
    let app = Arc::new(App::new_short(cmd_tx));
    let app2 = app.clone();
    let t2 = terminal.clone();
    spawn(move || {
        app2.rx_thread(state_rx, t2);
    });

    app.run(terminal)?;
    ratatui::restore();
    Ok(())
}

type DynResult<T> = Result<T, Box<dyn std::error::Error>>;

fn audio_task() -> (SyncSender<Command>, Receiver<State>) {
    let (ctrl_tx, ctrl_rx) = mpsc::sync_channel(1);
    let (pos_tx, pos_rx) = mpsc::sync_channel(1);

    // TODO in async land, use select to send a position update every 0.5s or so
    let update_tx = ctrl_tx.clone();
    spawn(move || loop {
        sleep(Duration::from_millis(500));
        let res = update_tx.send(Command::Update);
    });
    let player_ctrl_tx = ctrl_tx.clone();
    let _thread_handle = spawn(move || {
        // _stream must live as long as the sink
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        let mut player = Player::new(sink, player_ctrl_tx);
        let mut handler = |cmd| -> DynResult<()> {
            match cmd {
                Command::Update => {
                    pos_tx.send(State::Pos(player.get_pos()))?;
                    if let Some(wat) = player.cur() {
                        let wat = wat.info();
                        pos_tx.send(State::Track(wat));
                    };
                }
                Command::SeekForward => {
                    player.try_seek(player.get_pos().add(Duration::from_secs(5)))?;
                    pos_tx.send(State::Pos(player.get_pos()))?;
                }
                Command::SeekBack => {
                    player.try_seek(
                        player
                            .get_pos()
                            .checked_sub(Duration::from_secs(5))
                            .unwrap_or_default(),
                    )?;
                    pos_tx.send(State::Pos(player.get_pos()))?;
                }
                Command::SeekForwardMucho => {
                    player.try_seek(player.get_pos().add(Duration::from_secs(25)))?;
                    pos_tx.send(State::Pos(player.get_pos()))?;
                }
                Command::SeekBackMucho => {
                    player.try_seek(
                        player
                            .get_pos()
                            .checked_sub(Duration::from_secs(25))
                            .unwrap_or_default(),
                    )?;
                    pos_tx.send(State::Pos(player.get_pos()))?;
                }
                Command::Next => {
                    player.next();
                }
                Command::Prev => todo!(),
                Command::EnqueueUrl(url) => {
                    let sr = SeekRequest::new(url);
                    player.append(sr);
                }

                Command::Enqueue(s) => match File::open(s) {
                    Ok(f) => {
                        let reader = BufReader::new(f);
                        todo!();
                        //player.append(reader);
                    }
                    Err(e) => error!("File: {e:?}"),
                },
                Command::Stop => {
                    info!("kbye");
                    // sink.stop();
                }
                Command::Play => {
                    // sink.play();
                }
                Command::Pause => {
                    // sink.pause();
                }
            }

            Ok(())
        };
        while let Ok(cmd) = ctrl_rx.recv() {
            if let Err(e) = handler(cmd) {
                error!("{e:?}");
            }
        }
    });
    (ctrl_tx, pos_rx)
}
