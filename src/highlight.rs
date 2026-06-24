use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::{SyntaxSet, SyntaxSetBuilder};
use syntect::util::LinesWithEndings;

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    extension: String,
}

impl Highlighter {
    pub fn new(path: &str) -> Result<Self> {
        let syntax_set = Self::load_syntax_set();
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

    fn load_syntax_set() -> SyntaxSet {
        let bundled_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/syntax_set.bin"));

        if let Ok(syntax_set) = syntect::dumps::from_uncompressed_data::<SyntaxSet>(bundled_bytes) {
            let user_dir = argus_config_dir().join("syntaxes");
            if user_dir.exists() {
                let mut builder: SyntaxSetBuilder = syntax_set.into_builder();
                let _ = builder.add_from_folder(&user_dir, true);
                return builder.build();
            }

            return syntax_set;
        }

        SyntaxSet::load_defaults_newlines()
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

fn argus_config_dir() -> std::path::PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        return std::path::PathBuf::from(home).join(".config/argus");
    }

    std::path::PathBuf::from(".config/argus")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_highlighter(ext: &str) -> Highlighter {
        Highlighter {
            syntax_set: Highlighter::load_syntax_set(),
            theme_set: ThemeSet::load_defaults(),
            extension: ext.to_string(),
        }
    }

    #[test]
    fn japanese_text_has_no_extra_spaces() {
        let highlighter = create_test_highlighter("go");

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
        let highlighter = create_test_highlighter("go");

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

    #[test]
    fn jsonnet_syntax_is_available() {
        let highlighter = Highlighter::new("test.jsonnet").unwrap();
        let content = "local x = 1;\n{ key: x }\n";
        let lines = highlighter.highlight(content);
        let full_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();

        assert!(
            lines[0].spans.len() > 1,
            "jsonnet should have syntax highlighting (multiple spans), got {} span(s): {:?}",
            lines[0].spans.len(),
            full_text
        );
    }

    #[test]
    fn terraform_syntax_is_available() {
        let highlighter = Highlighter::new("main.tf").unwrap();
        let content = "resource \"aws_instance\" \"example\" {\n  ami = \"abc\"\n}\n";
        let lines = highlighter.highlight(content);

        assert!(
            lines[0].spans.len() > 1,
            "terraform should have syntax highlighting"
        );
    }

    #[test]
    fn carina_crn_syntax_is_available() {
        let highlighter = Highlighter::new("test.crn").unwrap();
        let content = "let vpc = use { source = 'aws_vpc' }\n";
        let lines = highlighter.highlight(content);
        assert!(
            lines[0].spans.len() > 1,
            "carina .crn should have syntax highlighting (multiple spans), got {} span(s)",
            lines[0].spans.len()
        );
    }
}
