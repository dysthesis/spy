use std::path::PathBuf;

use clap::{Parser, Subcommand};
use url::Url;
#[derive(Parser, Debug)]
#[command(version, name = "bm", about = "A command-line bookmark manager.")]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Command,

    /// Where to store bookmark data
    #[arg(short, long)]
    pub data_dir: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Add {
        /// The bookmark to the URL.
        url: Url,

        /// What tags to add to the bookmark
        #[arg(short = 't', long)]
        tags: Vec<String>,

        /// Optional explicit name for the entry. If not, provided, we will attempt to fetch the
        /// metadata from the page.
        #[arg(short = 'T', long)]
        title: Option<String>,
    },
}
