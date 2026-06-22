use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::diff::DiffLineKind;
use crate::viewer::Viewer;

const ADDITION_BG: Color = Color::Rgb(0, 50, 0);
const DELETION_MARKER: Color = Color::Rgb(200, 60, 60);
const SEARCH_HIGHLIGHT_BG: Color = Color::Rgb(120, 100, 0);
const GUTTER_ADDITION: &str = "+";
const GUTTER_DELETION: &str = "-";

pub fn draw(frame: &mut Frame, area: Rect, viewer: &mut Viewer, search_input: Option<&str>) {
    let bottom_height = if search_input.is_some() { 2 } else { 1 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(bottom_height)])
        .split(area);

    let content_area = chunks[0];
    let bottom_area = chunks[1];

    let inner_height = content_area.height.saturating_sub(2) as usize;
    viewer.update_viewport_height(inner_height);

    let highlighted = viewer.highlighter().highlight(viewer.content());

    let diff_marks = viewer
        .diff_state()
        .map(|d| d.line_marks.clone())
        .unwrap_or_default();

    let search_line_set: std::collections::HashSet<usize> = viewer
        .search_matches()
        .iter()
        .copied()
        .collect();

    let visible: Vec<Line> = highlighted
        .into_iter()
        .enumerate()
        .skip(viewer.scroll_offset())
        .take(inner_height)
        .map(|(i, line)| {
            let lineno = i + 1;
            let (marker, marker_style) = match diff_marks.get(&lineno) {
                Some(DiffLineKind::Addition) => (
                    GUTTER_ADDITION,
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ),
                Some(DiffLineKind::Deletion) => (
                    GUTTER_DELETION,
                    Style::default().fg(DELETION_MARKER).add_modifier(Modifier::BOLD),
                ),
                None => (" ", Style::default()),
            };

            let gutter = format!("{:>4} {} ", lineno, marker);
            let gutter_style = if diff_marks.contains_key(&lineno) {
                marker_style
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let mut spans = vec![Span::styled(gutter, gutter_style)];

            let is_addition = matches!(diff_marks.get(&lineno), Some(DiffLineKind::Addition));
            let is_search_hit = search_line_set.contains(&lineno);

            let bg = if is_search_hit {
                Some(SEARCH_HIGHLIGHT_BG)
            } else if is_addition {
                Some(ADDITION_BG)
            } else {
                None
            };

            if let Some(bg) = bg {
                spans.extend(line.spans.into_iter().map(|s| {
                    Span::styled(s.content, s.style.bg(bg))
                }));
            } else {
                spans.extend(line.spans);
            }

            Line::from(spans)
        })
        .collect();

    let title = if viewer.diff_state().is_some_and(|d| d.is_new_file) {
        format!("{} [new]", viewer.file_path())
    } else {
        viewer.file_path().to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title);

    let paragraph = Paragraph::new(visible).block(block);
    frame.render_widget(paragraph, content_area);

    if let Some(input) = search_input {
        let bottom_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(bottom_area);

        let status = build_status_line(viewer);
        frame.render_widget(Paragraph::new(status), bottom_chunks[0]);

        let search_line = Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(input.to_string()),
            Span::styled("_", Style::default().fg(Color::White).add_modifier(Modifier::SLOW_BLINK)),
        ]);
        frame.render_widget(Paragraph::new(search_line), bottom_chunks[1]);
    } else {
        let status = build_status_line(viewer);
        frame.render_widget(Paragraph::new(status), bottom_area);
    }
}

fn build_status_line<'a>(viewer: &Viewer) -> Line<'a> {
    let percent = if viewer.total_lines() == 0 {
        100
    } else {
        ((viewer.scroll_offset() + 1) * 100) / viewer.total_lines().max(1)
    };

    let mut parts = vec![
        format!(" {}", viewer.file_path()),
        format!("L:{}", viewer.total_lines()),
        format!("{}%", percent),
    ];

    if let Some(diff) = viewer.diff_state() {
        let hunk_count = diff.hunks.len();
        if hunk_count > 0 {
            parts.push(format!(
                "diff:{}/{}",
                viewer.current_hunk() + 1,
                hunk_count
            ));
        }
    }

    if let Some(query) = viewer.search_query() {
        let match_count = viewer.search_matches().len();
        if match_count > 0 {
            parts.push(format!(
                "\"{}\":{}/{}",
                query,
                viewer.current_match() + 1,
                match_count
            ));
        } else {
            parts.push(format!("\"{}\":no match", query));
        }
    }

    parts.push("q:quit /:search n/N:next/prev".to_string());

    let status_text = parts.join(" | ");

    Line::from(Span::styled(
        status_text,
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    ))
}
