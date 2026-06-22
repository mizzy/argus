use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{DefaultTerminal, Frame};

use crate::diff::DiffState;
use crate::highlight::Highlighter;
use crate::viewer::Viewer;

enum Mode {
    Normal,
    Search(String),
}

pub struct App {
    viewer: Viewer,
    mode: Mode,
    should_quit: bool,
}

impl App {
    pub fn new(file: String, rev: Option<String>) -> Result<Self> {
        let highlighter = Highlighter::new(&file)?;
        let diff_state = DiffState::load(&file, rev.as_deref()).ok();
        let viewer = Viewer::new(file, highlighter, diff_state)?;

        Ok(Self {
            viewer,
            mode: Mode::Normal,
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

    fn draw(&mut self, frame: &mut Frame) {
        let search_input = match &self.mode {
            Mode::Search(input) => Some(input.as_str()),
            Mode::Normal => None,
        };
        self.viewer.draw_with_search_input(frame, frame.area(), search_input);
    }

    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }
            match &mut self.mode {
                Mode::Search(input) => match key.code {
                    KeyCode::Enter => {
                        let query = std::mem::take(input);
                        self.mode = Mode::Normal;
                        self.viewer.search(query);
                    }
                    KeyCode::Esc => {
                        self.mode = Mode::Normal;
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                    }
                    _ => {}
                },
                Mode::Normal => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    KeyCode::Down | KeyCode::Char('j') => self.viewer.scroll_down(1),
                    KeyCode::Up | KeyCode::Char('k') => self.viewer.scroll_up(1),
                    KeyCode::PageDown | KeyCode::Char(' ') => self.viewer.page_down(),
                    KeyCode::PageUp => self.viewer.page_up(),
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.viewer.page_up();
                    }
                    KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.viewer.page_down();
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.viewer.half_page_down();
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.viewer.half_page_up();
                    }
                    KeyCode::Home | KeyCode::Char('g') => self.viewer.scroll_to_top(),
                    KeyCode::End | KeyCode::Char('G') => self.viewer.scroll_to_bottom(),
                    KeyCode::Char('n') => self.viewer.next_match_or_diff(),
                    KeyCode::Char('N') => self.viewer.prev_match_or_diff(),
                    KeyCode::Char('/') => {
                        self.viewer.clear_search();
                        self.mode = Mode::Search(String::new());
                    }
                    _ => {}
                },
            }
        }
        Ok(())
    }
}
