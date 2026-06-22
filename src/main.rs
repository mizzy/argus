use anyhow::Result;
use clap::Parser;

mod app;
mod diff;
mod highlight;
mod ui;
mod viewer;
mod word_diff;

#[derive(Parser)]
#[command(
    name = "argus",
    version,
    about = "A code reading tool with syntax highlighting and git diff navigation"
)]
struct Cli {
    file: String,

    #[arg(
        long,
        help = "Git revision range for diff (e.g. HEAD~1, HEAD~3..HEAD, abc123..def456)"
    )]
    rev: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut terminal = ratatui::init();
    let result = app::App::new(cli.file, cli.rev)?.run(&mut terminal);
    ratatui::restore();

    result
}
