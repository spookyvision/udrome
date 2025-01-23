use std::{
    io,
    sync::mpsc::{Receiver, SyncSender},
    thread::spawn,
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
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
}

#[derive(Debug)]
pub enum State {
    Pos(Duration),
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug)]
pub struct App {
    counter: u8,
    exit: bool,
    cmd_tx: SyncSender<Command>,
}
impl App {
    pub fn new(counter: u8, exit: bool, cmd_tx: SyncSender<Command>) -> Self {
        Self {
            counter,
            exit,
            cmd_tx,
        }
    }

    pub fn new_short(cmd_tx: SyncSender<Command>) -> Self {
        Self::new(Default::default(), Default::default(), cmd_tx)
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
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

    fn handle_events(&mut self) -> io::Result<()> {
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
    /// runs the application's main loop until the user quits
    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        state_rx: Receiver<State>,
    ) -> io::Result<()> {
        let _handle = spawn(move || {
            while let Ok(state) = state_rx.recv() {
                match state {
                    State::Pos(pos) => debug!("new pos {pos:?}"),
                    State::Playing => debug!("Play"),
                    State::Paused => debug!("Pause"),
                    State::Stopped => debug!("Stop"),
                }
            }
        });
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }
    fn exit(&mut self) {
        self.exit = true;
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn seek_back(&mut self) {
        if let Err(e) = self.cmd_tx.send(Command::SeekBack) {
            error!("{e:?}")
        }
        self.counter = self.counter.wrapping_sub(1);
    }

    fn seek_forward(&mut self) {
        if let Err(e) = self.cmd_tx.send(Command::SeekForward) {
            error!("{e:?}")
        }
        self.counter = self.counter.wrapping_add(1);
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

        let counter_text = Text::from(vec![Line::from(vec![
            "Value: ".into(),
            self.counter.to_string().yellow(),
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
