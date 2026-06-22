use anyhow::{Context, Result};
use git2::{DiffOptions, Repository};
use std::collections::HashMap;

use crate::word_diff;

const WORD_DIFF_SIMILARITY_THRESHOLD: f64 = 0.6;

type DiffBlock = (Vec<String>, Vec<(usize, String)>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Addition,
}

#[derive(Debug, Clone)]
pub struct DeletedLine {
    pub content: String,
}

pub struct DiffState {
    pub addition_lines: HashMap<usize, DiffLineKind>,
    pub deleted_lines: HashMap<usize, Vec<DeletedLine>>,
    pub change_groups: Vec<usize>,
    pub addition_contents: HashMap<usize, String>,
    pub word_diff_pairs: HashMap<usize, String>,
}

impl DiffState {
    pub fn load(file_path: &str, rev: Option<&str>) -> Result<Self> {
        let path = std::env::current_dir()?.join(file_path);
        let path = path.canonicalize().unwrap_or(path);
        let repo = Repository::discover(&path).context("not a git repository")?;
        let workdir = repo.workdir().context("bare repository")?;
        let relative = path
            .strip_prefix(workdir.canonicalize()?)
            .context("file is outside the repository")?
            .to_path_buf();

        let mut opts = DiffOptions::new();
        opts.pathspec(&relative);
        opts.include_untracked(true);

        let diff = if let Some(rev_spec) = rev {
            Self::diff_from_rev(&repo, rev_spec, &mut opts)?
        } else {
            Self::diff_workdir(&repo, &mut opts)?
        };

        let mut addition_lines: HashMap<usize, DiffLineKind> = HashMap::new();
        let mut addition_contents: HashMap<usize, String> = HashMap::new();
        let mut deleted_lines: HashMap<usize, Vec<DeletedLine>> = HashMap::new();
        let mut last_new_lineno: Option<u32> = None;
        let mut current_hunk_start: u32 = 1;

        let mut pending_deletions: Vec<String> = Vec::new();
        let mut pending_additions: Vec<(usize, String)> = Vec::new();
        let mut blocks: Vec<DiffBlock> = Vec::new();

        diff.print(git2::DiffFormat::Patch, |_delta, hunk, line| {
            if let Some(hunk) = hunk {
                current_hunk_start = hunk.new_start();
            }

            match line.origin() {
                '+' => {
                    if let Some(lineno) = line.new_lineno() {
                        let content = String::from_utf8_lossy(line.content())
                            .trim_end_matches('\n')
                            .to_string();
                        addition_lines.insert(lineno as usize, DiffLineKind::Addition);
                        addition_contents.insert(lineno as usize, content.clone());
                        pending_additions.push((lineno as usize, content));
                        last_new_lineno = Some(lineno);
                    }
                }
                '-' => {
                    if !pending_additions.is_empty() {
                        blocks.push((
                            std::mem::take(&mut pending_deletions),
                            std::mem::take(&mut pending_additions),
                        ));
                    }
                    let content = String::from_utf8_lossy(line.content())
                        .trim_end_matches('\n')
                        .to_string();
                    let insert_after = if let Some(prev) = last_new_lineno {
                        prev as usize
                    } else {
                        (current_hunk_start as usize).saturating_sub(1)
                    };
                    deleted_lines
                        .entry(insert_after)
                        .or_default()
                        .push(DeletedLine {
                            content: content.clone(),
                        });
                    pending_deletions.push(content);
                }
                ' ' | 'F' | 'H' => {
                    if !pending_deletions.is_empty() || !pending_additions.is_empty() {
                        blocks.push((
                            std::mem::take(&mut pending_deletions),
                            std::mem::take(&mut pending_additions),
                        ));
                    }
                    if line.origin() == ' '
                        && let Some(lineno) = line.new_lineno()
                    {
                        last_new_lineno = Some(lineno);
                    }
                }
                _ => {}
            }

            true
        })?;

        if !pending_deletions.is_empty() || !pending_additions.is_empty() {
            blocks.push((
                std::mem::take(&mut pending_deletions),
                std::mem::take(&mut pending_additions),
            ));
        }

        let word_diff_pairs = Self::build_word_diff_pairs(&blocks);

        if addition_lines.is_empty() && deleted_lines.is_empty() {
            anyhow::bail!("no diff");
        }

        let change_groups = Self::compute_change_groups(&addition_lines, &deleted_lines);

        Ok(Self {
            addition_lines,
            addition_contents,
            deleted_lines,
            change_groups,
            word_diff_pairs,
        })
    }

    fn diff_workdir<'a>(repo: &'a Repository, opts: &mut DiffOptions) -> Result<git2::Diff<'a>> {
        let head_tree = repo.head().ok().and_then(|r| r.peel_to_tree().ok());
        repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(opts))
            .context("failed to compute diff")
    }

    fn diff_from_rev<'a>(
        repo: &'a Repository,
        rev_spec: &str,
        opts: &mut DiffOptions,
    ) -> Result<git2::Diff<'a>> {
        if let Some((from_str, to_str)) = rev_spec.split_once("..") {
            let from_tree = Self::resolve_tree(repo, from_str)?;
            if to_str.is_empty() {
                let head_tree = repo
                    .head()?
                    .peel_to_tree()
                    .context("HEAD is not a valid tree")?;
                repo.diff_tree_to_tree(Some(&from_tree), Some(&head_tree), Some(opts))
                    .context("failed to compute diff")
            } else {
                let to_tree = Self::resolve_tree(repo, to_str)?;
                repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), Some(opts))
                    .context("failed to compute diff")
            }
        } else {
            let from_tree = Self::resolve_tree(repo, rev_spec)?;
            let head_tree = repo
                .head()?
                .peel_to_tree()
                .context("HEAD is not a valid tree")?;
            repo.diff_tree_to_tree(Some(&from_tree), Some(&head_tree), Some(opts))
                .context("failed to compute diff")
        }
    }

    fn resolve_tree<'a>(repo: &'a Repository, spec: &str) -> Result<git2::Tree<'a>> {
        repo.revparse_single(spec)?
            .peel_to_tree()
            .context("revision is not a valid tree")
    }

    fn build_word_diff_pairs(blocks: &[DiffBlock]) -> HashMap<usize, String> {
        let mut pairs = HashMap::new();
        for (dels, adds) in blocks {
            let pair_count = dels.len().min(adds.len());
            for i in 0..pair_count {
                let (add_lineno, add_text) = &adds[i];
                let del_text = &dels[i];
                if word_diff::similarity(del_text, add_text) < WORD_DIFF_SIMILARITY_THRESHOLD {
                    continue;
                }
                pairs.insert(*add_lineno, del_text.clone());
            }
        }
        pairs
    }

    fn compute_change_groups(
        addition_lines: &HashMap<usize, DiffLineKind>,
        deleted_lines: &HashMap<usize, Vec<DeletedLine>>,
    ) -> Vec<usize> {
        let mut changed: Vec<usize> = addition_lines.keys().copied().collect();
        for &lineno in deleted_lines.keys() {
            changed.push(lineno);
        }
        changed.sort();
        changed.dedup();

        let mut groups = Vec::new();
        let mut prev: Option<usize> = None;
        for lineno in changed {
            match prev {
                None => groups.push(lineno),
                Some(p) if lineno > p + 1 => groups.push(lineno),
                _ => {}
            }
            prev = Some(lineno);
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_deletions_one_addition_does_not_pair_unrelated_lines() {
        let blocks = vec![(
            vec![
                "        // Write dummy tensor data (64 bytes = 16 f32 values)".to_string(),
                "        file.write_all(&[0u8; 64]).unwrap();".to_string(),
            ],
            vec![(173, "        file.write_all(data).unwrap();".to_string())],
        )];

        let pairs = DiffState::build_word_diff_pairs(&blocks);

        assert!(
            !pairs.contains_key(&173),
            "comment line should not be paired with code line, but got pair: {:?}",
            pairs.get(&173)
        );

        assert!(
            pairs.is_empty(),
            "no pairs should be created since dels[0] (comment) is dissimilar to adds[0] (code), pairs={:?}",
            pairs
        );
    }

    #[test]
    fn duplicate_deletion_text_across_blocks_does_not_create_false_pairs() {
        // Block 1: fn name change (paired correctly)
        // Block 2: same "let file = create_test_safetensors();" appears but should
        // only pair within its own block, not with block 1's identical deletion
        let blocks = vec![
            (
                vec![
                    "    fn parse_header_returns_tensor_info() {".to_string(),
                    "        let file = create_test_safetensors();".to_string(),
                ],
                vec![
                    (
                        179,
                        "    fn dtype_element_size_returns_byte_widths() {".to_string(),
                    ),
                    (
                        180,
                        "        assert_eq!(Dtype::F32.element_size(), 4);".to_string(),
                    ),
                ],
            ),
            (
                vec![
                    "    fn parse_header_skips_metadata() {".to_string(),
                    "        let file = create_test_safetensors();".to_string(),
                ],
                vec![
                    (341, "    fn tensors_returns_all_metadata() {".to_string()),
                    (
                        342,
                        "        let file = create_test_safetensors(".to_string(),
                    ),
                ],
            ),
        ];

        let pairs = DiffState::build_word_diff_pairs(&blocks);

        // Block 2 pair: "let file = create_test_safetensors();" -> "let file = create_test_safetensors("
        // This is the only pair that should exist for L342
        if let Some(del_text) = pairs.get(&342) {
            assert_eq!(
                del_text, "        let file = create_test_safetensors();",
                "L342 should pair with its own block's deletion"
            );
        }

        // Block 1's "let file = create_test_safetensors();" should NOT pair with
        // block 2's addition at L342 — they are in different blocks
        // The word_diff_pairs map only creates pairs within each block, so this is
        // already correct in build_word_diff_pairs. The real bug is in ui.rs's
        // paired_del_contents fallback which does cross-block matching by content.
    }

    #[test]
    fn matching_lines_are_paired() {
        let blocks = vec![(
            vec!["    pub dtype: String,".to_string()],
            vec![(26, "    pub dtype: Dtype,".to_string())],
        )];

        let pairs = DiffState::build_word_diff_pairs(&blocks);

        assert_eq!(
            pairs.get(&26),
            Some(&"    pub dtype: String,".to_string()),
            "similar lines should be paired"
        );
    }
}
