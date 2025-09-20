use clap::Parser;
use libspy::{cli::Cli, entry::Entry, template::Template};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    // Where we store our data
    let entry = Entry::new(&cli.url, None)?;
    // println!("{}", serde_json::to_string(&entry)?);
    let rendered = cli
        .template
        .map(Template::new)
        .map(|t| t.render(&entry).map_err(color_eyre::Report::from))
        .unwrap_or_else(|| serde_json::to_string(&entry).map_err(color_eyre::Report::from))?;
    println!("{}", rendered);
    Ok(())
}
