use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{DefaultTerminal, Frame};

use crate::diff::DiffState;
use crate::highlight::Highlighter;
use crate::viewer::Viewer;

pub struct App {
    viewer: Viewer,
    should_quit: bool,
}

impl App {
    pub fn new(file: String) -> Result<Self> {
        let highlighter = Highlighter::new(&file)?;
        let diff_state = DiffState::load(&file).ok();
        let viewer = Viewer::new(file, highlighter, diff_state)?;

        Ok(Self {
            viewer,
            should_quit: false,
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        self.viewer.draw(frame, frame.area());
    }

    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                KeyCode::Down | KeyCode::Char('j') => self.viewer.scroll_down(1),
                KeyCode::Up | KeyCode::Char('k') => self.viewer.scroll_up(1),
                KeyCode::PageDown | KeyCode::Char(' ') => self.viewer.page_down(),
                KeyCode::PageUp | KeyCode::Char('b') => self.viewer.page_up(),
                KeyCode::Home | KeyCode::Char('g') => self.viewer.scroll_to_top(),
                KeyCode::End | KeyCode::Char('G') => self.viewer.scroll_to_bottom(),
                KeyCode::Char('n') => self.viewer.next_diff(),
                KeyCode::Char('N') => self.viewer.prev_diff(),
                _ => {}
            }
        }
        Ok(())
    }
}
