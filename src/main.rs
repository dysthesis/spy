use std::{collections::HashSet, env, path::PathBuf};

use clap::Parser;
use libspy::{cli::Cli, entry::Entry, tag::Tag};

pub const DEFAULT_DATA_DIR: &str = "~/.local/share/spy";

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    // Where we store our data
    dbg!(&cli);
    match cli.subcommand {
        libspy::cli::Command::Add { url, tags, title } => {
            let tag_set: HashSet<Tag> = tags
                .iter()
                .filter_map(|t| Tag::try_from(t.as_str()).ok())
                .collect();
            match Entry::new(&url, title, Some(tag_set)) {
                Ok(entry) => {
                    println!("{}", serde_json::to_string(&dbg!(entry))?);
                }
                Err(e) => {
                    eprintln!("Failed to add entry: {e}");
                }
            }
        }
    }
    Ok(())
}
