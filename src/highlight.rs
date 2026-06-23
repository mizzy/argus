use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    extension: String,
}

impl Highlighter {
    pub fn new(path: &str) -> Result<Self> {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        Ok(Self {
            syntax_set,
            theme_set,
            extension,
        })
    }

    pub fn highlight(&self, content: &str) -> Vec<Line<'static>> {
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(&self.extension)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        LinesWithEndings::from(content)
            .map(|line| {
                let ranges = h.highlight_line(line, &self.syntax_set).unwrap_or_default();
                let spans: Vec<Span<'static>> = ranges
                    .into_iter()
                    .map(|(style, text)| {
                        let fg =
                            Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                        Span::styled(text.replace('\t', "    "), Style::default().fg(fg))
                    })
                    .collect();
                Line::from(spans)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn japanese_text_has_no_extra_spaces() {
        let highlighter = Highlighter {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            extension: "go".to_string(),
        };

        let content = "fmt.Println(\"空席照会失敗\")\n";
        let lines = highlighter.highlight(content);

        // Concatenate all span text in the first line
        let full_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();

        // Print each span for debugging
        for (i, span) in lines[0].spans.iter().enumerate() {
            eprintln!("span {}: {:?}", i, span.content);
        }

        // The text should contain "空席照会失敗" without spaces between characters
        assert!(
            full_text.contains("空席照会失敗"),
            "Japanese text should not have extra spaces: got {:?}",
            full_text
        );
    }

    #[test]
    fn tabs_are_expanded_to_spaces() {
        let highlighter = Highlighter {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            extension: "go".to_string(),
        };

        let content = "\tfmt.Println(\"hello\")\n";
        let lines = highlighter.highlight(content);

        let full_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();

        // Tab should be expanded to spaces, not remain as \t
        assert!(
            !full_text.contains('\t'),
            "tabs should be expanded to spaces: got {:?}",
            full_text
        );
        // Should start with spaces (expanded tab)
        assert!(
            full_text.starts_with("    "),
            "expanded tab should produce leading spaces: got {:?}",
            full_text
        );
    }
}
