use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone)]
pub struct WordSpan {
    pub text: String,
    pub changed: bool,
}

pub fn similarity(old: &str, new: &str) -> f64 {
    let diff = TextDiff::configure().diff_words(old, new);
    diff.ratio() as f64
}

fn tokenize(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i].is_alphanumeric() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(chars[start..i].iter().collect());
        } else {
            tokens.push(chars[i].to_string());
            i += 1;
        }
    }
    tokens
}

pub fn compute_word_diff(old: &str, new: &str) -> (Vec<WordSpan>, Vec<WordSpan>) {
    let old_tokens = tokenize(old);
    let new_tokens = tokenize(new);
    let old_refs: Vec<&str> = old_tokens.iter().map(|s| s.as_str()).collect();
    let new_refs: Vec<&str> = new_tokens.iter().map(|s| s.as_str()).collect();

    let diff = TextDiff::from_slices(&old_refs, &new_refs);

    let mut old_spans = Vec::new();
    let mut new_spans = Vec::new();

    for change in diff.iter_all_changes() {
        let text = change.value().to_string();
        match change.tag() {
            ChangeTag::Equal => {
                old_spans.push(WordSpan {
                    text: text.clone(),
                    changed: false,
                });
                new_spans.push(WordSpan {
                    text,
                    changed: false,
                });
            }
            ChangeTag::Delete => {
                old_spans.push(WordSpan {
                    text,
                    changed: true,
                });
            }
            ChangeTag::Insert => {
                new_spans.push(WordSpan {
                    text,
                    changed: true,
                });
            }
        }
    }

    (old_spans, new_spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_deletion_highlights_only_removed_part() {
        let old = "use std::io::{self, Read};";
        let new = "use std::io;";
        let (old_spans, new_spans) = compute_word_diff(old, new);

        let old_unchanged: String = old_spans
            .iter()
            .filter(|s| !s.changed)
            .map(|s| s.text.as_str())
            .collect();
        let old_changed: String = old_spans
            .iter()
            .filter(|s| s.changed)
            .map(|s| s.text.as_str())
            .collect();

        assert!(
            old_unchanged.contains("use std::io"),
            "common prefix 'use std::io' should be unchanged, got unchanged={:?}",
            old_unchanged
        );
        assert!(
            old_changed.contains("{self, Read}"),
            "only '{{self, Read}}' should be changed, got changed={:?}",
            old_changed
        );

        let new_unchanged: String = new_spans
            .iter()
            .filter(|s| !s.changed)
            .map(|s| s.text.as_str())
            .collect();
        assert!(
            new_unchanged.contains("use std::io"),
            "new side should have 'use std::io' as unchanged, got unchanged={:?}",
            new_unchanged
        );
    }

    #[test]
    fn type_change_highlights_only_type_name() {
        let old = "    pub dtype: String,";
        let new = "    pub dtype: Dtype,";
        let (old_spans, new_spans) = compute_word_diff(old, new);

        let old_changed: String = old_spans
            .iter()
            .filter(|s| s.changed)
            .map(|s| s.text.as_str())
            .collect();
        let new_changed: String = new_spans
            .iter()
            .filter(|s| s.changed)
            .map(|s| s.text.as_str())
            .collect();

        assert!(
            old_changed.contains("String"),
            "old changed should contain 'String', got {:?}",
            old_changed
        );
        assert!(
            new_changed.contains("Dtype"),
            "new changed should contain 'Dtype', got {:?}",
            new_changed
        );
        assert!(
            !old_changed.contains("pub dtype"),
            "common part 'pub dtype' should not be changed, got {:?}",
            old_changed
        );
    }

    #[test]
    fn identical_lines_have_no_changes() {
        let line = "use std::collections::HashMap;";
        let (old_spans, new_spans) = compute_word_diff(line, line);

        assert!(old_spans.iter().all(|s| !s.changed));
        assert!(new_spans.iter().all(|s| !s.changed));
    }
}
