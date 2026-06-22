use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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
        self.viewer
            .draw_with_search_input(frame, frame.area(), search_input);
    }

    fn handle_events(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }
            self.handle_key(key);
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
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
                KeyCode::Backspace => self.viewer.page_up(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> App {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        for i in 0..100 {
            writeln!(tmp, "line {}", i).unwrap();
        }
        let path = tmp.path().to_str().unwrap().to_string();
        std::mem::forget(tmp);
        App::new(path, None).unwrap()
    }

    fn make_key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn space_pages_down() {
        let mut app = create_test_app();
        app.viewer.update_viewport_height(10);

        app.handle_key(make_key(KeyCode::Char(' '), KeyModifiers::NONE));

        assert!(app.viewer.scroll_offset() > 0);
    }

    #[test]
    fn backspace_pages_up() {
        let mut app = create_test_app();
        app.viewer.update_viewport_height(10);
        app.handle_key(make_key(KeyCode::Char(' '), KeyModifiers::NONE));
        app.handle_key(make_key(KeyCode::Char(' '), KeyModifiers::NONE));
        let offset_after_page_down = app.viewer.scroll_offset();

        app.handle_key(make_key(KeyCode::Backspace, KeyModifiers::NONE));

        assert!(app.viewer.scroll_offset() < offset_after_page_down);
    }
}
