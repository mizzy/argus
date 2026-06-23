use anyhow::Result;
use clap::Parser;
use std::io::{IsTerminal, Read};

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
    file: Option<String>,

    #[arg(
        long,
        help = "Git revision range for diff (e.g. HEAD~1, HEAD~3..HEAD, abc123..def456)"
    )]
    rev: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut app = if let Some(file) = cli.file {
        app::App::new(file, cli.rev)?
    } else {
        if std::io::stdin().is_terminal() {
            anyhow::bail!("file argument is required unless stdin is piped");
        }

        let mut content = String::new();
        std::io::stdin().read_to_string(&mut content)?;
        app::App::from_content(content, cli.rev)?
    };

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();

    result
}
