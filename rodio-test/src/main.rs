use std::{
    env,
    fs::File,
    io::BufReader,
    ops::Add,
    sync::mpsc::{self, Receiver, SyncSender},
    thread::spawn,
    time::Duration,
};

use app::{App, Command, State};
use rodio::{Decoder, OutputStream, Sink};
use tracing::{debug, error, info};
use tracing_subscriber::{prelude::*, EnvFilter};
use tui_logger::init_logger;

mod app;
mod http;
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

    let mut terminal = ratatui::init();
    App::new_short(cmd_tx).run(&mut terminal, state_rx)?;
    ratatui::restore();
    Ok(())
}

type DynResult<T> = Result<T, Box<dyn std::error::Error>>;

fn audio_task() -> (SyncSender<Command>, Receiver<State>) {
    let (ctrl_tx, ctrl_rx) = mpsc::sync_channel(1);
    let (pos_tx, pos_rx) = mpsc::sync_channel(1);
    let _thread_handle = spawn(move || {
        // _stream must live as long as the sink
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        let handler = |cmd| -> DynResult<()> {
            match cmd {
                Command::SeekForward => {
                    sink.try_seek(sink.get_pos().add(Duration::from_secs(5)))?;
                    pos_tx.send(State::Pos(sink.get_pos()))?;
                }
                Command::SeekBack => {
                    sink.try_seek(
                        sink.get_pos()
                            .checked_sub(Duration::from_secs(5))
                            .unwrap_or_default(),
                    )?;
                    pos_tx.send(State::Pos(sink.get_pos()))?;
                }
                Command::SeekForwardMucho => {
                    sink.try_seek(sink.get_pos().add(Duration::from_secs(25)))?;
                    pos_tx.send(State::Pos(sink.get_pos()))?;
                }
                Command::SeekBackMucho => {
                    sink.try_seek(
                        sink.get_pos()
                            .checked_sub(Duration::from_secs(25))
                            .unwrap_or_default(),
                    )?;
                    pos_tx.send(State::Pos(sink.get_pos()))?;
                }
                Command::Next => {
                    sink.skip_one();
                }
                Command::Prev => todo!(),
                Command::EnqueueUrl(url) => {
                    let sr = crate::http::SeekRequest::new(url);
                    match Decoder::new(sr) {
                        Ok(decoder) => {
                            sink.append(decoder);
                        }
                        Err(e) => error!("Decoder: {e:?}"),
                    }
                }

                Command::Enqueue(s) => match File::open(s) {
                    Ok(f) => {
                        let reader = BufReader::new(f);
                        match Decoder::new(reader) {
                            Ok(decoder) => {
                                sink.append(decoder);
                            }
                            Err(e) => error!("Decoder: {e:?}"),
                        }
                        // sink.append(re);
                    }
                    Err(e) => error!("File: {e:?}"),
                },
                Command::Stop => {
                    info!("kbye");
                    sink.stop();
                }
                Command::Play => {
                    sink.play();
                }
                Command::Pause => {
                    sink.pause();
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
