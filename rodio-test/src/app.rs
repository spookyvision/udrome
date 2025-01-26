use std::{
    io,
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc::{Receiver, SyncSender},
        Arc, Mutex,
    },
    thread::spawn,
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame, Terminal,
};
use tracing::{debug, error};
use tui_logger::{
    LevelFilter, TuiLoggerLevelOutput, TuiLoggerSmartWidget, TuiLoggerWidget, TuiWidgetState,
};

#[derive(Debug, Clone)]

pub enum Command {
    SeekForward,
    SeekBack,
    SeekForwardMucho,
    SeekBackMucho,
    Next,
    Prev,
    Enqueue(String),
    EnqueueUrl(String),
    Play,
    Pause,
    Stop,
    Update,
}

#[derive(Debug)]
pub enum State {
    Pos(Duration),
    Track(String),
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Default, Clone)]
struct PlaybackInfo {
    track: String,
    pos: u32,
}

#[derive(Debug, Default)]
struct AppData {
    info: Option<PlaybackInfo>,
    exit: bool,
}
#[derive(Debug)]
pub struct App {
    data: Arc<Mutex<AppData>>,
    cmd_tx: SyncSender<Command>,
}
impl App {
    pub fn new(data: AppData, cmd_tx: SyncSender<Command>) -> Self {
        Self {
            data: Arc::new(Mutex::new(data)),
            cmd_tx,
        }
    }

    pub fn new_short(cmd_tx: SyncSender<Command>) -> Self {
        Self::new(Default::default(), cmd_tx)
    }

    fn handle_key_event(&self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Left => self.seek_back(),
            KeyCode::Right => self.seek_forward(),
            KeyCode::Down => self.seek_back_mucho(),
            KeyCode::Up => self.seek_forward_much(),
            KeyCode::PageUp => self.next(),
            _ => {
                debug!("unhandled key event: {key_event:?}");
            }
        }
    }

    fn handle_events(&self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    pub fn rx_thread(&self, state_rx: Receiver<State>, terminal: Arc<Mutex<DefaultTerminal>>) {
        let data = self.data.clone();
        while let Ok(state) = state_rx.recv() {
            match state {
                State::Pos(pos) => {
                    let secs = pos.as_secs();
                    let mut data = data.lock().unwrap();
                    if data.info.is_none() {
                        data.info = Some(Default::default());
                    }
                    data.info.as_mut().unwrap().pos = secs as u32;
                }
                State::Track(track) => {
                    let mut data = data.lock().unwrap();
                    if data.info.is_none() {
                        data.info = Some(Default::default());
                    }
                    data.info.as_mut().unwrap().track = track;
                }

                State::Playing => debug!("Play"),
                State::Paused => debug!("Pause"),
                State::Stopped => debug!("Stop"),
            }

            let mut terminal = terminal.lock().unwrap();
            if let Err(e) = terminal.draw(|frame| self.draw(frame)) {
                error!("draw: {e:?}")
            }
        }
    }
    /// runs the application's main loop until the user quits
    pub fn run(&self, terminal: Arc<Mutex<DefaultTerminal>>) -> io::Result<()> {
        let mut exit = false;
        while !exit {
            let mut terminal = terminal.lock().unwrap();
            terminal.draw(|frame| self.draw(frame))?;
            drop(terminal);
            self.handle_events()?;
            let data = self.data.lock().unwrap();
            exit = data.exit;
        }
        Ok(())
    }
    fn exit(&self) {
        let mut data = self.data.lock().unwrap();
        data.exit = true;
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn seek_back(&self) {
        if let Err(e) = self.cmd_tx.send(Command::SeekBack) {
            error!("{e:?}")
        }
    }

    fn seek_forward(&self) {
        if let Err(e) = self.cmd_tx.send(Command::SeekForward) {
            error!("{e:?}")
        }
    }

    fn seek_back_mucho(&self) {
        if let Err(e) = self.cmd_tx.send(Command::SeekBackMucho) {
            error!("{e:?}")
        }
    }

    fn seek_forward_much(&self) {
        if let Err(e) = self.cmd_tx.send(Command::SeekForwardMucho) {
            error!("{e:?}")
        }
    }

    fn next(&self) {
        if let Err(e) = self.cmd_tx.send(Command::Next) {
            error!("{e:?}")
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Counter App Tutorial ".bold());
        let instructions = Line::from(vec![
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let data = self.data.lock().unwrap();
        let info = data.info.clone().unwrap_or_default();
        let secs = info.pos % 60;
        let mins = info.pos / 60;
        let hours = info.pos / 3600;

        let pos = format!("{hours:02}:{mins:02}:{secs:02}");

        let counter_text = Text::from(vec![Line::from(vec![
            info.track.white(),
            " ".into(),
            pos.yellow(),
        ])]);

        let [left, right] = Layout::horizontal([Constraint::Fill(1); 2]).areas(area);

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(left, buf);

        TuiLoggerWidget::default()
            .block(Block::bordered().title("Log!"))
            .style_error(Style::default().fg(Color::Red))
            .style_debug(Style::default().fg(Color::Green))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_trace(Style::default().fg(Color::Magenta))
            .style_info(Style::default().fg(Color::Cyan))
            // .state(
            //     &TuiWidgetState::default()
            //         .set_default_display_level(LevelFilter::Warn)
            //         .set_level_for_target("rodio_test", LevelFilter::Debug)
            //         .set_level_for_target("rodio_test::app", LevelFilter::Debug),
            // )
            .render(right, buf);
    }
}
