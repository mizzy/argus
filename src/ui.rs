use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthChar;

use crate::viewer::Viewer;
use crate::word_diff;

const ADDITION_BG: Color = Color::Rgb(0, 50, 0);
const ADDITION_HIGHLIGHT_BG: Color = Color::Rgb(0, 80, 0);
const DELETION_BG: Color = Color::Rgb(50, 0, 0);
const DELETION_HIGHLIGHT_BG: Color = Color::Rgb(120, 0, 0);
const SEARCH_HIGHLIGHT_BG: Color = Color::Rgb(120, 100, 0);
const GUTTER_WIDTH: usize = 7;

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

    let (display_lines, lineno_map) = build_display_lines(viewer, content_area.width);
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

fn build_display_lines(
    viewer: &Viewer,
    width: u16,
) -> (Vec<Line<'static>>, std::collections::HashMap<usize, usize>) {
    let highlighted = viewer.highlighter().highlight(viewer.content());
    let content_width = width.saturating_sub(GUTTER_WIDTH as u16) as usize;

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

    let search_line_set: std::collections::HashSet<usize> =
        viewer.search_matches().iter().copied().collect();

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut lineno_map: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();

    if let Some(deleted) = deleted_map.get(&0) {
        for d in deleted {
            append_wrapped_line(
                &mut lines,
                format!("{:>4} {} ", "", "-"),
                Style::default().fg(Color::Red),
                vec![Span::styled(
                    d.content.clone(),
                    Style::default().fg(Color::White).bg(DELETION_BG),
                )],
                content_width,
            );
        }
    }

    for (i, hl_line) in highlighted.into_iter().enumerate() {
        let lineno = i + 1;

        lineno_map.insert(lineno, lines.len());

        let is_addition = addition_marks.contains_key(&lineno);
        let is_search_hit = search_line_set.contains(&lineno);

        if is_addition {
            let word_diff_result = word_diff_pairs.get(&lineno).and_then(|old_text| {
                let add_text = addition_contents
                    .get(&lineno)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let (_, new_spans) = word_diff::compute_word_diff_if_useful(old_text, add_text)?;
                Some(new_spans)
            });
            if let Some(new_spans) = word_diff_result {
                append_wrapped_line(
                    &mut lines,
                    format!("{:>4} {} ", lineno, "+"),
                    Style::default().fg(Color::Green),
                    word_diff_addition_spans(&new_spans),
                    content_width,
                );
            } else {
                let gutter = format!("{:>4} {} ", lineno, "+");
                let gutter_style = Style::default().fg(Color::Green);
                let bg = if is_search_hit {
                    SEARCH_HIGHLIGHT_BG
                } else {
                    ADDITION_BG
                };
                let spans = hl_line
                    .spans
                    .into_iter()
                    .map(|s| Span::styled(s.content, s.style.bg(bg)))
                    .collect();
                append_wrapped_line(&mut lines, gutter, gutter_style, spans, content_width);
            }
        } else {
            let gutter = format!("{:>4}   ", lineno);
            let gutter_style = Style::default().fg(Color::DarkGray);

            let spans = if is_search_hit {
                hl_line
                    .spans
                    .into_iter()
                    .map(|s| Span::styled(s.content, s.style.bg(SEARCH_HIGHLIGHT_BG)))
                    .collect()
            } else {
                hl_line.spans
            };

            append_wrapped_line(&mut lines, gutter, gutter_style, spans, content_width);
        }

        if let Some(deleted) = deleted_map.get(&lineno) {
            for d in deleted {
                let word_diff_old = resolve_deleted_line_word_diff(
                    &d.content,
                    lineno,
                    &word_diff_pairs,
                    &addition_contents,
                );
                if let Some(old_spans) = word_diff_old {
                    append_wrapped_line(
                        &mut lines,
                        format!("{:>4} {} ", "", "-"),
                        Style::default().fg(Color::Red),
                        word_diff_deleted_spans(&old_spans),
                        content_width,
                    );
                } else {
                    append_wrapped_line(
                        &mut lines,
                        format!("{:>4} {} ", "", "-"),
                        Style::default().fg(Color::Red),
                        vec![Span::styled(
                            d.content.clone(),
                            Style::default().fg(Color::White).bg(DELETION_BG),
                        )],
                        content_width,
                    );
                }
            }
        }
    }

    (lines, lineno_map)
}

fn append_wrapped_line(
    lines: &mut Vec<Line<'static>>,
    gutter: String,
    gutter_style: Style,
    content_spans: Vec<Span<'static>>,
    content_width: usize,
) {
    for (row_index, row_spans) in wrap_spans(content_spans, content_width)
        .into_iter()
        .enumerate()
    {
        let row_gutter = if row_index == 0 {
            gutter.clone()
        } else {
            " ".repeat(GUTTER_WIDTH)
        };
        let mut spans = vec![Span::styled(row_gutter, gutter_style)];
        spans.extend(row_spans);
        lines.push(Line::from(spans));
    }
}

fn wrap_spans(spans: Vec<Span<'static>>, max_width: usize) -> Vec<Vec<Span<'static>>> {
    if max_width == 0 {
        return vec![vec![]];
    }

    let mut rows: Vec<Vec<Span<'static>>> = vec![vec![]];
    let mut current_width = 0;

    for span in spans {
        let style = span.style;
        let mut remaining = span.content.to_string();

        while !remaining.is_empty() {
            let available = max_width - current_width;
            if available == 0 {
                rows.push(vec![]);
                current_width = 0;
                continue;
            }

            let mut take_width = 0;
            let mut take_bytes = 0;
            for ch in remaining.chars() {
                let ch_width = ch.width().unwrap_or(0);
                if take_width + ch_width > available {
                    break;
                }
                take_width += ch_width;
                take_bytes += ch.len_utf8();
            }

            if take_bytes == 0 {
                if current_width > 0 {
                    rows.push(vec![]);
                    current_width = 0;
                    continue;
                }

                let ch = remaining.chars().next().unwrap();
                take_width = ch.width().unwrap_or(0);
                take_bytes = ch.len_utf8();
            }

            let (taken, rest) = remaining.split_at(take_bytes);
            rows.last_mut()
                .unwrap()
                .push(Span::styled(taken.to_string(), style));
            current_width = (current_width + take_width).min(max_width);
            remaining = rest.to_string();
        }
    }

    rows
}

fn resolve_deleted_line_word_diff(
    del_content: &str,
    lineno: usize,
    word_diff_pairs: &std::collections::HashMap<usize, String>,
    addition_contents: &std::collections::HashMap<usize, String>,
) -> Option<Vec<word_diff::WordSpan>> {
    let next_add_lineno = lineno + 1;
    let paired_old = word_diff_pairs.get(&next_add_lineno)?;
    if paired_old != del_content {
        return None;
    }
    let add_text = addition_contents.get(&next_add_lineno)?;
    let (old_spans, _) = word_diff::compute_word_diff_if_useful(del_content, add_text)?;
    Some(old_spans)
}

fn word_diff_addition_spans(new_spans: &[word_diff::WordSpan]) -> Vec<Span<'static>> {
    new_spans
        .iter()
        .map(|ws| {
            let style = if ws.changed {
                Style::default().fg(Color::White).bg(ADDITION_HIGHLIGHT_BG)
            } else {
                Style::default().fg(Color::White).bg(ADDITION_BG)
            };
            Span::styled(ws.text.clone(), style)
        })
        .collect()
}

fn word_diff_deleted_spans(old_spans: &[word_diff::WordSpan]) -> Vec<Span<'static>> {
    old_spans
        .iter()
        .map(|ws| {
            let style = if ws.changed {
                Style::default().fg(Color::White).bg(DELETION_HIGHLIGHT_BG)
            } else {
                Style::default().fg(Color::White).bg(DELETION_BG)
            };
            Span::styled(ws.text.clone(), style)
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn long_lines_are_wrapped() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        writeln!(tmp, "{}", "A".repeat(60)).unwrap();
        writeln!(tmp, "short").unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        std::mem::forget(tmp);

        let highlighter = crate::highlight::Highlighter::new(&path).unwrap();
        let mut viewer = crate::viewer::Viewer::new(path, highlighter, None).unwrap();

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                crate::ui::draw(frame, area, &mut viewer, None);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let row0: String = (0..40)
            .map(|x| {
                buf.cell(ratatui::layout::Position::new(x, 0))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect();
        assert!(
            row0.trim_start().starts_with("1"),
            "row 0 should have line number 1: {:?}",
            row0
        );

        let row1: String = (0..40)
            .map(|x| {
                buf.cell(ratatui::layout::Position::new(x, 1))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect();
        assert!(
            row1.contains("A"),
            "row 1 should contain continuation of A's: {:?}",
            row1
        );
        let gutter1 = &row1[..7];
        assert!(
            gutter1.trim().is_empty(),
            "continuation row gutter should be blank: {:?}",
            gutter1
        );

        let row2: String = (0..40)
            .map(|x| {
                buf.cell(ratatui::layout::Position::new(x, 2))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect();
        assert!(
            row2.contains("short"),
            "row 2 should contain 'short': {:?}",
            row2
        );
    }

    #[test]
    fn unpaired_deletion_is_not_word_diffed() {
        // Scenario from actual bug: "let file = create_test_safetensors();" appears
        // as a deletion at two different locations. Only one has a matching addition
        // (at L342). The other (at L178) should NOT get word diff.
        let mut word_diff_pairs = HashMap::new();
        // L342's pair: del="let file = create_test_safetensors();" add="let file = create_test_safetensors("
        word_diff_pairs.insert(
            342,
            "        let file = create_test_safetensors();".to_string(),
        );

        let mut addition_contents = HashMap::new();
        addition_contents.insert(
            342,
            "        let file = create_test_safetensors(".to_string(),
        );

        // Deletion at L178: same text but no pair at L179
        let result = resolve_deleted_line_word_diff(
            "        let file = create_test_safetensors();",
            178,
            &word_diff_pairs,
            &addition_contents,
        );

        assert!(
            result.is_none(),
            "deletion at L178 should NOT be word-diffed — it has no pair at L179, \
             but the old code's content-based fallback would match it to L342's pair"
        );
    }

    #[test]
    fn paired_deletion_is_word_diffed() {
        let mut word_diff_pairs = HashMap::new();
        word_diff_pairs.insert(26, "    pub dtype: String,".to_string());

        let mut addition_contents = HashMap::new();
        addition_contents.insert(26, "    pub dtype: Dtype,".to_string());

        // Deletion at L25: paired with addition at L26
        let result = resolve_deleted_line_word_diff(
            "    pub dtype: String,",
            25,
            &word_diff_pairs,
            &addition_contents,
        );

        assert!(
            result.is_some(),
            "deletion at L25 should be word-diffed — it has a matching pair at L26"
        );

        let spans = result.unwrap();
        let changed_text: String = spans
            .iter()
            .filter(|s| s.changed)
            .map(|s| s.text.as_str())
            .collect();
        assert!(
            changed_text.contains("String"),
            "changed portion should contain 'String', got {:?}",
            changed_text
        );
    }
}
