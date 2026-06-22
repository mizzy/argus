use anyhow::{Context, Result};
use git2::{DiffOptions, Repository};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Addition,
    Deletion,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub new_lineno: Option<u32>,
    pub content: String,
}

pub struct DiffState {
    pub hunks: Vec<DiffHunk>,
    pub lines: Vec<DiffLine>,
}

impl DiffState {
    pub fn load(file_path: &str) -> Result<Self> {
        let path = Path::new(file_path);
        let repo = Repository::discover(path).context("not a git repository")?;
        let workdir = repo.workdir().context("bare repository")?;
        let relative = path
            .canonicalize()?
            .strip_prefix(workdir.canonicalize()?)
            .context("file is outside the repository")?
            .to_path_buf();

        let mut opts = DiffOptions::new();
        opts.pathspec(relative);

        let diff = repo
            .diff_index_to_workdir(None, Some(&mut opts))
            .context("failed to compute diff")?;

        let mut hunks = Vec::new();
        let mut lines = Vec::new();

        diff.foreach(
            &mut |_, _| true,
            None,
            Some(&mut |_, hunk| {
                hunks.push(DiffHunk {
                    old_start: hunk.old_start(),
                    new_start: hunk.new_start(),
                    new_lines: hunk.new_lines(),
                    header: String::from_utf8_lossy(hunk.header()).trim().to_string(),
                });
                true
            }),
            Some(&mut |_, _hunk, line| {
                let kind = match line.origin() {
                    '+' => DiffLineKind::Addition,
                    '-' => DiffLineKind::Deletion,
                    _ => DiffLineKind::Context,
                };
                lines.push(DiffLine {
                    kind,
                    new_lineno: line.new_lineno(),
                    content: String::from_utf8_lossy(line.content()).to_string(),
                });
                true
            }),
        )?;

        Ok(Self { hunks, lines })
    }

    pub fn hunk_start_lines(&self) -> Vec<u32> {
        self.hunks.iter().map(|h| h.new_start).collect()
    }
}
