use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::viewer::Viewer;
use crate::word_diff;

const ADDITION_BG: Color = Color::Rgb(0, 50, 0);
const ADDITION_HIGHLIGHT_BG: Color = Color::Rgb(0, 130, 0);
const DELETION_BG: Color = Color::Rgb(50, 0, 0);
const DELETION_HIGHLIGHT_BG: Color = Color::Rgb(180, 0, 0);
const SEARCH_HIGHLIGHT_BG: Color = Color::Rgb(120, 100, 0);

pub fn draw(frame: &mut Frame, area: Rect, viewer: &mut Viewer, search_input: Option<&str>) {
    let bottom_height = if search_input.is_some() { 2 } else { 1 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(bottom_height)])
        .split(area);

    let content_area = chunks[0];
    let bottom_area = chunks[1];

    let inner_height = content_area.height as usize;
    viewer.update_viewport_height(inner_height);

    let (display_lines, lineno_map) = build_display_lines(viewer);
    viewer.update_total_display_lines(display_lines.len());
    viewer.update_lineno_to_display_row(lineno_map);

    let visible: Vec<Line> = display_lines
        .into_iter()
        .skip(viewer.scroll_offset())
        .take(inner_height)
        .collect();

    let paragraph = Paragraph::new(visible);
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

fn build_display_lines<'a>(
    viewer: &Viewer,
) -> (Vec<Line<'a>>, std::collections::HashMap<usize, usize>) {
    let highlighted = viewer.highlighter().highlight(viewer.content());

    let addition_marks = viewer
        .diff_state()
        .map(|d| &d.addition_lines)
        .cloned()
        .unwrap_or_default();

    let addition_contents = viewer
        .diff_state()
        .map(|d| &d.addition_contents)
        .cloned()
        .unwrap_or_default();

    let deleted_map = viewer
        .diff_state()
        .map(|d| &d.deleted_lines)
        .cloned()
        .unwrap_or_default();

    let word_diff_pairs = viewer
        .diff_state()
        .map(|d| &d.word_diff_pairs)
        .cloned()
        .unwrap_or_default();

    let paired_del_contents: std::collections::HashSet<String> =
        word_diff_pairs.values().cloned().collect();

    let search_line_set: std::collections::HashSet<usize> =
        viewer.search_matches().iter().copied().collect();

    let mut lines: Vec<Line<'a>> = Vec::new();
    let mut lineno_map: std::collections::HashMap<usize, usize> =
        std::collections::HashMap::new();

    if let Some(deleted) = deleted_map.get(&0) {
        for d in deleted {
            lines.push(make_deleted_line(&d.content));
        }
    }

    for (i, hl_line) in highlighted.into_iter().enumerate() {
        let lineno = i + 1;

        lineno_map.insert(lineno, lines.len());

        let is_addition = addition_marks.contains_key(&lineno);
        let is_search_hit = search_line_set.contains(&lineno);

        if is_addition {
            if let Some(old_text) = word_diff_pairs.get(&lineno) {
                let add_text = addition_contents.get(&lineno).map(|s| s.as_str()).unwrap_or("");
                let (_, new_spans) = word_diff::compute_word_diff(old_text, add_text);
                lines.push(make_word_diff_addition_line(lineno, &new_spans));
            } else {
                let gutter = format!("{:>4} {} ", lineno, "+");
                let gutter_style = Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD);
                let mut spans = vec![Span::styled(gutter, gutter_style)];
                let bg = if is_search_hit {
                    SEARCH_HIGHLIGHT_BG
                } else {
                    ADDITION_BG
                };
                spans.extend(
                    hl_line
                        .spans
                        .into_iter()
                        .map(|s| Span::styled(s.content, s.style.bg(bg))),
                );
                lines.push(Line::from(spans));
            }
        } else {
            let gutter = format!("{:>4}   ", lineno);
            let gutter_style = Style::default().fg(Color::DarkGray);
            let mut spans = vec![Span::styled(gutter, gutter_style)];

            if is_search_hit {
                spans.extend(
                    hl_line
                        .spans
                        .into_iter()
                        .map(|s| Span::styled(s.content, s.style.bg(SEARCH_HIGHLIGHT_BG))),
                );
            } else {
                spans.extend(hl_line.spans);
            }

            lines.push(Line::from(spans));
        }

        if let Some(deleted) = deleted_map.get(&lineno) {
            for d in deleted {
                let next_add_lineno = lineno + 1;
                if let Some(paired_old) = word_diff_pairs.get(&next_add_lineno) {
                    if paired_old == &d.content {
                        let add_text = addition_contents
                            .get(&next_add_lineno)
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        let (old_spans, _) =
                            word_diff::compute_word_diff(&d.content, add_text);
                        lines.push(make_word_diff_deleted_line(&old_spans));
                        continue;
                    }
                }
                if paired_del_contents.contains(&d.content) {
                    let paired_add = word_diff_pairs.iter().find(|(_, v)| **v == d.content);
                    if let Some((&add_lineno, _)) = paired_add {
                        let add_text = addition_contents
                            .get(&add_lineno)
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        let (old_spans, _) =
                            word_diff::compute_word_diff(&d.content, add_text);
                        lines.push(make_word_diff_deleted_line(&old_spans));
                        continue;
                    }
                }
                lines.push(make_deleted_line(&d.content));
            }
        }
    }

    (lines, lineno_map)
}

fn make_word_diff_addition_line(lineno: usize, new_spans: &[word_diff::WordSpan]) -> Line<'static> {
    let gutter = format!("{:>4} {} ", lineno, "+");
    let gutter_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let mut spans = vec![Span::styled(gutter, gutter_style)];

    for ws in new_spans {
        let style = if ws.changed {
            Style::default()
                .fg(Color::White)
                .bg(ADDITION_HIGHLIGHT_BG)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(Color::White).bg(ADDITION_BG)
        };
        spans.push(Span::styled(ws.text.clone(), style));
    }

    Line::from(spans)
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
            Style::default().fg(Color::White).bg(DELETION_BG),
        ),
    ])
}

fn make_word_diff_deleted_line(old_spans: &[word_diff::WordSpan]) -> Line<'static> {
    let gutter = format!("{:>4} {} ", "", "-");
    let mut spans = vec![Span::styled(
        gutter,
        Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD),
    )];

    for ws in old_spans {
        let style = if ws.changed {
            Style::default()
                .fg(Color::White)
                .bg(DELETION_HIGHLIGHT_BG)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(Color::White).bg(DELETION_BG)
        };
        spans.push(Span::styled(ws.text.clone(), style));
    }

    Line::from(spans)
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
        let group_count = diff.change_groups.len();
        if group_count > 0 {
            parts.push(format!(
                "diff:{}/{}",
                viewer.current_hunk() + 1,
                group_count
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
