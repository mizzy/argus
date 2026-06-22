use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::viewer::Viewer;

const ADDITION_BG: Color = Color::Rgb(0, 50, 0);
const DELETION_BG: Color = Color::Rgb(50, 0, 0);
const SEARCH_HIGHLIGHT_BG: Color = Color::Rgb(120, 100, 0);

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

    let display_lines = build_display_lines(viewer);
    viewer.update_total_display_lines(display_lines.len());

    let visible: Vec<Line> = display_lines
        .into_iter()
        .skip(viewer.scroll_offset())
        .take(inner_height)
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
            Span::styled(
                "_",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ]);
        frame.render_widget(Paragraph::new(search_line), bottom_chunks[1]);
    } else {
        let status = build_status_line(viewer);
        frame.render_widget(Paragraph::new(status), bottom_area);
    }
}

fn build_display_lines<'a>(viewer: &Viewer) -> Vec<Line<'a>> {
    let highlighted = viewer.highlighter().highlight(viewer.content());

    let addition_marks = viewer
        .diff_state()
        .map(|d| &d.addition_lines)
        .cloned()
        .unwrap_or_default();

    let deleted_map = viewer
        .diff_state()
        .map(|d| &d.deleted_lines)
        .cloned()
        .unwrap_or_default();

    let search_line_set: std::collections::HashSet<usize> = viewer
        .search_matches()
        .iter()
        .copied()
        .collect();

    let mut lines: Vec<Line<'a>> = Vec::new();

    if let Some(deleted) = deleted_map.get(&0) {
        for d in deleted {
            lines.push(make_deleted_line(&d.content));
        }
    }

    for (i, hl_line) in highlighted.into_iter().enumerate() {
        let lineno = i + 1;

        let is_addition = addition_marks.contains_key(&lineno);
        let is_search_hit = search_line_set.contains(&lineno);

        let (marker, marker_style) = if is_addition {
            (
                "+",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (" ", Style::default())
        };

        let gutter = format!("{:>4} {} ", lineno, marker);
        let gutter_style = if is_addition {
            marker_style
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let mut spans = vec![Span::styled(gutter, gutter_style)];

        let bg = if is_search_hit {
            Some(SEARCH_HIGHLIGHT_BG)
        } else if is_addition {
            Some(ADDITION_BG)
        } else {
            None
        };

        if let Some(bg) = bg {
            spans.extend(
                hl_line
                    .spans
                    .into_iter()
                    .map(|s| Span::styled(s.content, s.style.bg(bg))),
            );
        } else {
            spans.extend(hl_line.spans);
        }

        lines.push(Line::from(spans));

        if let Some(deleted) = deleted_map.get(&lineno) {
            for d in deleted {
                lines.push(make_deleted_line(&d.content));
            }
        }
    }

    lines
}

fn make_deleted_line(content: &str) -> Line<'static> {
    let gutter = format!("{:>4} {} ", "", "-");
    Line::from(vec![
        Span::styled(
            gutter,
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            content.to_string(),
            Style::default().fg(Color::Red).bg(DELETION_BG),
        ),
    ])
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
