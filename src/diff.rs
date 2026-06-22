use anyhow::{Context, Result};
use git2::{DiffOptions, Repository};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Addition,
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub new_start: u32,
}

#[derive(Debug, Clone)]
pub struct DeletedLine {
    pub content: String,
}

pub struct DiffState {
    pub hunks: Vec<DiffHunk>,
    pub addition_lines: HashMap<usize, DiffLineKind>,
    pub deleted_lines: HashMap<usize, Vec<DeletedLine>>,
    pub is_new_file: bool,
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

        let is_new_file = diff.deltas().any(|d| {
            matches!(d.status(), git2::Delta::Added | git2::Delta::Untracked)
        });

        if is_new_file {
            return Ok(Self {
                hunks: Vec::new(),
                addition_lines: HashMap::new(),
                deleted_lines: HashMap::new(),
                is_new_file: true,
            });
        }

        let mut hunks: Vec<DiffHunk> = Vec::new();
        let mut addition_lines: HashMap<usize, DiffLineKind> = HashMap::new();
        let mut deleted_lines: HashMap<usize, Vec<DeletedLine>> = HashMap::new();
        let mut last_new_lineno: Option<u32> = None;

        diff.print(git2::DiffFormat::Patch, |_delta, hunk, line| {
            if let Some(hunk) = hunk {
                let new_start = hunk.new_start();
                if hunks.last().is_none_or(|h| h.new_start != new_start) {
                    hunks.push(DiffHunk { new_start });
                }
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
                        let hunk_start = hunks.last().map(|h| h.new_start).unwrap_or(1);
                        (hunk_start as usize).saturating_sub(1)
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

        hunks.retain(|h| {
            let start = h.new_start as usize;
            addition_lines.keys().any(|&k| k >= start)
                || deleted_lines.keys().any(|&k| k >= start)
        });

        if hunks.is_empty() && addition_lines.is_empty() && deleted_lines.is_empty() {
            anyhow::bail!("no diff");
        }

        Ok(Self {
            hunks,
            addition_lines,
            deleted_lines,
            is_new_file: false,
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

    pub fn hunk_start_lines(&self) -> Vec<u32> {
        self.hunks.iter().map(|h| h.new_start).collect()
    }
}
