use anyhow::Result;
use clap::Parser;

mod app;
mod diff;
mod highlight;
mod ui;
mod viewer;

#[derive(Parser)]
#[command(name = "argus", version, about = "A code reading tool with syntax highlighting and git diff navigation")]
struct Cli {
    file: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut terminal = ratatui::init();
    let result = app::App::new(cli.file)?.run(&mut terminal);
    ratatui::restore();

    result
}
