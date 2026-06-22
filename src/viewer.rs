use anyhow::Result;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::diff::DiffState;
use crate::highlight::Highlighter;
use crate::ui;

pub struct Viewer {
    file_path: String,
    highlighter: Highlighter,
    content: String,
    scroll_offset: usize,
    total_lines: usize,
    viewport_height: usize,
    diff_state: Option<DiffState>,
    current_hunk: usize,
}

impl Viewer {
    pub fn new(file_path: String, highlighter: Highlighter, diff_state: Option<DiffState>) -> Result<Self> {
        let content = std::fs::read_to_string(&file_path)?;
        let total_lines = content.lines().count();
        Ok(Self {
            file_path,
            highlighter,
            content,
            scroll_offset: 0,
            total_lines,
            viewport_height: 0,
            diff_state,
            current_hunk: 0,
        })
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        ui::draw(frame, area, self);
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.total_lines.saturating_sub(self.viewport_height);
        self.scroll_offset = (self.scroll_offset + n).min(max);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn page_down(&mut self) {
        self.scroll_down(self.viewport_height.saturating_sub(2));
    }

    pub fn page_up(&mut self) {
        self.scroll_up(self.viewport_height.saturating_sub(2));
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.total_lines.saturating_sub(self.viewport_height);
    }

    pub fn next_diff(&mut self) {
        if let Some(ref diff) = self.diff_state {
            let starts = diff.hunk_start_lines();
            if starts.is_empty() {
                return;
            }
            if self.current_hunk + 1 < starts.len() {
                self.current_hunk += 1;
            }
            self.scroll_offset = starts[self.current_hunk].saturating_sub(1) as usize;
        }
    }

    pub fn prev_diff(&mut self) {
        if let Some(ref diff) = self.diff_state {
            let starts = diff.hunk_start_lines();
            if starts.is_empty() {
                return;
            }
            if self.current_hunk > 0 {
                self.current_hunk -= 1;
            }
            self.scroll_offset = starts[self.current_hunk].saturating_sub(1) as usize;
        }
    }

    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn highlighter(&self) -> &Highlighter {
        &self.highlighter
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn total_lines(&self) -> usize {
        self.total_lines
    }

    pub fn diff_state(&self) -> Option<&DiffState> {
        self.diff_state.as_ref()
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
    }
}
