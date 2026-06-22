use anyhow::{Context, Result};
use git2::{DiffOptions, Repository};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Addition,
    Deletion,
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub new_start: u32,
}

pub struct DiffState {
    pub hunks: Vec<DiffHunk>,
    pub line_marks: HashMap<usize, DiffLineKind>,
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

        let diff = if let Some(rev_spec) = rev {
            Self::diff_from_rev(&repo, rev_spec, &mut opts)?
        } else {
            Self::diff_workdir(&repo, &mut opts)?
        };

        let mut hunks: Vec<DiffHunk> = Vec::new();
        let mut line_marks: HashMap<usize, DiffLineKind> = HashMap::new();

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
                        line_marks.insert(lineno as usize, DiffLineKind::Addition);
                    }
                }
                '-' => {
                    if let Some(lineno) = line.old_lineno() {
                        line_marks.insert(lineno as usize, DiffLineKind::Deletion);
                    }
                }
                _ => {}
            }

            true
        })?;

        hunks.retain(|h| {
            line_marks
                .iter()
                .any(|(&lineno, _)| lineno >= h.new_start as usize)
        });

        if hunks.is_empty() && line_marks.is_empty() {
            anyhow::bail!("no diff");
        }

        Ok(Self { hunks, line_marks })
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
            let commit = repo
                .revparse_single(rev_spec)?
                .peel_to_commit()
                .context("revision is not a commit")?;
            let tree = commit.tree()?;
            let parent_tree = commit
                .parent(0)
                .ok()
                .and_then(|p| p.tree().ok());
            repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(opts))
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
