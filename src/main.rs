use clap::Parser;
use libspy::{cli::Cli, entry::Entry};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    // Where we store our data
    match Entry::new(&cli.url, None, None) {
        Ok(entry) => {
            println!("{}", serde_json::to_string(&entry)?);
        }
        Err(e) => {
            eprintln!("Failed to add entry: {e}");
        }
    }

    Ok(())
}
