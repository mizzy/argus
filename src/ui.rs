use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::diff::DiffLineKind;
use crate::viewer::Viewer;

pub fn draw(frame: &mut Frame, area: Rect, viewer: &Viewer) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let content_area = chunks[0];
    let status_area = chunks[1];

    let inner_height = content_area.height.saturating_sub(2) as usize;

    let highlighted = viewer.highlighter().highlight(viewer.content());

    let diff_line_colors = build_diff_line_colors(viewer);

    let visible: Vec<Line> = highlighted
        .into_iter()
        .enumerate()
        .skip(viewer.scroll_offset())
        .take(inner_height)
        .map(|(i, line)| {
            let lineno = format!("{:>4} ", i + 1);
            let mut spans = vec![Span::styled(
                lineno,
                Style::default().fg(Color::DarkGray),
            )];

            if let Some(&bg) = diff_line_colors.get(&(i + 1)) {
                spans.extend(line.spans.into_iter().map(|s| {
                    Span::styled(s.content, s.style.bg(bg))
                }));
            } else {
                spans.extend(line.spans);
            }

            Line::from(spans)
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(viewer.file_path().to_string());

    let paragraph = Paragraph::new(visible).block(block);
    frame.render_widget(paragraph, content_area);

    let status = build_status_line(viewer);
    let status_widget = Paragraph::new(status);
    frame.render_widget(status_widget, status_area);
}

fn build_diff_line_colors(viewer: &Viewer) -> std::collections::HashMap<usize, Color> {
    let mut colors = std::collections::HashMap::new();
    if let Some(diff) = viewer.diff_state() {
        for line in &diff.lines {
            if let Some(lineno) = line.new_lineno {
                let color = match line.kind {
                    DiffLineKind::Addition => Color::Rgb(0, 60, 0),
                    DiffLineKind::Deletion => Color::Rgb(60, 0, 0),
                    DiffLineKind::Context => continue,
                };
                colors.insert(lineno as usize, color);
            }
        }
    }
    colors
}

fn build_status_line<'a>(viewer: &Viewer) -> Line<'a> {
    let percent = if viewer.total_lines() == 0 {
        100
    } else {
        (viewer.scroll_offset() * 100) / viewer.total_lines().max(1)
    };

    let status_text = format!(
        " {} | Lines: {} | {}% | q:quit j/k:scroll n/N:diff",
        viewer.file_path(),
        viewer.total_lines(),
        percent,
    );

    Line::from(Span::styled(
        status_text,
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    ))
}
