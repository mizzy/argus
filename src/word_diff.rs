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

pub fn compute_word_diff(old: &str, new: &str) -> (Vec<WordSpan>, Vec<WordSpan>) {
    let diff = TextDiff::configure().diff_words(old, new);

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
