use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone)]
pub struct WordSpan {
    pub text: String,
    pub changed: bool,
}

pub fn similarity(old: &str, new: &str) -> f64 {
    let old_tokens: Vec<String> = tokenize(old.trim())
        .into_iter()
        .filter(|t| !t.trim().is_empty())
        .collect();
    let new_tokens: Vec<String> = tokenize(new.trim())
        .into_iter()
        .filter(|t| !t.trim().is_empty())
        .collect();
    let old_refs: Vec<&str> = old_tokens.iter().map(|s| s.as_str()).collect();
    let new_refs: Vec<&str> = new_tokens.iter().map(|s| s.as_str()).collect();
    let diff = TextDiff::from_slices(&old_refs, &new_refs);
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

const MAX_CHANGED_RATIO: f64 = 0.7;

pub fn compute_word_diff_if_useful(
    old: &str,
    new: &str,
) -> Option<(Vec<WordSpan>, Vec<WordSpan>)> {
    let (old_spans, new_spans) = compute_word_diff(old, new);

    let non_ws = |s: &&WordSpan| !s.text.trim().is_empty();
    let old_total: usize = old_spans.iter().filter(non_ws).map(|s| s.text.len()).sum();
    let old_changed: usize = old_spans.iter().filter(|s| s.changed).filter(non_ws).map(|s| s.text.len()).sum();
    let new_total: usize = new_spans.iter().filter(non_ws).map(|s| s.text.len()).sum();
    let new_changed: usize = new_spans.iter().filter(|s| s.changed).filter(non_ws).map(|s| s.text.len()).sum();

    let total = old_total + new_total;
    if total == 0 {
        return None;
    }

    let changed_ratio = (old_changed + new_changed) as f64 / total as f64;
    if changed_ratio > MAX_CHANGED_RATIO {
        return None;
    }

    Some((old_spans, new_spans))
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

    #[test]
    fn dissimilar_lines_should_not_be_word_diffed() {
        let cases = vec![
            (
                "    let mut tensors = HashMap::new();",
                "        let bytes = &self.mmap[start..end];",
            ),
            (
                "pub fn parse_header(path: &Path) -> io::Result<HashMap<String, TensorInfo>> {",
                "#[derive(Debug, Deserialize)]",
            ),
            (
                "    let mut file = File::open(path)?;",
                "struct RawTensorInfo {",
            ),
            (
                "            continue;",
                "                io::ErrorKind::InvalidData,",
            ),
        ];

        for (old, new) in &cases {
            assert!(
                similarity(old, new) < 0.6,
                "lines should have low similarity (<0.6): old={:?} new={:?} ratio={}",
                old, new, similarity(old, new)
            );
        }
    }

    #[test]
    fn mostly_changed_word_diff_returns_none() {
        let old = "            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;";
        let new = "            ));";
        let result = compute_word_diff_if_useful(old, new);
        assert!(
            result.is_none(),
            "word diff should be None when most of the line changed, got {:?}",
            result
        );
    }

    #[test]
    fn useful_word_diff_returns_some() {
        let old = "    pub dtype: String,";
        let new = "    pub dtype: Dtype,";
        let result = compute_word_diff_if_useful(old, new);
        assert!(
            result.is_some(),
            "word diff should be Some for useful changes"
        );
    }

    #[test]
    fn similar_lines_should_be_word_diffed() {
        let cases = vec![
            (
                "use std::io::{self, Read};",
                "use std::io;",
            ),
            (
                "    pub dtype: String,",
                "    pub dtype: Dtype,",
            ),
        ];

        for (old, new) in &cases {
            assert!(
                similarity(old, new) >= 0.6,
                "lines should have high similarity (>=0.6): old={:?} new={:?} ratio={}",
                old, new, similarity(old, new)
            );
        }
    }
}
