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
    pub fn load(file_path: &str) -> Result<Self> {
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

        let head_tree = repo.head().ok().and_then(|r| r.peel_to_tree().ok());

        let diff = repo
            .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut opts))
            .context("failed to compute diff")?;

        let mut hunks: Vec<DiffHunk> = Vec::new();
        let mut line_marks: HashMap<usize, DiffLineKind> = HashMap::new();

        diff.print(git2::DiffFormat::Patch, |_delta, hunk, line| {
            if let Some(hunk) = hunk {
                let new_start = hunk.new_start();
                if hunks.last().map_or(true, |h| h.new_start != new_start) {
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
            line_marks.iter().any(|(&lineno, _)| {
                lineno >= h.new_start as usize
            })
        });

        if hunks.is_empty() && line_marks.is_empty() {
            anyhow::bail!("no diff");
        }

        Ok(Self { hunks, line_marks })
    }

    pub fn hunk_start_lines(&self) -> Vec<u32> {
        self.hunks.iter().map(|h| h.new_start).collect()
    }
}
