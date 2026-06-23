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
    current_hunk: Option<usize>,
    search_query: Option<String>,
    search_matches: Vec<usize>,
    current_match: usize,
    lineno_to_display_row: std::collections::HashMap<usize, usize>,
}

impl Viewer {
    pub fn new(
        file_path: String,
        highlighter: Highlighter,
        diff_state: Option<DiffState>,
    ) -> Result<Self> {
        let content = std::fs::read_to_string(&file_path)?;
        Ok(Self::from_content(
            content,
            file_path,
            highlighter,
            diff_state,
        ))
    }

    pub fn from_content(
        content: String,
        file_label: String,
        highlighter: Highlighter,
        diff_state: Option<DiffState>,
    ) -> Self {
        let total_lines = content.lines().count();
        Self {
            file_path: file_label,
            highlighter,
            content,
            scroll_offset: 0,
            total_lines,
            viewport_height: 0,
            diff_state,
            current_hunk: None,
            search_query: None,
            search_matches: Vec::new(),
            current_match: 0,
            lineno_to_display_row: std::collections::HashMap::new(),
        }
    }

    pub fn draw_with_search_input(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        search_input: Option<&str>,
    ) {
        ui::draw(frame, area, self, search_input);
    }

    pub fn update_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
    }

    pub fn update_total_display_lines(&mut self, count: usize) {
        self.total_lines = count;
    }

    pub fn update_lineno_to_display_row(&mut self, map: std::collections::HashMap<usize, usize>) {
        self.lineno_to_display_row = map;
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.max_scroll();
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

    pub fn half_page_down(&mut self) {
        self.scroll_down(self.viewport_height / 2);
    }

    pub fn half_page_up(&mut self) {
        self.scroll_up(self.viewport_height / 2);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.max_scroll();
    }

    pub fn next_match_or_diff(&mut self) {
        if self.search_query.is_some() {
            self.next_match();
        } else {
            self.next_diff();
        }
    }

    pub fn prev_match_or_diff(&mut self) {
        if self.search_query.is_some() {
            self.prev_match();
        } else {
            self.prev_diff();
        }
    }

    pub fn search(&mut self, query: String) {
        if query.is_empty() {
            self.clear_search();
            return;
        }

        let query_lower = query.to_lowercase();
        let mut matches = Vec::new();
        for (i, line) in self.content.lines().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                matches.push(i + 1);
            }
        }

        self.search_query = Some(query);
        self.search_matches = matches;
        self.current_match = 0;

        if !self.search_matches.is_empty() {
            self.scroll_to_line(self.search_matches[0]);
        }
    }

    pub fn clear_search(&mut self) {
        self.search_query = None;
        self.search_matches.clear();
        self.current_match = 0;
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

    pub fn current_hunk(&self) -> usize {
        self.current_hunk.unwrap_or(0)
    }

    pub fn search_query(&self) -> Option<&str> {
        self.search_query.as_deref()
    }

    pub fn search_matches(&self) -> &[usize] {
        &self.search_matches
    }

    pub fn current_match(&self) -> usize {
        self.current_match
    }

    fn next_diff(&mut self) {
        if let Some(ref diff) = self.diff_state {
            let groups = &diff.change_groups;
            if groups.is_empty() {
                return;
            }
            let next = match self.current_hunk {
                None => 0,
                Some(i) if i + 1 < groups.len() => i + 1,
                Some(i) => i,
            };
            self.current_hunk = Some(next);
            self.scroll_to_line(groups[next]);
        }
    }

    fn prev_diff(&mut self) {
        if let Some(ref diff) = self.diff_state {
            let groups = &diff.change_groups;
            if groups.is_empty() {
                return;
            }
            let prev = match self.current_hunk {
                None => 0,
                Some(i) if i > 0 => i - 1,
                Some(i) => i,
            };
            self.current_hunk = Some(prev);
            self.scroll_to_line(groups[prev]);
        }
    }

    fn next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.current_match = (self.current_match + 1) % self.search_matches.len();
        self.scroll_to_line(self.search_matches[self.current_match]);
    }

    fn prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.current_match == 0 {
            self.current_match = self.search_matches.len() - 1;
        } else {
            self.current_match -= 1;
        }
        self.scroll_to_line(self.search_matches[self.current_match]);
    }

    fn max_scroll(&self) -> usize {
        self.total_lines.saturating_sub(self.viewport_height)
    }

    fn scroll_to_line(&mut self, lineno: usize) {
        let display_row = self
            .lineno_to_display_row
            .get(&lineno)
            .copied()
            .unwrap_or_else(|| lineno.saturating_sub(1));
        let context_lines = self.viewport_height / 4;
        self.scroll_offset = display_row
            .saturating_sub(context_lines)
            .min(self.max_scroll());
    }
}
