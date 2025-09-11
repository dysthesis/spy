use std::{collections::HashSet, env, path::PathBuf};

use clap::Parser;
use libbm::{cli::Cli, entry::Entry, tag::Tag};

pub const DEFAULT_DATA_DIR: &str = "~/.local/share/bm";

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    // Where we store our data
    let _data_dir = cli
        .data_dir
        .clone()
        .or(env::var("BM_DATA_DIR").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DATA_DIR));
    dbg!(&cli);
    match cli.subcommand {
        libbm::cli::Command::Add { url, tags, title } => {
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
