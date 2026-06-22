use anyhow::{Context, Result};
use git2::{DiffOptions, Repository};
use std::collections::HashMap;

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
        let mut deleted_lines: HashMap<usize, Vec<DeletedLine>> = HashMap::new();
        let mut last_new_lineno: Option<u32> = None;
        let mut current_hunk_start: u32 = 1;

        diff.print(git2::DiffFormat::Patch, |_delta, hunk, line| {
            if let Some(hunk) = hunk {
                current_hunk_start = hunk.new_start();
            }

            match line.origin() {
                '+' => {
                    if let Some(lineno) = line.new_lineno() {
                        addition_lines.insert(lineno as usize, DiffLineKind::Addition);
                        last_new_lineno = Some(lineno);
                    }
                }
                '-' => {
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
                        .push(DeletedLine { content });
                }
                ' ' => {
                    if let Some(lineno) = line.new_lineno() {
                        last_new_lineno = Some(lineno);
                    }
                }
                _ => {}
            }

            true
        })?;

        if addition_lines.is_empty() && deleted_lines.is_empty() {
            anyhow::bail!("no diff");
        }

        let change_groups = Self::compute_change_groups(&addition_lines, &deleted_lines);

        Ok(Self {
            addition_lines,
            deleted_lines,
            change_groups,
        })
    }

    fn diff_workdir<'a>(
        repo: &'a Repository,
        opts: &mut DiffOptions,
    ) -> Result<git2::Diff<'a>> {
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
